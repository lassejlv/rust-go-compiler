package typeck

import (
	"math/big"
	"strconv"
	"strings"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
	"rustgo/compiler/resolve"
)

// ConstValue caches an evaluated constant item.
type ConstValue struct {
	Val int64
	OK  bool
	V   *cval // the full value (nil when evaluation failed / in progress)
}

// ---------------------------------------------------------------------------
// Compile-time constant evaluation.
//
// This is a small interpreter for the subset of Rust allowed in const
// contexts: integer/bool/char/str arithmetic, arrays, tuples and structs,
// local variables, control flow (if/match/while/loop/for over ranges,
// break/continue/return) and calls to (const) fns, associated fns and
// associated consts (inherent, trait impls and trait defaults). It is used for
// array lengths, const generic arguments and enum discriminants.

type cvKind uint8

const (
	cvUnit cvKind = iota
	cvInt
	cvBool
	cvFloat
	cvStr
	cvArr // arrays and slices
	cvTuple
	cvStruct // structs, tuple structs and enum variants
)

type cval struct {
	k     cvKind
	n     *big.Int // cvInt (never mutated once built)
	b     bool     // cvBool
	f     float64  // cvFloat
	f32   bool     // cvFloat: an f32
	ity   IntTy
	typed bool // cvInt: ity is known (literal suffix, cast, declared type); cvFloat: width known
	s     string
	elems []*cval
	def   *Def     // cvStruct: struct or variant def
	names []string // cvStruct: field names parallel to elems
}

func (v *cval) copy() *cval {
	if v == nil {
		return nil
	}
	n := *v
	if v.elems != nil {
		n.elems = make([]*cval, len(v.elems))
		for i, e := range v.elems {
			n.elems[i] = e.copy()
		}
	}
	return &n
}

func cvEqual(a, b *cval) bool {
	if a == nil || b == nil {
		return false
	}
	switch a.k {
	case cvInt, cvBool:
		x, ok1 := intOf(a)
		y, ok2 := intOf(b)
		return ok1 && ok2 && x.Cmp(y) == 0
	case cvFloat:
		return b.k == cvFloat && a.f == b.f
	case cvStr:
		return b.k == cvStr && a.s == b.s
	case cvUnit:
		return b.k == cvUnit
	}
	if a.k != b.k || a.def != b.def || len(a.elems) != len(b.elems) {
		return false
	}
	for i := range a.elems {
		if !cvEqual(a.elems[i], b.elems[i]) {
			return false
		}
	}
	return true
}

type ctlKind uint8

const (
	ctlBreak ctlKind = iota
	ctlContinue
	ctlReturn
)

// cvCtl is panicked to unwind for break/continue/return.
type cvCtl struct {
	kind  ctlKind
	label string
	v     *cval
}

type constEval struct {
	t     *Tcx
	steps int
	depth int
	// hint is the expected type of the expression being evaluated (consumed
	// by calls to infer omitted generic args, e.g. `const P: [u32; 4] =
	// primes();`).
	hint Ty
}

type cframe struct {
	ac     *astConv
	scopes []map[string]*cval
}

const (
	constMaxSteps = 5000000
	constMaxDepth = 256
)

// evalConstInt evaluates an integer constant expression (array lengths,
// enum discriminants, const generic arguments).
func (t *Tcx) evalConstInt(ac *astConv, e ast.Expr) (int64, bool) {
	v, ok := t.evalConstValue(ac, e)
	if !ok || v == nil {
		return 0, false
	}
	return cvToInt64(v)
}

// cvToInt64 converts an integer/bool value to int64 (u64 values as their
// bit pattern).
func cvToInt64(v *cval) (int64, bool) {
	n, ok := intOf(v)
	switch {
	case !ok:
		return 0, false
	case n.IsInt64():
		return n.Int64(), true
	case n.IsUint64():
		return int64(n.Uint64()), true
	}
	return 0, false
}

// evalConstValue evaluates a constant expression to a value.
func (t *Tcx) evalConstValue(ac *astConv, e ast.Expr) (v *cval, ok bool) {
	ev := &constEval{t: t}
	return ev.evalTop(ac, e)
}

func (ev *constEval) evalTop(ac *astConv, e ast.Expr) (v *cval, ok bool) {
	defer func() {
		if r := recover(); r != nil {
			if _, isCtl := r.(cvCtl); isCtl {
				v, ok = nil, false
				return
			}
			panic(r)
		}
	}()
	fr := &cframe{ac: ac}
	return ev.eval(fr, e)
}

func (fr *cframe) lookup(name string) (*cval, bool) {
	for i := len(fr.scopes) - 1; i >= 0; i-- {
		if v, ok := fr.scopes[i][name]; ok {
			return v, true
		}
	}
	return nil, false
}

func (fr *cframe) declare(name string, v *cval) {
	if len(fr.scopes) == 0 {
		fr.scopes = append(fr.scopes, map[string]*cval{})
	}
	fr.scopes[len(fr.scopes)-1][name] = v
}

func (fr *cframe) setVar(name string, v *cval) bool {
	for i := len(fr.scopes) - 1; i >= 0; i-- {
		if old, ok := fr.scopes[i][name]; ok {
			if old != nil {
				*old = *v
			} else {
				fr.scopes[i][name] = v
			}
			return true
		}
	}
	return false
}

// valueOf evaluates e for binding by value (copying unless it is a
// reference, which aliases its referent).
func (ev *constEval) valueOf(fr *cframe, e ast.Expr) (*cval, bool) {
	v, ok := ev.eval(fr, e)
	if !ok {
		return nil, false
	}
	if _, isRef := e.(*ast.RefExpr); isRef {
		return v, true
	}
	return v.copy(), true
}

func (ev *constEval) cond(fr *cframe, e ast.Expr) (bool, bool) {
	if le, ok := e.(*ast.LetExpr); ok {
		v, ok := ev.eval(fr, le.Expr)
		if !ok {
			return false, false
		}
		return ev.matchPat(fr, le.Pat, v)
	}
	if be, ok := e.(*ast.BinaryExpr); ok && be.Op == ast.BinAnd {
		// let chains bind into the current scope
		a, ok := ev.cond(fr, be.L)
		if !ok || !a {
			return false, ok
		}
		return ev.cond(fr, be.R)
	}
	v, ok := ev.eval(fr, e)
	if !ok {
		return false, false
	}
	return truthy(v)
}

// evalExpect evaluates e with an expected type.
func (ev *constEval) evalExpect(fr *cframe, e ast.Expr, ty Ty) (*cval, bool) {
	ev.hint = ty
	return ev.eval(fr, e)
}

func (ev *constEval) eval(fr *cframe, e ast.Expr) (*cval, bool) {
	hint := ev.hint
	ev.hint = nil
	ev.steps++
	if ev.steps > constMaxSteps {
		return nil, false
	}
	switch x := e.(type) {
	case *ast.LitExpr:
		switch x.Kind {
		case ast.LitInt:
			n := new(big.Int).SetUint64(x.IntVal)
			if x.Big != "" {
				if _, ok := n.SetString(x.Big, 10); !ok {
					return nil, false
				}
			}
			switch x.Suffix {
			case "":
				return cvI(n, 0, false), true
			case "f32", "f64":
				f, _ := new(big.Float).SetInt(n).Float64()
				return cvF(f, x.Suffix == "f32", true), true
			}
			it, ok := IntTyByName(x.Suffix)
			if !ok {
				return nil, false
			}
			return cvI(n, it, true), true
		case ast.LitFloat:
			f, err := strconv.ParseFloat(strings.ReplaceAll(x.Value, "_", ""), 64)
			if err != nil {
				return nil, false
			}
			return cvF(f, x.Suffix == "f32", x.Suffix != ""), true
		case ast.LitChar:
			return cvIi(int64(x.IntVal), 0, false), true
		case ast.LitByte:
			return cvIi(int64(x.IntVal), U8, true), true
		case ast.LitBool:
			return cvB(x.Value == "true"), true
		case ast.LitStr:
			return &cval{k: cvStr, s: x.Value}, true
		case ast.LitByteStr:
			arr := &cval{k: cvArr, elems: make([]*cval, len(x.Value))}
			for i := 0; i < len(x.Value); i++ {
				arr.elems[i] = cvIi(int64(x.Value[i]), U8, true)
			}
			return arr, true
		}
		return nil, false
	case *ast.BlockExpr:
		if x.Async {
			return nil, false
		}
		if x.Label != "" {
			return ev.labeledBlock(fr, x)
		}
		return ev.evalBlock(fr, x.Block)
	case *ast.PathExpr:
		return ev.evalPath(fr, x)
	case *ast.CastExpr:
		v, ok := ev.eval(fr, x.X)
		if !ok {
			return nil, false
		}
		ty := fr.ac.sub(fr.ac.lowerTy(x.Ty))
		switch ty.Kind {
		case TInt, TFloat, TChar, TBool:
			return castTo(v, ty)
		case TError:
			return nil, false
		}
		return v, true
	case *ast.UnaryExpr:
		v, ok := ev.eval(fr, x.X)
		if !ok {
			return nil, false
		}
		return unop(x.Op, v)
	case *ast.RefExpr:
		if x.Raw {
			return nil, false
		}
		return ev.eval(fr, x.X)
	case *ast.BinaryExpr:
		if x.Op == ast.BinAnd || x.Op == ast.BinOr {
			a, ok := ev.cond(fr, x.L)
			if !ok {
				return nil, false
			}
			if a == (x.Op == ast.BinOr) {
				return cvB(a), true
			}
			b, ok := ev.cond(fr, x.R)
			if !ok {
				return nil, false
			}
			return cvB(b), true
		}
		a, ok := ev.eval(fr, x.L)
		if !ok {
			return nil, false
		}
		b, ok := ev.eval(fr, x.R)
		if !ok {
			return nil, false
		}
		return binop(x.Op, a, b)
	case *ast.CallExpr:
		return ev.evalCall(fr, x, hint)
	case *ast.ParenExpr:
		return ev.evalExpect(fr, x.X, hint)
	case *ast.MethodCallExpr:
		return ev.evalMethod(fr, x)
	case *ast.IfExpr:
		n := len(fr.scopes)
		fr.scopes = append(fr.scopes, map[string]*cval{})
		c, ok := ev.cond(fr, x.Cond)
		if !ok {
			fr.scopes = fr.scopes[:n]
			return nil, false
		}
		if c {
			v, ok := ev.evalBlock(fr, x.Then)
			fr.scopes = fr.scopes[:n]
			return v, ok
		}
		fr.scopes = fr.scopes[:n]
		if x.Else == nil {
			return &cval{k: cvUnit}, true
		}
		return ev.eval(fr, x.Else)
	case *ast.MatchExpr:
		v, ok := ev.eval(fr, x.Scrutinee)
		if !ok {
			return nil, false
		}
		for _, arm := range x.Arms {
			n := len(fr.scopes)
			fr.scopes = append(fr.scopes, map[string]*cval{})
			m, ok := ev.matchPat(fr, arm.Pat, v)
			if ok && m && arm.Guard != nil {
				m, ok = ev.cond(fr, arm.Guard)
			}
			if !ok {
				fr.scopes = fr.scopes[:n]
				return nil, false
			}
			if m {
				r, ok := ev.eval(fr, arm.Body)
				fr.scopes = fr.scopes[:n]
				return r, ok
			}
			fr.scopes = fr.scopes[:n]
		}
		return nil, false
	case *ast.WhileExpr:
		for {
			n := len(fr.scopes)
			fr.scopes = append(fr.scopes, map[string]*cval{})
			c, ok := ev.cond(fr, x.Cond)
			if !ok {
				fr.scopes = fr.scopes[:n]
				return nil, false
			}
			if !c {
				fr.scopes = fr.scopes[:n]
				return &cval{k: cvUnit}, true
			}
			brk, _, ok := ev.loopBody(fr, x.Body, x.Label)
			fr.scopes = fr.scopes[:n]
			if !ok {
				return nil, false
			}
			if brk {
				return &cval{k: cvUnit}, true
			}
		}
	case *ast.LoopExpr:
		for {
			brk, v, ok := ev.loopBody(fr, x.Body, x.Label)
			if !ok {
				return nil, false
			}
			if brk {
				if v == nil {
					v = &cval{k: cvUnit}
				}
				return v, true
			}
		}
	case *ast.ForExpr:
		r, ok := x.Iter.(*ast.RangeExpr)
		if !ok || r.Start == nil || r.End == nil {
			return nil, false
		}
		lo, ok1 := ev.eval(fr, r.Start)
		hi, ok2 := ev.eval(fr, r.End)
		if !ok1 || !ok2 || lo.k != cvInt || hi.k != cvInt {
			return nil, false
		}
		it, typed := lo.ity, lo.typed
		if !typed {
			it, typed = hi.ity, hi.typed
		}
		start, ok1 := smallInt(lo)
		end, ok2 := smallInt(hi)
		if !ok1 || !ok2 {
			return nil, false
		}
		if r.Inclusive {
			end++
		}
		for i := start; i < end; i++ {
			n := len(fr.scopes)
			fr.scopes = append(fr.scopes, map[string]*cval{})
			m, ok := ev.matchPat(fr, x.Pat, cvIi(i, it, typed))
			if !ok || !m {
				fr.scopes = fr.scopes[:n]
				return nil, false
			}
			brk, _, ok := ev.loopBody(fr, x.Body, x.Label)
			fr.scopes = fr.scopes[:n]
			if !ok {
				return nil, false
			}
			if brk {
				break
			}
		}
		return &cval{k: cvUnit}, true
	case *ast.BreakExpr:
		var v *cval
		if x.Value != nil {
			var ok bool
			if v, ok = ev.eval(fr, x.Value); !ok {
				return nil, false
			}
		}
		panic(cvCtl{kind: ctlBreak, label: x.Label, v: v})
	case *ast.ContinueExpr:
		panic(cvCtl{kind: ctlContinue, label: x.Label})
	case *ast.ReturnExpr:
		v := &cval{k: cvUnit}
		if x.Value != nil {
			var ok bool
			if v, ok = ev.eval(fr, x.Value); !ok {
				return nil, false
			}
		}
		panic(cvCtl{kind: ctlReturn, v: v})
	case *ast.AssignExpr:
		r, ok := ev.valueOf(fr, x.R)
		if !ok {
			return nil, false
		}
		if p, isPath := x.L.(*ast.PathExpr); isPath && p.QSelf == nil && len(p.Path.Segments) == 1 {
			if !fr.setVar(p.Path.Segments[0].Ident.Name, r) {
				return nil, false
			}
			return &cval{k: cvUnit}, true
		}
		if _, isUnderscore := x.L.(*ast.UnderscoreExpr); isUnderscore {
			return &cval{k: cvUnit}, true
		}
		slot, ok := ev.place(fr, x.L)
		if !ok {
			return nil, false
		}
		*slot = *r
		return &cval{k: cvUnit}, true
	case *ast.AssignOpExpr:
		slot, ok := ev.place(fr, x.L)
		if !ok {
			return nil, false
		}
		r, ok := ev.eval(fr, x.R)
		if !ok {
			return nil, false
		}
		nv, ok := binop(x.Op, slot, r)
		if !ok {
			return nil, false
		}
		*slot = *nv
		return &cval{k: cvUnit}, true
	case *ast.ArrayExpr:
		arr := &cval{k: cvArr, elems: make([]*cval, len(x.Elems))}
		for i, el := range x.Elems {
			v, ok := ev.valueOf(fr, el)
			if !ok {
				return nil, false
			}
			arr.elems[i] = v
		}
		return arr, true
	case *ast.RepeatExpr:
		v, ok := ev.eval(fr, x.Elem)
		if !ok {
			return nil, false
		}
		nv, ok := ev.eval(fr, x.Count)
		if !ok {
			return nil, false
		}
		n, ok := smallInt(nv)
		if !ok || n < 0 || n > 1<<24 {
			return nil, false
		}
		arr := &cval{k: cvArr, elems: make([]*cval, n)}
		for i := range arr.elems {
			arr.elems[i] = v.copy()
		}
		return arr, true
	case *ast.TupleExpr:
		if len(x.Elems) == 0 {
			return &cval{k: cvUnit}, true
		}
		tup := &cval{k: cvTuple, elems: make([]*cval, len(x.Elems))}
		for i, el := range x.Elems {
			v, ok := ev.valueOf(fr, el)
			if !ok {
				return nil, false
			}
			tup.elems[i] = v
		}
		return tup, true
	case *ast.StructExpr:
		return ev.evalStruct(fr, x)
	case *ast.FieldExpr:
		v, ok := ev.eval(fr, x.X)
		if !ok {
			return nil, false
		}
		return fieldOf(v, x.Name.Name)
	case *ast.IndexExpr:
		v, ok := ev.eval(fr, x.X)
		if !ok {
			return nil, false
		}
		return ev.index(fr, v, x.Index)
	}
	return nil, false
}

func fieldOf(v *cval, name string) (*cval, bool) {
	switch v.k {
	case cvTuple:
		i, err := strconv.Atoi(name)
		if err != nil || i < 0 || i >= len(v.elems) {
			return nil, false
		}
		return v.elems[i], true
	case cvStruct:
		for i, n := range v.names {
			if n == name {
				return v.elems[i], true
			}
		}
	}
	return nil, false
}

func (ev *constEval) index(fr *cframe, v *cval, ie ast.Expr) (*cval, bool) {
	if v.k != cvArr {
		if v.k == cvStr {
			if r, ok := ie.(*ast.RangeExpr); ok {
				lo, hi, ok := ev.rangeBounds(fr, r, len(v.s))
				if !ok {
					return nil, false
				}
				return &cval{k: cvStr, s: v.s[lo:hi]}, true
			}
		}
		return nil, false
	}
	if r, ok := ie.(*ast.RangeExpr); ok {
		lo, hi, ok := ev.rangeBounds(fr, r, len(v.elems))
		if !ok {
			return nil, false
		}
		// a subslice shares its elements with the base array
		return &cval{k: cvArr, elems: v.elems[lo:hi:hi]}, true
	}
	iv, ok := ev.eval(fr, ie)
	if !ok || iv.k != cvInt {
		return nil, false
	}
	i, ok := smallInt(iv)
	if !ok || i < 0 || i >= int64(len(v.elems)) {
		return nil, false
	}
	return v.elems[i], true
}

func (ev *constEval) rangeBounds(fr *cframe, r *ast.RangeExpr, n int) (int, int, bool) {
	lo, hi := int64(0), int64(n)
	if r.Start != nil {
		v, ok := ev.eval(fr, r.Start)
		if !ok || v.k != cvInt || !v.n.IsInt64() {
			return 0, 0, false
		}
		lo = v.n.Int64()
	}
	if r.End != nil {
		v, ok := ev.eval(fr, r.End)
		if !ok || v.k != cvInt || !v.n.IsInt64() {
			return 0, 0, false
		}
		hi = v.n.Int64()
		if r.Inclusive {
			hi++
		}
	}
	if lo < 0 || hi < lo || hi > int64(n) {
		return 0, 0, false
	}
	return int(lo), int(hi), true
}

// place returns the storage of an assignable expression.
func (ev *constEval) place(fr *cframe, e ast.Expr) (*cval, bool) {
	switch x := e.(type) {
	case *ast.PathExpr:
		if x.QSelf == nil && len(x.Path.Segments) == 1 {
			v, ok := fr.lookup(x.Path.Segments[0].Ident.Name)
			return v, ok && v != nil
		}
	case *ast.ParenExpr:
		return ev.place(fr, x.X)
	case *ast.UnaryExpr:
		if x.Op == ast.UnDeref {
			return ev.place(fr, x.X)
		}
	case *ast.IndexExpr:
		base, ok := ev.place(fr, x.X)
		if !ok {
			return nil, false
		}
		if _, isRange := x.Index.(*ast.RangeExpr); isRange {
			return nil, false
		}
		return ev.index(fr, base, x.Index)
	case *ast.FieldExpr:
		base, ok := ev.place(fr, x.X)
		if !ok {
			return nil, false
		}
		return fieldOf(base, x.Name.Name)
	}
	return nil, false
}

func (ev *constEval) evalBlock(fr *cframe, b *ast.Block) (*cval, bool) {
	n := len(fr.scopes)
	fr.scopes = append(fr.scopes, map[string]*cval{})
	scope := fr.ac.scope
	if m := ev.t.R.BlockMods[b.ID]; m != nil {
		// items declared in the block (`const N: usize = ..;`)
		fr.ac.scope = m
	}
	defer func() { fr.scopes = fr.scopes[:n]; fr.ac.scope = scope }()
	last := &cval{k: cvUnit}
	for i, st := range b.Stmts {
		switch x := st.(type) {
		case *ast.LetStmt:
			if x.Else != nil {
				return nil, false
			}
			if x.Init == nil {
				ip, ok := x.Pat.(*ast.IdentPat)
				if !ok {
					return nil, false
				}
				fr.declare(ip.Name.Name, nil)
				continue
			}
			var lty Ty
			if x.Ty != nil {
				lty = fr.ac.sub(fr.ac.lowerTy(x.Ty))
				ev.hint = lty
			}
			v, ok := ev.valueOf(fr, x.Init)
			if !ok {
				return nil, false
			}
			v = retype(v, lty)
			m, ok := ev.matchPat(fr, x.Pat, v)
			if !ok || !m {
				return nil, false
			}
		case *ast.ExprStmt:
			v, ok := ev.eval(fr, x.Expr)
			if !ok {
				return nil, false
			}
			if i == len(b.Stmts)-1 && !x.Semi {
				last = v
			}
		case *ast.ItemStmt:
			// nested items are resolved by path
		default:
			return nil, false
		}
	}
	return last, true
}

func (ev *constEval) loopBody(fr *cframe, body *ast.Block, label string) (brk bool, v *cval, ok bool) {
	defer func() {
		if r := recover(); r != nil {
			c, isCtl := r.(cvCtl)
			if !isCtl || c.kind == ctlReturn || (c.label != "" && c.label != label) {
				panic(r)
			}
			brk, v, ok = c.kind == ctlBreak, c.v, true
		}
	}()
	_, ok = ev.evalBlock(fr, body)
	return false, nil, ok
}

func (ev *constEval) labeledBlock(fr *cframe, x *ast.BlockExpr) (v *cval, ok bool) {
	defer func() {
		if r := recover(); r != nil {
			c, isCtl := r.(cvCtl)
			if !isCtl || c.kind != ctlBreak || c.label != x.Label {
				panic(r)
			}
			v, ok = c.v, true
			if v == nil {
				v = &cval{k: cvUnit}
			}
		}
	}()
	return ev.evalBlock(fr, x.Block)
}

// ---------------------------------------------------------------------------
// Patterns

func (ev *constEval) matchPat(fr *cframe, p ast.Pat, v *cval) (bool, bool) {
	if v == nil {
		return false, false
	}
	switch x := p.(type) {
	case *ast.WildPat:
		return true, true
	case *ast.ParenPat:
		return ev.matchPat(fr, x.Pat, v)
	case *ast.RefPat:
		return ev.matchPat(fr, x.Pat, v)
	case *ast.IdentPat:
		if !x.ByRef && !x.Mut && x.Sub == nil {
			// a unit struct / variant / const in scope is a value pattern
			pr, err := ev.t.R.ResolvePath(fr.ac.scope, []string{x.Name.Name}, false, ValueNS, fr.ac.firstLookup)
			if err == nil && pr.Consumed == 1 && (pr.Res.Kind == ResDef || pr.Res.Kind == ResCtor) {
				switch pr.Res.Def.Kind {
				case DefConst, DefStruct, DefVariant:
					c, ok := ev.resValue(fr, pr.Res)
					if !ok {
						return false, false
					}
					return cvEqual(c, v), true
				}
			}
		}
		if x.Sub != nil {
			m, ok := ev.matchPat(fr, x.Sub, v)
			if !ok || !m {
				return m, ok
			}
		}
		if x.ByRef {
			fr.declare(x.Name.Name, v)
		} else {
			fr.declare(x.Name.Name, v.copy())
		}
		return true, true
	case *ast.LitPat:
		c, ok := ev.eval(fr, x.Expr)
		if !ok {
			return false, false
		}
		return cvEqual(c, v), true
	case *ast.RangePat:
		if v.k != cvInt {
			return false, false
		}
		if x.Lo != nil {
			lo, ok := ev.eval(fr, x.Lo)
			if !ok {
				return false, false
			}
			if c, _ := binop(ast.BinLt, v, lo); c == nil || c.b {
				return false, c != nil
			}
		}
		if x.Hi != nil {
			hi, ok := ev.eval(fr, x.Hi)
			if !ok {
				return false, false
			}
			op := ast.BinGe
			if x.Inclusive {
				op = ast.BinGt
			}
			if c, _ := binop(op, v, hi); c == nil || c.b {
				return false, c != nil
			}
		}
		return true, true
	case *ast.OrPat:
		for _, a := range x.Alts {
			m, ok := ev.matchPat(fr, a, v)
			if !ok || m {
				return m, ok
			}
		}
		return false, true
	case *ast.TuplePat:
		if v.k == cvUnit && len(x.Elems) == 0 {
			return true, true
		}
		if v.k != cvTuple {
			return false, false
		}
		return ev.matchSeq(fr, x.Elems, v.elems)
	case *ast.SlicePat:
		if v.k != cvArr {
			return false, false
		}
		hasRest := false
		for _, e := range x.Elems {
			if _, ok := e.(*ast.RestPat); ok {
				hasRest = true
			}
		}
		if !hasRest && len(x.Elems) != len(v.elems) {
			return false, true
		}
		if hasRest && len(x.Elems)-1 > len(v.elems) {
			return false, true
		}
		return ev.matchSeq(fr, x.Elems, v.elems)
	case *ast.PathPat:
		c, ok := ev.evalPath(fr, &ast.PathExpr{QSelf: x.QSelf, Path: x.Path})
		if !ok {
			return false, false
		}
		return cvEqual(c, v), true
	case *ast.TupleStructPat:
		d := ev.ctorDef(fr, x.QSelf, x.Path)
		if d == nil {
			return false, false
		}
		if v.k != cvStruct || v.def != d {
			return false, v.k == cvStruct
		}
		return ev.matchSeq(fr, x.Elems, v.elems)
	case *ast.StructPat:
		d := ev.ctorDef(fr, x.QSelf, x.Path)
		if d == nil {
			return false, false
		}
		if v.k != cvStruct || v.def != d {
			return false, v.k == cvStruct
		}
		for _, f := range x.Fields {
			fv, ok := fieldOf(v, f.Name.Name)
			if !ok {
				return false, false
			}
			m, ok := ev.matchPat(fr, f.Pat, fv)
			if !ok || !m {
				return m, ok
			}
		}
		return true, true
	}
	return false, false
}

// matchSeq matches tuple/slice element patterns (with an optional `..`).
func (ev *constEval) matchSeq(fr *cframe, pats []ast.Pat, vals []*cval) (bool, bool) {
	rest := -1
	for i, p := range pats {
		if _, ok := p.(*ast.RestPat); ok {
			rest = i
		}
	}
	if rest < 0 {
		if len(pats) != len(vals) {
			return false, false
		}
		for i, p := range pats {
			if m, ok := ev.matchPat(fr, p, vals[i]); !ok || !m {
				return m, ok
			}
		}
		return true, true
	}
	after := len(pats) - rest - 1
	if rest+after > len(vals) {
		return false, true
	}
	for i := 0; i < rest; i++ {
		if m, ok := ev.matchPat(fr, pats[i], vals[i]); !ok || !m {
			return m, ok
		}
	}
	for i := 0; i < after; i++ {
		if m, ok := ev.matchPat(fr, pats[rest+1+i], vals[len(vals)-after+i]); !ok || !m {
			return m, ok
		}
	}
	return true, true
}

// ctorDef resolves the struct/variant named by a pattern or struct
// expression path.
func (ev *constEval) ctorDef(fr *cframe, qself *ast.QSelf, p *ast.Path) *Def {
	if qself != nil {
		return nil
	}
	segs := pathSegs(p)
	for _, ns := range []Namespace{TypeNS, ValueNS} {
		pr, err := ev.t.R.ResolvePath(fr.ac.scope, segs, p.Global, ns, fr.ac.firstLookup)
		if err != nil || pr.Consumed != len(segs) {
			continue
		}
		switch pr.Res.Kind {
		case ResDef, ResCtor:
			if d := pr.Res.Def; d.Kind == DefStruct || d.Kind == DefVariant {
				return d
			}
		case ResSelfTy, ResSelfCtor:
			if self := ev.selfTyOf(fr.ac, pr.Res); self != nil && self.Kind == TAdt && self.Def.Kind == DefStruct {
				return self.Def
			}
		}
	}
	return nil
}

func (ev *constEval) evalStruct(fr *cframe, x *ast.StructExpr) (*cval, bool) {
	d := ev.ctorDef(fr, x.QSelf, x.Path)
	if d == nil {
		return nil, false
	}
	var fieldNames []string
	switch {
	case d.Kind == DefVariant:
		ev.t.AdtFields(d.Parent)
		if d.Variant == nil {
			return nil, false
		}
		for _, f := range d.Variant.Fields {
			fieldNames = append(fieldNames, f.Name)
		}
	case d.Kind == DefStruct:
		ev.t.AdtFields(d)
		if d.Adt == nil || len(d.Adt.Variants) == 0 {
			return nil, false
		}
		for _, f := range d.Adt.Variants[0].Fields {
			fieldNames = append(fieldNames, f.Name)
		}
	default:
		return nil, false
	}
	sv := &cval{k: cvStruct, def: d, names: fieldNames, elems: make([]*cval, len(fieldNames))}
	var base *cval
	if x.Base != nil {
		var ok bool
		if base, ok = ev.valueOf(fr, x.Base); !ok || base.k != cvStruct {
			return nil, false
		}
	}
	for _, f := range x.Fields {
		var v *cval
		var ok bool
		if f.Expr == nil {
			v, ok = ev.valueOf(fr, &ast.PathExpr{Path: &ast.Path{Segments: []*ast.PathSegment{{Ident: f.Name}}}})
		} else {
			v, ok = ev.valueOf(fr, f.Expr)
		}
		if !ok {
			return nil, false
		}
		found := false
		for i, n := range fieldNames {
			if n == f.Name.Name {
				sv.elems[i], found = v, true
			}
		}
		if !found {
			return nil, false
		}
	}
	for i, n := range fieldNames {
		if sv.elems[i] != nil {
			continue
		}
		if base == nil {
			return nil, false
		}
		fv, ok := fieldOf(base, n)
		if !ok {
			return nil, false
		}
		sv.elems[i] = fv
	}
	return sv, true
}

// ---------------------------------------------------------------------------
// Paths

func (ev *constEval) evalPath(fr *cframe, x *ast.PathExpr) (*cval, bool) {
	t := ev.t
	ac := fr.ac
	segs := pathSegs(x.Path)
	if x.QSelf == nil && len(segs) == 1 && !x.Path.Global {
		if v, ok := fr.lookup(segs[0]); ok {
			return v, v != nil
		}
	}
	if x.QSelf != nil {
		self := ac.sub(ac.lowerTy(x.QSelf.Ty))
		rest := x.Path.Segments[x.QSelf.Position:]
		if len(rest) != 1 {
			return nil, false
		}
		var tr *TraitRef
		if x.QSelf.Position > 0 {
			traitPath := &ast.Path{Global: x.Path.Global, Segments: x.Path.Segments[:x.QSelf.Position], Span: x.Path.Span}
			tr, _ = ac.lowerTraitPath(traitPath, self, x.Span)
			if tr == nil {
				return nil, false
			}
			tr = &TraitRef{Trait: tr.Trait, Args: append([]Ty(nil), tr.Args...)}
			for i := range tr.Args {
				tr.Args[i] = ac.sub(tr.Args[i])
			}
		}
		if v, ok := primConst(self, rest[0].Ident.Name); ok {
			return v, true
		}
		item, args := ev.assocItem(self, tr, rest[0].Ident.Name, DefConst)
		if item == nil {
			return nil, false
		}
		return ev.itemConst(item, args)
	}
	pr, err := t.R.ResolvePath(ac.scope, segs, x.Path.Global, ValueNS, ac.firstLookup)
	if err != nil {
		if len(segs) == 2 && resolve.IsPrimName(segs[0]) {
			return primConst(primTy(segs[0]), segs[1])
		}
		return nil, false
	}
	if pr.Consumed == len(segs) {
		return ev.resValue(fr, pr.Res)
	}
	if pr.Consumed != len(segs)-1 {
		return nil, false
	}
	// type-relative: Type::CONST / Self::CONST / <param>::CONST
	self := ev.baseTy(ac, pr.Res, x.Path.Segments[pr.Consumed-1])
	if self == nil {
		return nil, false
	}
	if v, ok := primConst(self, segs[len(segs)-1]); ok {
		return v, true
	}
	item, args := ev.assocItem(self, nil, segs[len(segs)-1], DefConst)
	if item == nil {
		return nil, false
	}
	return ev.itemConst(item, args)
}

// resValue evaluates a fully resolved value path.
func (ev *constEval) resValue(fr *cframe, res Res) (*cval, bool) {
	t := ev.t
	switch res.Kind {
	case ResDef, ResCtor:
		d := res.Def
		switch d.Kind {
		case DefConst:
			if d.Const == nil || d.Const.Expr == nil || d.Const.Mut {
				return nil, false
			}
			return ev.itemConst(d, nil)
		case DefConstParam:
			gp := t.paramOf[d]
			if gp == nil {
				return nil, false
			}
			ty := fr.ac.sub(fr.ac.paramTy(gp))
			if ty.Kind != TConst {
				return nil, false
			}
			v := cvI(new(big.Int).SetUint64(ty.Val), 0, false)
			if gp.ConstTy != nil {
				v = retype(v, gp.ConstTy)
				if gp.ConstTy.Kind == TBool {
					v = cvB(ty.Val != 0)
				}
			}
			return v, true
		case DefStruct, DefVariant:
			// unit struct / variant value
			return &cval{k: cvStruct, def: d}, true
		}
	case ResSelfCtor:
		if self := ev.selfTyOf(fr.ac, res); self != nil && self.Kind == TAdt && self.Def.Kind == DefStruct {
			return &cval{k: cvStruct, def: self.Def}, true
		}
	}
	return nil, false
}

func (ev *constEval) selfTyOf(ac *astConv, res Res) Ty {
	if ac.selfTy != nil {
		return ac.sub(ac.selfTy)
	}
	if res.Def != nil && res.Def.Kind == DefImpl {
		ev.t.ImplHeader(res.Def)
		return ac.sub(res.Def.Impl.SelfTy)
	}
	return nil
}

// baseTy lowers the type part of a type-relative path (`Type::item`).
// Generic types written without arguments get inference variables.
func (ev *constEval) baseTy(ac *astConv, res Res, seg *ast.PathSegment) Ty {
	t := ev.t
	var ty Ty
	switch res.Kind {
	case ResPrim:
		ty = primTy(res.Prim)
	case ResSelfTy:
		ty = ev.selfTyOf(ac, res)
	case ResDef:
		d := res.Def
		switch d.Kind {
		case DefStruct, DefEnum, DefUnion:
			g := t.Generics(d)
			if seg.Args == nil && len(g.Own()) > 0 {
				args := make([]Ty, len(g.Params))
				for i := range args {
					args[i] = NewInfer(0)
				}
				ty = Adt(d, args)
			} else {
				ty = ac.resToTy(res, seg, seg.Ident.Span)
			}
		case DefTyAlias, DefTyParam:
			ty = ac.sub(ac.resToTy(res, seg, seg.Ident.Span))
		}
	}
	if ty == nil || ty.HasError() || ty.HasParam() {
		return nil
	}
	return ty
}

// assocItem finds the associated item `name` of the given kind on self:
// with a trait ref, from the selected impl (or the trait's default); without
// one, from an inherent impl or else from an implemented trait declaring it.
// It returns the item and the args of its parent (impl or trait) generics.
func (ev *constEval) assocItem(self Ty, tr *TraitRef, name string, kind DefKind) (*Def, []Ty) {
	t := ev.t
	t.BuildImpls()
	if tr != nil {
		return ev.traitItem(tr, name, kind)
	}
	if key := SimplifyKey(self); key != "" {
		for _, impl := range ev.inherentImpls(self, key) {
			item, ok := impl.Impl.Items[name]
			if !ok || item.Kind != kind {
				continue
			}
			g := t.Generics(impl)
			ic := newInferCtxt(t)
			args := make([]Ty, len(g.Params))
			for i := range args {
				args[i] = ic.newVar()
			}
			if ic.unify(Subst(impl.Impl.SelfTy, args), self) {
				return item, ic.resolveList(args)
			}
		}
	}
	if self.HasInfer() {
		return nil, nil
	}
	for _, trd := range t.traitsByItem[name] {
		if item := trd.Trait.Items[name]; item == nil || item.Kind != kind {
			continue
		}
		if len(t.Generics(trd).Params) != 1 {
			continue
		}
		if item, args := ev.traitItem(&TraitRef{Trait: trd, Args: []Ty{self}}, name, kind); item != nil {
			return item, args
		}
	}
	return nil, nil
}

func (ev *constEval) traitItem(tr *TraitRef, name string, kind DefKind) (*Def, []Ty) {
	t := ev.t
	for _, a := range tr.Args {
		if a.HasInfer() || a.HasParam() || a.HasError() {
			return nil, nil
		}
	}
	for _, st := range t.AllSupertraits(tr) {
		decl, ok := st.Trait.Trait.Items[name]
		if !ok || decl.Kind != kind {
			continue
		}
		var impl *Def
		var args []Ty
		if t.implsIndexed {
			impl, args = t.SelectImplSimple(st)
		} else {
			impl, args = ev.selectImplUnindexed(st)
		}
		if impl == nil {
			return nil, nil
		}
		if item, ok := impl.Impl.Items[name]; ok && item.Kind == kind {
			return item, args
		}
		return decl, st.Args
	}
	return nil, nil
}

// While BuildImpls is still running (struct fields and impl headers are
// lowered first, and may contain array lengths using associated consts) the
// impl index is incomplete, so impls are found by scanning the AST instead.

func (ev *constEval) inherentImpls(self Ty, key string) []*Def {
	t := ev.t
	if t.implsIndexed {
		return t.inherent[key]
	}
	name := ""
	switch self.Kind {
	case TAdt:
		name = self.Def.Name
	case TInt, TFloat, TBool, TChar, TStr:
		name = self.String()
	}
	var out []*Def
	for _, impl := range t.R.Impls {
		ia := impl.Impl.Ast
		if ia == nil || ia.Trait != nil {
			continue
		}
		if pt, ok := ia.SelfTy.(*ast.PathType); ok && name != "" && pt.QSelf == nil &&
			pt.Path.Segments[len(pt.Path.Segments)-1].Ident.Name != name {
			continue
		}
		t.ImplHeader(impl)
		if impl.Impl.SelfTy != nil && SimplifyKey(impl.Impl.SelfTy) == key {
			out = append(out, impl)
		}
	}
	return out
}

func (ev *constEval) selectImplUnindexed(tr *TraitRef) (*Def, []Ty) {
	t := ev.t
	for _, impl := range t.R.Impls {
		ia := impl.Impl.Ast
		if ia == nil || ia.Trait == nil || ia.Negative ||
			ia.Trait.Segments[len(ia.Trait.Segments)-1].Ident.Name != tr.Trait.Name {
			continue
		}
		t.ImplHeader(impl)
		if impl.Impl.TraitRef == nil || impl.Impl.TraitRef.Trait != tr.Trait {
			continue
		}
		ic := newInferCtxt(t)
		if args, ok, ambig := ic.matchImpl(impl, tr, 0); ok && !ambig {
			return impl, ic.resolveList(args)
		}
	}
	return nil, nil
}

// itemConst evaluates a const item (args: its parent's generic args, for
// associated consts).
func (ev *constEval) itemConst(d *Def, args []Ty) (*cval, bool) {
	t := ev.t
	if d.Const == nil || d.Const.Expr == nil {
		return nil, false
	}
	cache := len(args) == 0
	if cache {
		if cv, ok := t.constCache[d]; ok {
			return cv.V, cv.OK
		}
		t.constCache[d] = &ConstValue{}
	}
	t.Generics(d)
	ac := t.itemConv(d)
	if len(args) > 0 {
		ac.subst = args
	}
	sub := &constEval{t: t, depth: ev.depth + 1}
	var v *cval
	ok := false
	if sub.depth <= constMaxDepth {
		if cty := t.ConstTy(d); !cty.HasError() {
			sub.hint = ac.sub(cty)
		}
		v, ok = sub.evalTop(ac, d.Const.Expr)
	}
	ev.steps += sub.steps
	if ok {
		ty := t.ConstTy(d)
		if len(args) > 0 {
			ty = Subst(ty, args)
		}
		v = retype(v, ty)
	} else {
		v = nil
	}
	if cache {
		cv := &ConstValue{OK: ok, V: v}
		if ok {
			cv.Val, _ = cvToInt64(v)
		}
		t.constCache[d] = cv
	}
	return v, ok
}

// ---------------------------------------------------------------------------
// Calls

func (ev *constEval) evalCall(fr *cframe, x *ast.CallExpr, exp Ty) (*cval, bool) {
	t := ev.t
	ac := fr.ac
	pe, isPath := x.Fn.(*ast.PathExpr)
	if !isPath {
		return nil, false
	}
	last := pe.Path.Segments[len(pe.Path.Segments)-1]
	if pe.QSelf != nil {
		self := ac.sub(ac.lowerTy(pe.QSelf.Ty))
		rest := pe.Path.Segments[pe.QSelf.Position:]
		if len(rest) != 1 {
			return nil, false
		}
		var tr *TraitRef
		if pe.QSelf.Position > 0 {
			traitPath := &ast.Path{Global: pe.Path.Global, Segments: pe.Path.Segments[:pe.QSelf.Position], Span: pe.Path.Span}
			tr, _ = ac.lowerTraitPath(traitPath, self, pe.Span)
			if tr == nil {
				return nil, false
			}
			tr = &TraitRef{Trait: tr.Trait, Args: append([]Ty(nil), tr.Args...)}
			for i := range tr.Args {
				tr.Args[i] = ac.sub(tr.Args[i])
			}
		}
		item, args := ev.assocItem(self, tr, last.Ident.Name, DefFn)
		if item == nil {
			return nil, false
		}
		return ev.callFn(fr, item, args, last, x.Args, nil, exp)
	}
	segs := pathSegs(pe.Path)
	pr, err := t.R.ResolvePath(ac.scope, segs, pe.Path.Global, ValueNS, ac.firstLookup)
	if err != nil {
		return nil, false
	}
	if pr.Consumed == len(segs) {
		switch pr.Res.Kind {
		case ResDef, ResCtor:
			d := pr.Res.Def
			switch d.Kind {
			case DefFn:
				if d.IsAssoc() {
					return nil, false
				}
				return ev.callFn(fr, d, nil, last, x.Args, nil, exp)
			case DefStruct, DefVariant:
				return ev.tupleCtor(fr, d, x.Args)
			}
		case ResSelfCtor:
			if self := ev.selfTyOf(ac, pr.Res); self != nil && self.Kind == TAdt && self.Def.Kind == DefStruct {
				return ev.tupleCtor(fr, self.Def, x.Args)
			}
		}
		return nil, false
	}
	if pr.Consumed != len(segs)-1 {
		return nil, false
	}
	self := ev.baseTy(ac, pr.Res, pe.Path.Segments[pr.Consumed-1])
	if self == nil {
		return nil, false
	}
	if self.Kind == TInt || self.Kind == TFloat {
		args := make([]*cval, len(x.Args))
		argsOK := true
		for i, a := range x.Args {
			v, ok := ev.eval(fr, a)
			if !ok {
				argsOK = false
				break
			}
			args[i] = v
		}
		if argsOK {
			if v, ok := ev.primAssocFn(self, last.Ident.Name, args); ok {
				return v, true
			}
		}
	}
	if self.Kind == TAdt && self.Def.Kind == DefEnum {
		// Self::Variant(..) through an alias / Self
		for _, v := range self.Def.Adt.Variants {
			if v.Def != nil && v.Def.Name == last.Ident.Name {
				return ev.tupleCtor(fr, v.Def, x.Args)
			}
		}
	}
	item, args := ev.assocItem(self, nil, last.Ident.Name, DefFn)
	if item == nil {
		return nil, false
	}
	return ev.callFn(fr, item, args, last, x.Args, nil, exp)
}

// inferArgs infers the callee's own generic args that were not given
// explicitly (TyErr in subst) from the expected type of the call.
func (ev *constEval) inferArgs(d *Def, g *Generics, subst []Ty, exp Ty, vals []*cval) {
	missing := false
	for i := g.OwnStart; i < len(subst); i++ {
		if subst[i] == TyErr {
			missing = true
		}
	}
	if !missing {
		return
	}
	t := ev.t
	ic := newInferCtxt(t)
	vs := append([]Ty(nil), subst...)
	for i := g.OwnStart; i < len(vs); i++ {
		if vs[i] == TyErr {
			vs[i] = ic.newVar()
		}
	}
	sig := t.FnSig(d)
	off := len(sig.Inputs) - len(vals)
	for i, v := range vals {
		if off+i < 0 {
			continue
		}
		pt := Subst(sig.Inputs[off+i], vs)
		for pt.Kind == TRef || pt.Kind == TPtr {
			pt = pt.Elem
		}
		if vt := ev.valTy(ic, v); vt != nil {
			s := ic.snapshot()
			if !ic.unify(pt, vt) {
				ic.rollback(s)
			}
		}
	}
	if exp != nil && !exp.HasError() && !exp.HasParam() {
		s := ic.snapshot()
		if !ic.unify(Subst(sig.Output, vs), exp) {
			ic.rollback(s)
		}
	}
	r := ic.resolveList(vs)
	for i := g.OwnStart; i < len(subst); i++ {
		if subst[i] == TyErr && !r[i].HasInfer() && !r[i].HasError() {
			subst[i] = r[i]
		}
	}
}

// valTy approximates the type of a value, for generic arg inference (unknown
// parts are inference variables).
func (ev *constEval) valTy(ic *InferCtxt, v *cval) Ty {
	switch v.k {
	case cvInt:
		if v.typed {
			return IntT(v.ity)
		}
	case cvBool:
		return TyBool
	case cvFloat:
		if v.typed {
			if v.f32 {
				return TyF32
			}
			return TyF64
		}
	case cvStr:
		return TyStr
	case cvUnit:
		return TyUnit
	case cvArr:
		var et Ty
		if len(v.elems) > 0 {
			et = ev.valTy(ic, v.elems[0])
		}
		if et == nil {
			et = ic.newVar()
		}
		return ArrayN(et, uint64(len(v.elems)))
	case cvTuple:
		es := make([]Ty, len(v.elems))
		for i, e := range v.elems {
			if es[i] = ev.valTy(ic, e); es[i] == nil {
				es[i] = ic.newVar()
			}
		}
		return Tuple(es)
	}
	return ic.newVar()
}

// intrinsic evaluates the compiler intrinsics usable in constants.
func (ev *constEval) intrinsic(name string, targs []Ty) (*cval, bool) {
	if len(targs) != 1 {
		return nil, false
	}
	switch name {
	case "size_of", "align_of", "min_align_of", "pref_align_of":
		size, align, ok := ev.t.CLayout(targs[0])
		if !ok {
			return nil, false
		}
		if name == "size_of" {
			return cvIi(int64(size), Usize, true), true
		}
		return cvIi(int64(align), Usize, true), true
	}
	return nil, false
}

func (ev *constEval) tupleCtor(fr *cframe, d *Def, argExprs []ast.Expr) (*cval, bool) {
	sv := &cval{k: cvStruct, def: d, elems: make([]*cval, len(argExprs)), names: make([]string, len(argExprs))}
	for i, a := range argExprs {
		v, ok := ev.valueOf(fr, a)
		if !ok {
			return nil, false
		}
		sv.elems[i] = v
		sv.names[i] = strconv.Itoa(i)
	}
	return sv, true
}

// callFn evaluates a call of fn d. parentArgs are the args of its parent
// impl/trait generics; seg carries explicit generic args (turbofish); recv is
// the receiver of a method call (nil for path calls).
func (ev *constEval) callFn(fr *cframe, d *Def, parentArgs []Ty, seg *ast.PathSegment, argExprs []ast.Expr, recv *cval, exp Ty) (res *cval, ok bool) {
	t := ev.t
	fa := d.Fn.Ast
	if fa == nil || (fa.Body == nil && d.Intrinsic == "") || d.Fn.Extern && d.Intrinsic == "" || fa.Async {
		return nil, false
	}
	nparams := len(fa.Decl.Params)
	hasSelf := fa.Decl.SelfParam != nil
	if hasSelf && recv == nil {
		nparams++
	}
	if nparams != len(argExprs) {
		return nil, false
	}
	if ev.depth >= constMaxDepth {
		return nil, false
	}
	g := t.Generics(d)
	subst := make([]Ty, len(g.Params))
	for i := range subst {
		if i < len(parentArgs) && i < g.OwnStart {
			subst[i] = parentArgs[i]
		} else {
			subst[i] = TyErr
		}
	}
	if seg != nil && seg.Args != nil && !seg.Args.Parenthesized {
		own := g.Own()
		j := 0
		for _, a := range seg.Args.Args {
			if a.Lifetime != "" {
				continue
			}
			if j >= len(own) {
				return nil, false
			}
			gp := own[j]
			j++
			if gp.Const {
				ce := a.Const
				if ce == nil {
					// a bare `N` parses as a type
					pt, isPath := a.Type.(*ast.PathType)
					if !isPath || pt.QSelf != nil {
						return nil, false
					}
					ce = &ast.PathExpr{Path: pt.Path}
				}
				v, ok := ev.eval(fr, ce)
				if !ok {
					return nil, false
				}
				n, ok := intOf(v)
				if !ok {
					return nil, false
				}
				subst[gp.Index] = ConstVal(wrapInt(n, U64).Uint64())
			} else if a.Type != nil {
				subst[gp.Index] = fr.ac.sub(fr.ac.lowerTy(a.Type))
			}
		}
	}
	// evaluate the arguments (in the caller's frame)
	var sig *FnSig
	if fa.Body != nil {
		sig = t.FnSig(d)
	}
	off := 0
	if hasSelf {
		off = 1
	}
	ai := 0
	if hasSelf {
		if recv == nil {
			v, ok := ev.valueOf(fr, argExprs[0])
			if !ok {
				return nil, false
			}
			recv = v
			ai = 1
		} else if fa.Decl.SelfParam.Kind == ast.SelfValue {
			recv = recv.copy()
		}
	}
	vals := make([]*cval, len(fa.Decl.Params))
	for i := range fa.Decl.Params {
		if sig != nil && off+i < len(sig.Inputs) {
			ev.hint = Subst(sig.Inputs[off+i], subst)
		}
		v, ok := ev.valueOf(fr, argExprs[ai+i])
		if !ok {
			return nil, false
		}
		vals[i] = v
	}
	ev.inferArgs(d, g, subst, exp, vals)
	if d.Intrinsic != "" && fa.Body == nil {
		return ev.intrinsic(d.Intrinsic, subst[g.OwnStart:])
	}
	t.Generics(d)
	fac := t.itemConv(d)
	fac.subst = subst
	nfr := &cframe{ac: fac, scopes: []map[string]*cval{{}}}
	if hasSelf {
		nfr.declare("self", recv)
	}
	for i, p := range fa.Decl.Params {
		v := vals[i]
		if off+i < len(sig.Inputs) {
			v = retype(v, Subst(sig.Inputs[off+i], subst))
		}
		m, ok := ev.matchPat(nfr, p.Pat, v)
		if !ok || !m {
			return nil, false
		}
	}
	ev.depth++
	defer func() {
		ev.depth--
		if r := recover(); r != nil {
			c, isCtl := r.(cvCtl)
			if !isCtl || c.kind != ctlReturn {
				panic(r)
			}
			res, ok = retype(c.v, Subst(sig.Output, subst)), true
		}
	}()
	v, ok := ev.evalBlock(nfr, fa.Body)
	if !ok {
		return nil, false
	}
	return retype(v, Subst(sig.Output, subst)), true
}

func (ev *constEval) evalMethod(fr *cframe, x *ast.MethodCallExpr) (*cval, bool) {
	recv, ok := ev.eval(fr, x.Receiver)
	if !ok {
		return nil, false
	}
	args := make([]*cval, len(x.Args))
	evalArgs := func() bool {
		for i, a := range x.Args {
			v, ok := ev.eval(fr, a)
			if !ok {
				return false
			}
			args[i] = v
		}
		return true
	}
	name := x.Method.Name
	switch recv.k {
	case cvStr:
		switch name {
		case "len":
			return cvIi(int64(len(recv.s)), Usize, true), true
		case "is_empty":
			return cvB(len(recv.s) == 0), true
		case "as_bytes":
			arr := &cval{k: cvArr, elems: make([]*cval, len(recv.s))}
			for i := 0; i < len(recv.s); i++ {
				arr.elems[i] = cvIi(int64(recv.s[i]), U8, true)
			}
			return arr, true
		case "starts_with", "ends_with", "contains":
			if !evalArgs() || len(args) != 1 || args[0].k != cvStr {
				return nil, false
			}
			switch name {
			case "starts_with":
				return cvB(strings.HasPrefix(recv.s, args[0].s)), true
			case "ends_with":
				return cvB(strings.HasSuffix(recv.s, args[0].s)), true
			}
			return cvB(strings.Contains(recv.s, args[0].s)), true
		}
		return nil, false
	case cvArr:
		switch name {
		case "len":
			return cvIi(int64(len(recv.elems)), Usize, true), true
		case "is_empty":
			return cvB(len(recv.elems) == 0), true
		case "as_slice", "as_mut_slice":
			return recv, true
		}
		return nil, false
	case cvBool:
		if name == "then_some" || name == "then" {
			return nil, false
		}
	case cvInt, cvFloat:
		if !evalArgs() {
			return nil, false
		}
		if v, ok := ev.numMethod(recv, name, args); ok {
			return v, true
		}
		// a (trait) method on a primitive defined in Rust source
		var self Ty
		switch {
		case recv.k == cvFloat && recv.typed:
			self = TyF64
			if recv.f32 {
				self = TyF32
			}
		case recv.k == cvInt && recv.typed:
			self = IntT(recv.ity)
		default:
			return nil, false
		}
		item, iargs := ev.assocItem(self, nil, name, DefFn)
		if item == nil || item.Fn.Ast == nil || item.Fn.Ast.Decl.SelfParam == nil {
			return nil, false
		}
		seg := &ast.PathSegment{Ident: x.Method, Args: x.Generics}
		return ev.callFn(fr, item, iargs, seg, x.Args, recv, nil)
	case cvStruct:
		if v, ok := ev.optionMethod(recv, name, nil); ok && len(x.Args) == 0 {
			return v, true
		}
		if name == "unwrap_or" && len(x.Args) == 1 {
			if !evalArgs() {
				return nil, false
			}
			if v, ok := ev.optionMethod(recv, name, args); ok {
				return v, true
			}
		}
		d := recv.def
		if d.Kind == DefVariant {
			d = d.Parent
		}
		self := ev.baseTy(fr.ac, Res{Kind: ResDef, Def: d}, &ast.PathSegment{Ident: ast.Ident{Name: d.Name}})
		if self == nil {
			return nil, false
		}
		item, iargs := ev.assocItem(self, nil, name, DefFn)
		if item == nil || item.Fn.Ast == nil || item.Fn.Ast.Decl.SelfParam == nil {
			return nil, false
		}
		for i, a := range iargs {
			if a.HasInfer() {
				iargs[i] = TyErr
			}
		}
		seg := &ast.PathSegment{Ident: x.Method, Args: x.Generics}
		return ev.callFn(fr, item, iargs, seg, x.Args, recv, nil)
	}
	return nil, false
}

// IsPrimInt reports whether s names an integer type.
func IsPrimInt(s string) bool {
	_, ok := IntTyByName(s)
	return ok && !strings.HasPrefix(s, "f")
}
