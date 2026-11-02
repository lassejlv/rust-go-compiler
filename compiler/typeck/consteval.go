package typeck

import (
	"strconv"
	"strings"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
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
	cvStr
	cvArr // arrays and slices
	cvTuple
	cvStruct // structs, tuple structs and enum variants
)

type cval struct {
	k     cvKind
	i     int64
	ity   IntTy
	typed bool // ity is known (literal suffix, cast, declared type)
	s     string
	elems []*cval
	def   *Def     // cvStruct: struct or variant def
	names []string // cvStruct: field names parallel to elems
}

func cvI(v int64, it IntTy, typed bool) *cval {
	if typed {
		v = truncInt(v, it)
	}
	return &cval{k: cvInt, i: v, ity: it, typed: typed}
}

func cvB(b bool) *cval { return &cval{k: cvBool, i: b2i(b)} }

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
		return (b.k == cvInt || b.k == cvBool) && a.i == b.i
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

// retype converts an integer value to the given type (if it is an integer
// type).
func retype(v *cval, ty Ty) *cval {
	if v != nil && v.k == cvInt && ty != nil && ty.Kind == TInt {
		return cvI(v.i, ty.IntKind(), true)
	}
	return v
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
	if !ok || v == nil || (v.k != cvInt && v.k != cvBool) {
		return 0, false
	}
	return v.i, true
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
	if !ok || (v.k != cvBool && v.k != cvInt) {
		return false, false
	}
	return v.i != 0, true
}

func (ev *constEval) eval(fr *cframe, e ast.Expr) (*cval, bool) {
	ev.steps++
	if ev.steps > constMaxSteps {
		return nil, false
	}
	switch x := e.(type) {
	case *ast.LitExpr:
		switch x.Kind {
		case ast.LitInt:
			if x.Big != "" {
				return nil, false
			}
			if x.Suffix != "" {
				it, ok := IntTyByName(x.Suffix)
				if !ok {
					return nil, false
				}
				return cvI(int64(x.IntVal), it, true), true
			}
			return cvI(int64(x.IntVal), 0, false), true
		case ast.LitChar:
			return cvI(int64(x.IntVal), 0, false), true
		case ast.LitByte:
			return cvI(int64(x.IntVal), U8, true), true
		case ast.LitBool:
			return cvB(x.Value == "true"), true
		case ast.LitStr:
			return &cval{k: cvStr, s: x.Value}, true
		case ast.LitByteStr:
			arr := &cval{k: cvArr, elems: make([]*cval, len(x.Value))}
			for i := 0; i < len(x.Value); i++ {
				arr.elems[i] = cvI(int64(x.Value[i]), U8, true)
			}
			return arr, true
		}
		return nil, false
	case *ast.ParenExpr:
		return ev.eval(fr, x.X)
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
		ty := fr.ac.lowerTy(x.Ty)
		switch ty.Kind {
		case TInt:
			if v.k != cvInt && v.k != cvBool {
				return nil, false
			}
			return cvI(v.i, ty.IntKind(), true), true
		case TChar:
			if v.k != cvInt {
				return nil, false
			}
			return cvI(v.i, 0, false), true
		case TFloat, TError:
			return nil, false
		}
		return v, true
	case *ast.UnaryExpr:
		v, ok := ev.eval(fr, x.X)
		if !ok {
			return nil, false
		}
		switch x.Op {
		case ast.UnNeg:
			if v.k != cvInt {
				return nil, false
			}
			return cvI(-v.i, v.ity, v.typed), true
		case ast.UnNot:
			switch v.k {
			case cvBool:
				return cvB(v.i == 0), true
			case cvInt:
				return cvI(^v.i, v.ity, v.typed), true
			}
			return nil, false
		case ast.UnDeref:
			return v, true
		}
		return nil, false
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
		return ev.evalCall(fr, x)
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
		end := hi.i
		if r.Inclusive {
			end++
		}
		for i := lo.i; i < end; i++ {
			n := len(fr.scopes)
			fr.scopes = append(fr.scopes, map[string]*cval{})
			m, ok := ev.matchPat(fr, x.Pat, cvI(i, it, typed))
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
		n, ok := ev.eval(fr, x.Count)
		if !ok || n.k != cvInt || n.i < 0 || n.i > 1<<24 {
			return nil, false
		}
		arr := &cval{k: cvArr, elems: make([]*cval, n.i)}
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
	i, ok := ev.eval(fr, ie)
	if !ok || i.k != cvInt || i.i < 0 || i.i >= int64(len(v.elems)) {
		return nil, false
	}
	return v.elems[i.i], true
}

func (ev *constEval) rangeBounds(fr *cframe, r *ast.RangeExpr, n int) (int, int, bool) {
	lo, hi := int64(0), int64(n)
	if r.Start != nil {
		v, ok := ev.eval(fr, r.Start)
		if !ok || v.k != cvInt {
			return 0, 0, false
		}
		lo = v.i
	}
	if r.End != nil {
		v, ok := ev.eval(fr, r.End)
		if !ok || v.k != cvInt {
			return 0, 0, false
		}
		hi = v.i
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
	defer func() { fr.scopes = fr.scopes[:n] }()
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
			v, ok := ev.valueOf(fr, x.Init)
			if !ok {
				return nil, false
			}
			if x.Ty != nil && v.k == cvInt {
				v = retype(v, fr.ac.lowerTy(x.Ty))
			}
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

func binop(op ast.BinOp, a, b *cval) (*cval, bool) {
	if a.k != cvInt || b.k != cvInt {
		if a.k == cvBool && b.k == cvBool {
			switch op {
			case ast.BinBitAnd:
				return cvB(a.i&b.i != 0), true
			case ast.BinBitOr:
				return cvB(a.i|b.i != 0), true
			case ast.BinBitXor:
				return cvB(a.i != b.i), true
			}
		}
		switch op {
		case ast.BinEq:
			return cvB(cvEqual(a, b)), true
		case ast.BinNe:
			return cvB(!cvEqual(a, b)), true
		}
		if (a.k == cvBool || a.k == cvInt) && (b.k == cvBool || b.k == cvInt) {
			switch op {
			case ast.BinLt:
				return cvB(a.i < b.i), true
			case ast.BinLe:
				return cvB(a.i <= b.i), true
			case ast.BinGt:
				return cvB(a.i > b.i), true
			case ast.BinGe:
				return cvB(a.i >= b.i), true
			}
		}
		return nil, false
	}
	it, typed := a.ity, a.typed
	if !typed && op != ast.BinShl && op != ast.BinShr {
		it, typed = b.ity, b.typed
	}
	unsigned := typed && !it.Signed()
	x, y := a.i, b.i
	ux, uy := uint64(x), uint64(y)
	var r int64
	switch op {
	case ast.BinAdd:
		r = x + y
	case ast.BinSub:
		r = x - y
	case ast.BinMul:
		r = x * y
	case ast.BinDiv, ast.BinRem:
		if y == 0 {
			return nil, false
		}
		switch {
		case unsigned && op == ast.BinDiv:
			r = int64(ux / uy)
		case unsigned:
			r = int64(ux % uy)
		case op == ast.BinDiv:
			r = x / y
		default:
			r = x % y
		}
	case ast.BinShl:
		if y < 0 || y >= 128 {
			return nil, false
		}
		r = x << uint(y)
	case ast.BinShr:
		if y < 0 || y >= 128 {
			return nil, false
		}
		if unsigned {
			r = int64(ux >> uint(y))
		} else {
			r = x >> uint(y)
		}
	case ast.BinBitAnd:
		r = x & y
	case ast.BinBitOr:
		r = x | y
	case ast.BinBitXor:
		r = x ^ y
	case ast.BinEq:
		return cvB(x == y), true
	case ast.BinNe:
		return cvB(x != y), true
	case ast.BinLt, ast.BinLe, ast.BinGt, ast.BinGe:
		var c int
		switch {
		case unsigned && ux < uy, !unsigned && x < y:
			c = -1
		case x == y:
			c = 0
		default:
			c = 1
		}
		switch op {
		case ast.BinLt:
			return cvB(c < 0), true
		case ast.BinLe:
			return cvB(c <= 0), true
		case ast.BinGt:
			return cvB(c > 0), true
		default:
			return cvB(c >= 0), true
		}
	default:
		return nil, false
	}
	return cvI(r, it, typed), true
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
			if c, _ := binop(ast.BinLt, v, lo); c == nil || c.i != 0 {
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
			if c, _ := binop(op, v, hi); c == nil || c.i != 0 {
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
		item, args := ev.assocItem(self, tr, rest[0].Ident.Name, DefConst)
		if item == nil {
			return nil, false
		}
		return ev.itemConst(item, args)
	}
	if len(segs) == 2 && IsPrimInt(segs[0]) {
		it, _ := IntTyByName(segs[0])
		switch segs[1] {
		case "MAX":
			return cvI(intMax(it), it, true), true
		case "MIN":
			return cvI(intMin(it), it, true), true
		case "BITS":
			return cvI(int64(it.Bits()), U32, true), true
		}
	}
	pr, err := t.R.ResolvePath(ac.scope, segs, x.Path.Global, ValueNS, ac.firstLookup)
	if err != nil {
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
			v := cvI(int64(ty.Val), 0, false)
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
		for _, impl := range t.inherent[key] {
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
		impl, args := t.SelectImplSimple(st)
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
		if ok && (v.k == cvInt || v.k == cvBool) {
			cv.Val = v.i
		}
		t.constCache[d] = cv
	}
	return v, ok
}

// ---------------------------------------------------------------------------
// Calls

func (ev *constEval) evalCall(fr *cframe, x *ast.CallExpr) (*cval, bool) {
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
		return ev.callFn(fr, item, args, last, x.Args, nil)
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
				return ev.callFn(fr, d, nil, last, x.Args, nil)
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
	return ev.callFn(fr, item, args, last, x.Args, nil)
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
func (ev *constEval) callFn(fr *cframe, d *Def, parentArgs []Ty, seg *ast.PathSegment, argExprs []ast.Expr, recv *cval) (res *cval, ok bool) {
	t := ev.t
	fa := d.Fn.Ast
	if fa == nil || fa.Body == nil || d.Fn.Extern || fa.Async {
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
				if !ok || (v.k != cvInt && v.k != cvBool) {
					return nil, false
				}
				subst[gp.Index] = ConstVal(uint64(v.i))
			} else if a.Type != nil {
				subst[gp.Index] = fr.ac.sub(fr.ac.lowerTy(a.Type))
			}
		}
	}
	sig := t.FnSig(d)
	t.Generics(d)
	fac := t.itemConv(d)
	fac.subst = subst
	nfr := &cframe{ac: fac, scopes: []map[string]*cval{{}}}
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
		nfr.declare("self", recv)
	}
	off := 0
	if hasSelf {
		off = 1
	}
	for i, p := range fa.Decl.Params {
		v, ok := ev.valueOf(fr, argExprs[ai+i])
		if !ok {
			return nil, false
		}
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
			return cvI(int64(len(recv.s)), Usize, true), true
		case "is_empty":
			return cvB(len(recv.s) == 0), true
		case "as_bytes":
			arr := &cval{k: cvArr, elems: make([]*cval, len(recv.s))}
			for i := 0; i < len(recv.s); i++ {
				arr.elems[i] = cvI(int64(recv.s[i]), U8, true)
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
			return cvI(int64(len(recv.elems)), Usize, true), true
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
	case cvInt:
		if !evalArgs() {
			return nil, false
		}
		return intMethod(recv, name, args)
	case cvStruct:
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
		return ev.callFn(fr, item, iargs, seg, x.Args, recv)
	}
	return nil, false
}

func intMethod(v *cval, name string, args []*cval) (*cval, bool) {
	it, typed := v.ity, v.typed
	bits := 0
	if typed {
		bits = it.Bits()
		if bits > 64 {
			return nil, false
		}
	}
	u := uint64(v.i)
	if typed && bits < 64 {
		u &= uint64(1)<<bits - 1
	}
	arg := func(i int) (int64, bool) {
		if i >= len(args) || (args[i].k != cvInt && args[i].k != cvBool) {
			return 0, false
		}
		return args[i].i, true
	}
	switch name {
	case "wrapping_add", "wrapping_sub", "wrapping_mul", "min", "max", "pow", "wrapping_pow", "abs_diff",
		"wrapping_shl", "wrapping_shr", "rotate_left", "rotate_right", "saturating_add", "saturating_sub",
		"wrapping_div", "wrapping_rem", "div_euclid", "rem_euclid":
		if len(args) != 1 {
			return nil, false
		}
	default:
		if len(args) != 0 {
			return nil, false
		}
	}
	switch name {
	case "wrapping_add":
		b, _ := arg(0)
		return cvI(v.i+b, it, typed), true
	case "wrapping_sub":
		b, _ := arg(0)
		return cvI(v.i-b, it, typed), true
	case "wrapping_mul":
		b, _ := arg(0)
		return cvI(v.i*b, it, typed), true
	case "wrapping_div", "wrapping_rem", "div_euclid", "rem_euclid":
		op := ast.BinDiv
		if name == "wrapping_rem" || name == "rem_euclid" {
			op = ast.BinRem
		}
		r, ok := binop(op, v, args[0])
		if ok && name == "rem_euclid" && r.i < 0 {
			b, _ := arg(0)
			if b < 0 {
				b = -b
			}
			r = cvI(r.i+b, r.ity, r.typed)
		}
		return r, ok
	case "min", "max":
		lt, ok := binop(ast.BinLt, v, args[0])
		if !ok {
			return nil, false
		}
		if (lt.i != 0) == (name == "min") {
			return v, true
		}
		return retype(args[0], nil), true
	case "pow", "wrapping_pow":
		e, _ := arg(0)
		if e < 0 {
			return nil, false
		}
		r := int64(1)
		for i := int64(0); i < e; i++ {
			r *= v.i
		}
		return cvI(r, it, typed), true
	case "abs":
		if v.i < 0 {
			return cvI(-v.i, it, typed), true
		}
		return v, true
	case "abs_diff":
		gt, ok := binop(ast.BinGt, v, args[0])
		if !ok {
			return nil, false
		}
		if gt.i != 0 {
			return binop(ast.BinSub, v, args[0])
		}
		return binop(ast.BinSub, args[0], v)
	case "is_power_of_two":
		return cvB(u != 0 && u&(u-1) == 0), true
	case "next_power_of_two":
		p := uint64(1)
		for p < u {
			p <<= 1
		}
		return cvI(int64(p), it, typed), true
	case "saturating_add", "saturating_sub":
		if !typed {
			return nil, false
		}
		b, _ := arg(0)
		var r int64
		if name == "saturating_add" {
			r = v.i + b
		} else {
			r = v.i - b
		}
		if !it.Signed() && bits == 64 {
			ub := uint64(b)
			if name == "saturating_add" && uint64(r) < u {
				return cvI(-1, it, true), true
			}
			if name == "saturating_sub" && ub > u {
				return cvI(0, it, true), true
			}
			return cvI(r, it, true), true
		}
		if bits < 64 {
			if r > intMax(it) {
				r = intMax(it)
			}
			if r < intMin(it) {
				r = intMin(it)
			}
		}
		return cvI(r, it, true), true
	}
	if !typed {
		return nil, false
	}
	switch name {
	case "count_ones", "count_zeros", "leading_zeros", "trailing_zeros", "leading_ones", "trailing_ones":
		n := 0
		switch name {
		case "count_ones", "count_zeros":
			for i := 0; i < bits; i++ {
				if u&(1<<uint(i)) != 0 {
					n++
				}
			}
			if name == "count_zeros" {
				n = bits - n
			}
		case "leading_zeros", "leading_ones":
			want := name == "leading_ones"
			for i := bits - 1; i >= 0 && (u&(1<<uint(i)) != 0) == want; i-- {
				n++
			}
		default:
			want := name == "trailing_ones"
			for i := 0; i < bits && (u&(1<<uint(i)) != 0) == want; i++ {
				n++
			}
		}
		return cvI(int64(n), U32, true), true
	case "rotate_left", "rotate_right", "wrapping_shl", "wrapping_shr":
		s, _ := arg(0)
		s = s % int64(bits)
		if s < 0 {
			s += int64(bits)
		}
		mask := uint64(1)<<uint(bits) - 1
		if bits == 64 {
			mask = ^uint64(0)
		}
		var r uint64
		switch name {
		case "rotate_left":
			r = (u<<uint(s) | u>>uint(int64(bits)-s)) & mask
		case "rotate_right":
			r = (u>>uint(s) | u<<uint(int64(bits)-s)) & mask
		case "wrapping_shl":
			r = (u << uint(s)) & mask
		default:
			if it.Signed() {
				return cvI(v.i>>uint(s), it, true), true
			}
			r = u >> uint(s)
		}
		return cvI(int64(r), it, true), true
	case "swap_bytes", "to_be", "reverse_bits":
		return nil, false
	}
	return nil, false
}

func b2i(b bool) int64 {
	if b {
		return 1
	}
	return 0
}

// IsPrimInt reports whether s names an integer type.
func IsPrimInt(s string) bool {
	_, ok := IntTyByName(s)
	return ok && !strings.HasPrefix(s, "f")
}

func intMax(it IntTy) int64 {
	bits := it.Bits()
	if bits >= 64 {
		if it.Signed() {
			return 1<<63 - 1
		}
		return -1 // u64::MAX as bit pattern
	}
	if it.Signed() {
		return 1<<(bits-1) - 1
	}
	return 1<<bits - 1
}

func intMin(it IntTy) int64 {
	if !it.Signed() {
		return 0
	}
	if it.Bits() >= 64 {
		return -1 << 63
	}
	return -1 << (it.Bits() - 1)
}

func truncInt(v int64, it IntTy) int64 {
	bits := it.Bits()
	if bits >= 64 {
		return v
	}
	mask := int64(1)<<bits - 1
	v &= mask
	if it.Signed() && v&(int64(1)<<(bits-1)) != 0 {
		v -= int64(1) << bits
	}
	return v
}
