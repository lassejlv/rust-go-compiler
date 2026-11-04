package typeck

// Pattern exhaustiveness checking (mirrors rustc_pattern_analysis): `match`
// expressions must be exhaustive (E0004) and the patterns of `let`
// statements without `else`, `for` loops and function/closure parameters
// must be irrefutable (E0005).
//
// Patterns are lowered to "deconstructed patterns" (a constructor applied
// to field patterns) and checked with the usefulness algorithm of
// Maranget ("Warnings for pattern matching"), splitting integer ranges and
// slice lengths like rustc does. Whenever a pattern or type is not modelled
// (e.g. a constant of a non-scalar type) the check gives up and treats the
// patterns as exhaustive: it never rejects a program it does not understand.

import (
	"fmt"
	"math/big"
	"sort"
	"strconv"
	"strings"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

type ctorKind uint8

const (
	ckWild    ctorKind = iota // wildcard / binding
	ckOr                      // or-pattern (alts in dpat.alts)
	ckSingle                  // struct, tuple, reference, box
	ckVariant                 // enum variant idx
	ckBool                    // bool b
	ckRange                   // integer/char range lo..=hi
	ckSlice                   // array/slice of fixed length n, or var length prefix..suffix
	ckOpaque                  // literal that is not enumerable (str, float): covers only itself
)

type ctor struct {
	kind   ctorKind
	idx    int
	b      bool
	lo, hi *big.Int
	// slices: fixed length n (= prefix), or variable length (prefix + `..` + suffix)
	fixed          bool
	prefix, suffix int
	key            string // opaque value
}

func (c ctor) arityOfSlice() int {
	if c.fixed {
		return c.prefix
	}
	return c.prefix + c.suffix
}

// dpat is a deconstructed pattern.
type dpat struct {
	c      ctor
	fields []*dpat
	alts   []*dpat
	ty     Ty
}

func wildOf(ty Ty) *dpat { return &dpat{c: ctor{kind: ckWild}, ty: ty} }

// errBail aborts the analysis of a match whose patterns are not modelled.
type errBail struct{}

type patCx struct {
	t      *Tcx
	r      *Results
	budget int
}

func (cx *patCx) bail() { panic(errBail{}) }

// run executes f, returning false if the analysis gave up.
func (cx *patCx) run(f func()) (ok bool) {
	defer func() {
		if e := recover(); e != nil {
			if _, fatal := e.(fatalError); fatal {
				panic(e)
			}
			// errBail, or anything unexpected: no verdict
			ok = false
		}
	}()
	f()
	return true
}

// ---------------------------------------------------------------------------
// Entry point

// checkPatterns checks the exhaustiveness of matches and the
// irrefutability of binding patterns in a type-checked body.
func (t *Tcx) checkPatterns(r *Results, def *Def) {
	if r == nil {
		return
	}
	cx := &patCx{t: t, r: r}
	var roots []any
	if def.Kind == DefFn && def.Fn != nil && def.Fn.Ast != nil && def.Fn.Ast.Body != nil {
		for _, p := range def.Fn.Ast.Decl.Params {
			cx.checkIrrefutable(p.Pat, "function argument")
		}
		roots = append(roots, def.Fn.Ast.Body)
	} else if r.ConstExpr != nil {
		roots = append(roots, r.ConstExpr)
	}
	for _, root := range roots {
		ast.Walk(root, func(n any) bool {
			switch x := n.(type) {
			case *ast.ItemStmt:
				return false // nested items are separate bodies
			case *ast.MatchExpr:
				cx.checkMatch(x)
			case *ast.LetStmt:
				if x.Else == nil {
					cx.checkIrrefutable(x.Pat, "local binding")
				}
			case *ast.ForExpr:
				cx.checkIrrefutable(x.Pat, "`for` loop binding")
			case *ast.ClosureExpr:
				for _, p := range x.Params {
					cx.checkIrrefutable(p.Pat, "function argument")
				}
			}
			return true
		})
	}
}

// outerTy returns the type a pattern is matched against (before the
// implicit dereferences of match ergonomics).
func (cx *patCx) outerTy(p ast.Pat) Ty {
	if ds := cx.r.PatDerefs[p.GetID()]; len(ds) > 0 {
		return ds[0]
	}
	return cx.r.NodeTypes[p.GetID()]
}

func usableTy(t Ty) bool {
	return t != nil && !t.HasInfer() && !t.HasError() && !t.HasParam() && !t.HasProj()
}

func (cx *patCx) checkMatch(x *ast.MatchExpr) {
	st := cx.r.AdjustedType(x.Scrutinee)
	if !usableTy(st) {
		return
	}
	var ws []*dpat
	ok := cx.run(func() {
		var rows [][]*dpat
		for _, arm := range x.Arms {
			p := cx.lower(arm.Pat, st)
			if arm.Guard != nil {
				continue
			}
			rows = append(rows, []*dpat{p})
		}
		for _, w := range cx.missing(rows, []Ty{st}) {
			ws = append(ws, w[0])
		}
	})
	if !ok || len(ws) == 0 {
		return
	}
	if len(x.Arms) == 0 {
		cx.t.ErrorCode(x.Scrutinee.GetSpan(), "E0004", "non-exhaustive patterns: type `%s` is non-empty", st)
		return
	}
	cx.t.ErrorCode(x.Scrutinee.GetSpan(), "E0004", "non-exhaustive patterns: %s not covered", cx.witnessList(ws))
}

func (cx *patCx) checkIrrefutable(p ast.Pat, what string) {
	if p == nil {
		return
	}
	switch x := p.(type) {
	case *ast.WildPat:
		return
	case *ast.IdentPat:
		if x.Sub == nil && cx.r.Paths[x.ID] == nil {
			return // plain bindings are irrefutable
		}
	}
	ty := cx.outerTy(p)
	if !usableTy(ty) {
		return
	}
	var ws []*dpat
	ok := cx.run(func() {
		rows := [][]*dpat{{cx.lower(p, ty)}}
		for _, w := range cx.missing(rows, []Ty{ty}) {
			ws = append(ws, w[0])
		}
	})
	if !ok || len(ws) == 0 {
		return
	}
	cx.t.ErrorCode(p.GetSpan(), "E0005", "refutable pattern in %s\n  = note: pattern%s %s not covered", what, plural(len(ws)), cx.witnessList(ws))
}

// ---------------------------------------------------------------------------
// Lowering

func (cx *patCx) lower(p ast.Pat, ty Ty) *dpat {
	if ds := cx.r.PatDerefs[p.GetID()]; len(ds) > 0 {
		inner := ds[len(ds)-1].Elem
		res := cx.lowerInner(p, inner)
		for i := len(ds) - 1; i >= 0; i-- {
			if ds[i].Kind != TRef {
				cx.bail()
			}
			res = &dpat{c: ctor{kind: ckSingle}, fields: []*dpat{res}, ty: ds[i]}
		}
		return res
	}
	return cx.lowerInner(p, ty)
}

func isBoxTy(t Ty) bool { return t.Kind == TAdt && t.Def != nil && t.Def.LangItem == "owned_box" }

func (cx *patCx) lowerInner(p ast.Pat, ty Ty) *dpat {
	if !usableTy(ty) {
		cx.bail()
	}
	switch x := p.(type) {
	case *ast.WildPat:
		return wildOf(ty)
	case *ast.ParenPat:
		return cx.lower(x.Pat, ty)
	case *ast.IdentPat:
		if vr := cx.r.Paths[x.ID]; vr != nil {
			return cx.lowerPathValue(vr, ty)
		}
		if x.Sub != nil {
			if _, ok := x.Sub.(*ast.RestPat); ok {
				cx.bail()
			}
			return cx.lower(x.Sub, ty)
		}
		return wildOf(ty)
	case *ast.PathPat:
		vr := cx.r.Paths[x.ID]
		if vr == nil {
			cx.bail()
		}
		return cx.lowerPathValue(vr, ty)
	case *ast.LitPat:
		return cx.lowerLit(x.Expr, ty)
	case *ast.RangePat:
		if ty.Kind != TInt && ty.Kind != TChar {
			if ty.Kind == TFloat {
				return &dpat{c: ctor{kind: ckOpaque, key: fmt.Sprintf("range%d", x.ID)}, ty: ty}
			}
			cx.bail()
		}
		lo, hi := tyMin(ty), tyMax(ty)
		if x.Lo != nil {
			lo = cx.scalarOf(x.Lo, ty)
		}
		if x.Hi != nil {
			hi = cx.scalarOf(x.Hi, ty)
			if !x.Inclusive {
				hi = new(big.Int).Sub(hi, big.NewInt(1))
			}
		}
		if lo.Cmp(hi) > 0 {
			if x.Lo != nil && x.Hi != nil {
				if x.Inclusive {
					cx.t.ErrorCode(x.Span, "E0030", "lower range bound must be less than or equal to upper")
				} else {
					cx.t.ErrorCode(x.Span, "E0579", "lower range bound must be less than upper")
				}
			}
			cx.bail()
		}
		return &dpat{c: ctor{kind: ckRange, lo: lo, hi: hi}, ty: ty}
	case *ast.TuplePat:
		if ty.Kind != TTuple {
			cx.bail()
		}
		return &dpat{c: ctor{kind: ckSingle}, fields: cx.lowerSeq(x.Elems, ty.Elems), ty: ty}
	case *ast.TupleStructPat:
		v := cx.r.Variants[x.ID]
		if v == nil || ty.Kind != TAdt {
			cx.bail()
		}
		fts := cx.fieldTys(ty, v)
		return cx.adtPat(ty, v, cx.lowerSeq(x.Elems, fts))
	case *ast.StructPat:
		v := cx.r.Variants[x.ID]
		if v == nil || ty.Kind != TAdt {
			cx.bail()
		}
		fts := cx.fieldTys(ty, v)
		fields := make([]*dpat, len(fts))
		for i, ft := range fts {
			fields[i] = wildOf(ft)
		}
		for _, f := range x.Fields {
			fi := v.FieldIndex(f.Name.Name)
			if fi < 0 {
				cx.bail()
			}
			fields[fi] = cx.lower(f.Pat, fts[fi])
		}
		return cx.adtPat(ty, v, fields)
	case *ast.RefPat:
		if ty.Kind != TRef {
			cx.bail()
		}
		return &dpat{c: ctor{kind: ckSingle}, fields: []*dpat{cx.lower(x.Pat, ty.Elem)}, ty: ty}
	case *ast.BoxPat:
		if !isBoxTy(ty) || len(ty.Args) == 0 {
			cx.bail()
		}
		return &dpat{c: ctor{kind: ckSingle}, fields: []*dpat{cx.lower(x.Pat, ty.Args[0])}, ty: ty}
	case *ast.SlicePat:
		var elem Ty
		arrLen := -1
		switch ty.Kind {
		case TArray:
			if ty.Len == nil || ty.Len.Kind != TConst || ty.Len.Val > 4096 {
				cx.bail()
			}
			elem, arrLen = ty.Elem, int(ty.Len.Val)
		case TSlice:
			elem = ty.Elem
		default:
			cx.bail()
		}
		var prefix, suffix []*dpat
		rest := false
		for _, e := range x.Elems {
			if isRestLike(e) {
				if rest {
					cx.bail()
				}
				rest = true
				continue
			}
			dp := cx.lower(e, elem)
			if rest {
				suffix = append(suffix, dp)
			} else {
				prefix = append(prefix, dp)
			}
		}
		if !rest {
			if arrLen >= 0 && len(prefix) != arrLen {
				cx.bail()
			}
			return &dpat{c: ctor{kind: ckSlice, fixed: true, prefix: len(prefix)}, fields: prefix, ty: ty}
		}
		if arrLen >= 0 {
			if len(prefix)+len(suffix) > arrLen {
				cx.bail()
			}
			fields := append([]*dpat{}, prefix...)
			for i := len(prefix) + len(suffix); i < arrLen; i++ {
				fields = append(fields, wildOf(elem))
			}
			fields = append(fields, suffix...)
			return &dpat{c: ctor{kind: ckSlice, fixed: true, prefix: arrLen}, fields: fields, ty: ty}
		}
		return &dpat{c: ctor{kind: ckSlice, prefix: len(prefix), suffix: len(suffix)}, fields: append(prefix, suffix...), ty: ty}
	case *ast.OrPat:
		d := &dpat{c: ctor{kind: ckOr}, ty: ty}
		for _, a := range x.Alts {
			d.alts = append(d.alts, cx.lower(a, ty))
		}
		return d
	}
	cx.bail()
	return nil
}

func (cx *patCx) adtPat(ty Ty, v *VariantDef, fields []*dpat) *dpat {
	switch ty.Def.Adt.Kind {
	case AdtEnum:
		return &dpat{c: ctor{kind: ckVariant, idx: v.Index}, fields: fields, ty: ty}
	case AdtStruct:
		if isBoxTy(ty) {
			cx.bail()
		}
		return &dpat{c: ctor{kind: ckSingle}, fields: fields, ty: ty}
	}
	cx.bail() // unions
	return nil
}

// lowerSeq lowers tuple-like subpatterns with an optional `..`.
func (cx *patCx) lowerSeq(pats []ast.Pat, tys []Ty) []*dpat {
	rest := -1
	for i, p := range pats {
		if _, ok := p.(*ast.RestPat); ok {
			rest = i
		}
	}
	out := make([]*dpat, len(tys))
	for i, t := range tys {
		out[i] = wildOf(t)
	}
	if rest < 0 {
		if len(pats) != len(tys) {
			cx.bail()
		}
		for i, p := range pats {
			out[i] = cx.lower(p, tys[i])
		}
		return out
	}
	after := len(pats) - rest - 1
	if rest+after > len(tys) {
		cx.bail()
	}
	for i := 0; i < rest; i++ {
		out[i] = cx.lower(pats[i], tys[i])
	}
	for i := 0; i < after; i++ {
		fi := len(tys) - after + i
		out[fi] = cx.lower(pats[rest+1+i], tys[fi])
	}
	return out
}

func (cx *patCx) fieldTys(ty Ty, v *VariantDef) []Ty {
	cx.t.AdtFields(ty.Def)
	out := make([]Ty, len(v.Fields))
	for i, f := range v.Fields {
		ft := cx.t.Normalize(Subst(f.Ty, ty.Args))
		if ft.HasOpaque() {
			ft = cx.t.RevealOpaque(ft)
		}
		out[i] = ft
	}
	return out
}

func (cx *patCx) lowerPathValue(vr *ValueRes, ty Ty) *dpat {
	switch vr.Kind {
	case VUnitCtor:
		if ty.Kind != TAdt || vr.Def == nil {
			cx.bail()
		}
		v := variantOf(vr.Def)
		return cx.adtPat(ty, v, nil)
	case VConst:
		return cx.constPat(vr.Def, vr.Args, ty)
	}
	cx.bail()
	return nil
}

func (cx *patCx) constPat(d *Def, args []Ty, ty Ty) *dpat {
	switch ty.Kind {
	case TInt, TChar, TBool:
		n, ok := cx.t.ScalarConst(d, args)
		if !ok {
			cx.bail()
		}
		if ty.Kind == TBool {
			return &dpat{c: ctor{kind: ckBool, b: n.Sign() != 0}, ty: ty}
		}
		return &dpat{c: ctor{kind: ckRange, lo: n, hi: n}, ty: ty}
	case TFloat:
		return &dpat{c: ctor{kind: ckOpaque, key: fmt.Sprintf("const%d", d.ID)}, ty: ty}
	case TRef:
		if ty.Elem.Kind == TStr {
			return &dpat{c: ctor{kind: ckOpaque, key: fmt.Sprintf("const%d", d.ID)}, ty: ty}
		}
	}
	cx.bail()
	return nil
}

func (cx *patCx) lowerLit(e ast.Expr, ty Ty) *dpat {
	switch l := e.(type) {
	case *ast.ParenExpr:
		return cx.lowerLit(l.X, ty)
	case *ast.PathExpr:
		vr := cx.r.Paths[l.ID]
		if vr == nil || vr.Kind != VConst {
			cx.bail()
		}
		return cx.constPat(vr.Def, vr.Args, ty)
	case *ast.LitExpr:
		switch l.Kind {
		case ast.LitBool:
			if ty.Kind != TBool {
				cx.bail()
			}
			return &dpat{c: ctor{kind: ckBool, b: l.Value == "true"}, ty: ty}
		case ast.LitStr, ast.LitByteStr, ast.LitCStr, ast.LitFloat:
			return &dpat{c: ctor{kind: ckOpaque, key: strconv.Itoa(int(l.Kind)) + ":" + l.Value}, ty: ty}
		}
	case *ast.UnaryExpr:
		if l.Op == ast.UnNeg {
			if inner, ok := l.X.(*ast.LitExpr); ok && inner.Kind == ast.LitFloat {
				return &dpat{c: ctor{kind: ckOpaque, key: "-" + inner.Value}, ty: ty}
			}
		}
	}
	if ty.Kind != TInt && ty.Kind != TChar {
		cx.bail()
	}
	n := cx.scalarOf(e, ty)
	return &dpat{c: ctor{kind: ckRange, lo: n, hi: n}, ty: ty}
}

// scalarOf evaluates an integer/char literal or constant path of a pattern.
func (cx *patCx) scalarOf(e ast.Expr, ty Ty) *big.Int {
	switch l := e.(type) {
	case *ast.ParenExpr:
		return cx.scalarOf(l.X, ty)
	case *ast.LitExpr:
		switch l.Kind {
		case ast.LitInt, ast.LitChar, ast.LitByte:
			if l.Big != "" {
				n, ok := new(big.Int).SetString(l.Big, 10)
				if !ok {
					cx.bail()
				}
				return n
			}
			return new(big.Int).SetUint64(l.IntVal)
		}
	case *ast.UnaryExpr:
		if l.Op == ast.UnNeg {
			return new(big.Int).Neg(cx.scalarOf(l.X, ty))
		}
	case *ast.PathExpr:
		vr := cx.r.Paths[l.ID]
		if vr != nil && vr.Kind == VConst {
			if n, ok := cx.t.ScalarConst(vr.Def, vr.Args); ok {
				return n
			}
		}
	}
	cx.bail()
	return nil
}

func tyMin(t Ty) *big.Int {
	if t.Kind == TInt && t.IntKind().Signed() {
		return new(big.Int).Neg(new(big.Int).Lsh(big.NewInt(1), uint(t.IntKind().Bits()-1)))
	}
	return big.NewInt(0)
}

func tyMax(t Ty) *big.Int {
	if t.Kind == TChar {
		return big.NewInt(0x10FFFF)
	}
	bits := t.IntKind().Bits()
	if t.IntKind().Signed() {
		bits--
	}
	return new(big.Int).Sub(new(big.Int).Lsh(big.NewInt(1), uint(bits)), big.NewInt(1))
}

// ---------------------------------------------------------------------------
// Usefulness

// isUninhabited reports whether t has no values (used to allow omitting
// enum variants whose fields are uninhabited, like rustc's
// min_exhaustive_patterns).
func (cx *patCx) isUninhabited(t Ty, depth int) bool {
	if depth > 8 || t == nil {
		return false
	}
	switch t.Kind {
	case TNever:
		return true
	case TTuple:
		for _, e := range t.Elems {
			if cx.isUninhabited(e, depth+1) {
				return true
			}
		}
	case TArray:
		return t.Len != nil && t.Len.Kind == TConst && t.Len.Val > 0 && cx.isUninhabited(t.Elem, depth+1)
	case TAdt:
		if t.Def.Adt == nil {
			return false
		}
		switch t.Def.Adt.Kind {
		case AdtEnum:
			for _, v := range t.Def.Adt.Variants {
				if !cx.variantUninhabited(t, v, depth+1) {
					return false
				}
			}
			return true
		case AdtStruct:
			if isBoxTy(t) {
				return false
			}
			return cx.variantUninhabited(t, t.Def.Adt.Variants[0], depth+1)
		}
	}
	return false
}

func (cx *patCx) variantUninhabited(t Ty, v *VariantDef, depth int) bool {
	for _, ft := range cx.fieldTys(t, v) {
		if cx.isUninhabited(ft, depth) {
			return true
		}
	}
	return false
}

// ctorFieldTys returns the field types of constructor c of type ty.
func (cx *patCx) ctorFieldTys(c ctor, ty Ty) []Ty {
	switch c.kind {
	case ckSingle:
		switch ty.Kind {
		case TTuple:
			return ty.Elems
		case TRef:
			return []Ty{ty.Elem}
		case TAdt:
			if isBoxTy(ty) {
				return []Ty{ty.Args[0]}
			}
			cx.t.AdtFields(ty.Def)
			return cx.fieldTys(ty, ty.Def.Adt.Variants[0])
		}
		cx.bail()
	case ckVariant:
		cx.t.AdtFields(ty.Def)
		return cx.fieldTys(ty, ty.Def.Adt.Variants[c.idx])
	case ckSlice:
		n := c.arityOfSlice()
		out := make([]Ty, n)
		for i := range out {
			out[i] = ty.Elem
		}
		return out
	}
	return nil
}

func covers(h, c ctor) bool {
	switch h.kind {
	case ckSingle:
		return c.kind == ckSingle
	case ckVariant:
		return c.kind == ckVariant && c.idx == h.idx
	case ckBool:
		return c.kind == ckBool && c.b == h.b
	case ckRange:
		return c.kind == ckRange && c.lo.Cmp(h.lo) >= 0 && c.hi.Cmp(h.hi) <= 0
	case ckSlice:
		if c.kind != ckSlice {
			return false
		}
		if h.fixed {
			return c.fixed && c.prefix == h.prefix
		}
		if c.fixed {
			return c.prefix >= h.prefix+h.suffix
		}
		return true
	case ckOpaque:
		return c.kind == ckOpaque && c.key == h.key
	}
	return false
}

// specializeFields returns the fields of head pattern h (which covers c)
// as c's fields.
func specializeFields(h *dpat, c ctor, ftys []Ty) []*dpat {
	if h.c.kind == ckWild {
		out := make([]*dpat, len(ftys))
		for i, ft := range ftys {
			out[i] = wildOf(ft)
		}
		return out
	}
	if h.c.kind == ckSlice && !h.c.fixed {
		n := c.arityOfSlice()
		out := append([]*dpat{}, h.fields[:h.c.prefix]...)
		for i := h.c.prefix + h.c.suffix; i < n; i++ {
			out = append(out, wildOf(ftys[len(out)]))
		}
		return append(out, h.fields[h.c.prefix:]...)
	}
	return h.fields
}

// expandOr replaces rows whose head is an or-pattern by one row per
// alternative.
func expandOr(rows [][]*dpat) [][]*dpat {
	has := false
	for _, r := range rows {
		if r[0].c.kind == ckOr {
			has = true
			break
		}
	}
	if !has {
		return rows
	}
	var out [][]*dpat
	var add func(r []*dpat)
	add = func(r []*dpat) {
		if r[0].c.kind != ckOr {
			out = append(out, r)
			return
		}
		for _, a := range r[0].alts {
			nr := append([]*dpat{a}, r[1:]...)
			add(nr)
		}
	}
	for _, r := range rows {
		add(r)
	}
	return out
}

// splitCtors returns the constructors of ty, split so that every head
// constructor either covers a split constructor or is disjoint from it.
// infinite reports types whose values cannot be enumerated (only a
// wildcard covers them).
func (cx *patCx) splitCtors(ty Ty, heads []ctor) (split []ctor, infinite bool) {
	opaque := false
	for _, h := range heads {
		if h.kind == ckOpaque {
			opaque = true
		}
	}
	if opaque {
		seen := map[string]bool{}
		for _, h := range heads {
			if h.kind != ckOpaque {
				cx.bail()
			}
			if !seen[h.key] {
				seen[h.key] = true
				split = append(split, h)
			}
		}
		return split, true
	}
	check := func(k ctorKind) {
		for _, h := range heads {
			if h.kind != k {
				cx.bail()
			}
		}
	}
	switch ty.Kind {
	case TBool:
		check(ckBool)
		return []ctor{{kind: ckBool, b: false}, {kind: ckBool, b: true}}, false
	case TInt, TChar:
		check(ckRange)
		var full [][2]*big.Int
		if ty.Kind == TChar {
			full = [][2]*big.Int{{big.NewInt(0), big.NewInt(0xD7FF)}, {big.NewInt(0xE000), big.NewInt(0x10FFFF)}}
		} else {
			full = [][2]*big.Int{{tyMin(ty), tyMax(ty)}}
		}
		// boundaries: every head range starts a segment at lo and ends
		// one at hi (a new segment starts at hi+1)
		var points []*big.Int
		for _, h := range heads {
			points = append(points, h.lo, new(big.Int).Add(h.hi, big.NewInt(1)))
		}
		sort.Slice(points, func(i, j int) bool { return points[i].Cmp(points[j]) < 0 })
		for _, fr := range full {
			start := fr[0]
			for _, p := range points {
				if p.Cmp(start) <= 0 || p.Cmp(fr[1]) > 0 {
					continue
				}
				split = append(split, ctor{kind: ckRange, lo: start, hi: new(big.Int).Sub(p, big.NewInt(1))})
				start = p
			}
			split = append(split, ctor{kind: ckRange, lo: start, hi: fr[1]})
		}
		return split, false
	case TAdt:
		if ty.Def.Adt == nil {
			cx.bail()
		}
		if ty.Def.Adt.Kind == AdtEnum {
			check(ckVariant)
			cx.t.AdtFields(ty.Def)
			for i := range ty.Def.Adt.Variants {
				split = append(split, ctor{kind: ckVariant, idx: i})
			}
			return split, false
		}
		if ty.Def.Adt.Kind == AdtUnion {
			if len(heads) > 0 {
				cx.bail()
			}
			return nil, true
		}
		check(ckSingle)
		return []ctor{{kind: ckSingle}}, false
	case TTuple, TRef:
		check(ckSingle)
		return []ctor{{kind: ckSingle}}, false
	case TArray:
		check(ckSlice)
		if ty.Len == nil || ty.Len.Kind != TConst || ty.Len.Val > 4096 {
			if len(heads) > 0 {
				cx.bail()
			}
			return nil, true
		}
		return []ctor{{kind: ckSlice, fixed: true, prefix: int(ty.Len.Val)}}, false
	case TSlice:
		check(ckSlice)
		maxFixed, maxPrefix, maxSuffix := -1, 0, 0
		hasVar := false
		for _, h := range heads {
			if h.fixed {
				if h.prefix > maxFixed {
					maxFixed = h.prefix
				}
			} else {
				hasVar = true
				if h.prefix > maxPrefix {
					maxPrefix = h.prefix
				}
				if h.suffix > maxSuffix {
					maxSuffix = h.suffix
				}
			}
		}
		if !hasVar {
			// lengths not mentioned are never covered
			seen := map[int]bool{}
			for _, h := range heads {
				if !seen[h.prefix] {
					seen[h.prefix] = true
					split = append(split, h)
				}
			}
			return split, true
		}
		limit := maxFixed + 1
		if maxPrefix+maxSuffix > limit {
			limit = maxPrefix + maxSuffix
		}
		for n := 0; n < limit; n++ {
			split = append(split, ctor{kind: ckSlice, fixed: true, prefix: n})
		}
		split = append(split, ctor{kind: ckSlice, prefix: maxPrefix, suffix: limit - maxPrefix})
		return split, false
	case TNever:
		if len(heads) > 0 {
			cx.bail()
		}
		return nil, false
	}
	if len(heads) > 0 {
		cx.bail()
	}
	return nil, true
}

// missing returns the value vectors (witnesses) of types tys not matched
// by any row.
func (cx *patCx) missing(rows [][]*dpat, tys []Ty) [][]*dpat {
	cx.budget++
	if cx.budget > 200000 {
		cx.bail()
	}
	if len(tys) == 0 {
		if len(rows) == 0 {
			return [][]*dpat{{}}
		}
		return nil
	}
	rows = expandOr(rows)
	ty := tys[0]
	var heads []ctor
	for _, r := range rows {
		if r[0].c.kind != ckWild {
			heads = append(heads, r[0].c)
		}
	}
	split, infinite := cx.splitCtors(ty, heads)
	var present, missingC []ctor
	for _, sc := range split {
		found := false
		for _, h := range heads {
			if covers(h, sc) {
				found = true
				break
			}
		}
		if found {
			present = append(present, sc)
		} else if !(sc.kind == ckVariant && cx.variantUninhabited(ty, ty.Def.Adt.Variants[sc.idx], 0)) {
			missingC = append(missingC, sc)
		}
	}
	if infinite || len(missingC) > 0 {
		var def [][]*dpat
		for _, r := range rows {
			if r[0].c.kind == ckWild {
				def = append(def, r[1:])
			}
		}
		ws := cx.missing(def, tys[1:])
		if len(ws) == 0 {
			return nil
		}
		var heads []*dpat
		if infinite || len(present) == 0 {
			heads = []*dpat{wildOf(ty)}
		} else {
			for _, mc := range mergeRanges(missingC) {
				fs := cx.ctorFieldTys(mc, ty)
				h := &dpat{c: mc, ty: ty}
				for _, ft := range fs {
					h.fields = append(h.fields, wildOf(ft))
				}
				heads = append(heads, h)
			}
		}
		var out [][]*dpat
		for _, h := range heads {
			for _, w := range ws {
				out = append(out, append([]*dpat{h}, w...))
				if len(out) >= 16 {
					return out
				}
			}
		}
		return out
	}
	var out [][]*dpat
	for _, sc := range split {
		ftys := cx.ctorFieldTys(sc, ty)
		var spec [][]*dpat
		for _, r := range rows {
			if r[0].c.kind == ckWild || covers(r[0].c, sc) {
				nr := append(append([]*dpat{}, specializeFields(r[0], sc, ftys)...), r[1:]...)
				spec = append(spec, nr)
			}
		}
		ws := cx.missing(spec, append(append([]Ty{}, ftys...), tys[1:]...))
		for _, w := range ws {
			h := &dpat{c: sc, fields: w[:len(ftys)], ty: ty}
			out = append(out, append([]*dpat{h}, w[len(ftys):]...))
		}
		if len(out) >= 16 {
			break
		}
	}
	return out
}

// mergeRanges merges adjacent missing integer ranges (for reporting).
func mergeRanges(cs []ctor) []ctor {
	var out []ctor
	for _, c := range cs {
		if n := len(out); n > 0 && c.kind == ckRange && out[n-1].kind == ckRange &&
			new(big.Int).Add(out[n-1].hi, big.NewInt(1)).Cmp(c.lo) == 0 {
			out[n-1].hi = c.hi
			continue
		}
		out = append(out, c)
	}
	return out
}

// ---------------------------------------------------------------------------
// Witness printing

func (cx *patCx) witnessList(ws []*dpat) string {
	var strs []string
	seen := map[string]bool{}
	for _, w := range ws {
		s := "`" + cx.patString(w) + "`"
		if !seen[s] {
			seen[s] = true
			strs = append(strs, s)
		}
	}
	switch len(strs) {
	case 1:
		return strs[0]
	case 2:
		return strs[0] + " and " + strs[1]
	case 3:
		return strs[0] + ", " + strs[1] + " and " + strs[2]
	}
	return fmt.Sprintf("%s, %s, %s and %d more", strs[0], strs[1], strs[2], len(strs)-3)
}

func (cx *patCx) patString(p *dpat) string {
	ty := p.ty
	switch p.c.kind {
	case ckWild, ckOpaque:
		return "_"
	case ckBool:
		return strconv.FormatBool(p.c.b)
	case ckRange:
		lo, hi := cx.scalarString(p.c.lo, ty), cx.scalarString(p.c.hi, ty)
		if p.c.lo.Cmp(p.c.hi) == 0 {
			return lo
		}
		return lo + "..=" + hi
	case ckSlice:
		var parts []string
		for i, f := range p.fields {
			if !p.c.fixed && i == p.c.prefix {
				parts = append(parts, "..")
			}
			parts = append(parts, cx.patString(f))
		}
		if !p.c.fixed && p.c.prefix == len(p.fields) {
			parts = append(parts, "..")
		}
		return "[" + strings.Join(parts, ", ") + "]"
	case ckSingle:
		switch ty.Kind {
		case TRef:
			return "&" + cx.patString(p.fields[0])
		case TTuple:
			var parts []string
			for _, f := range p.fields {
				parts = append(parts, cx.patString(f))
			}
			if len(parts) == 1 {
				return "(" + parts[0] + ",)"
			}
			return "(" + strings.Join(parts, ", ") + ")"
		case TAdt:
			if isBoxTy(ty) {
				return "box " + cx.patString(p.fields[0])
			}
			return cx.variantString(ty, ty.Def.Adt.Variants[0], ty.Def.Name, p.fields)
		}
		return "_"
	case ckVariant:
		v := ty.Def.Adt.Variants[p.c.idx]
		name := ty.Def.Name + "::" + v.Name
		if ty.Def.Crate != nil && ty.Def.Crate.Name == "std" && (ty.Def.LangItem == "Option" || ty.Def.LangItem == "Result") {
			name = v.Name
		}
		return cx.variantString(ty, v, name, p.fields)
	}
	return "_"
}

func (cx *patCx) variantString(ty Ty, v *VariantDef, name string, fields []*dpat) string {
	switch v.Shape {
	case ast.ShapeUnit:
		return name
	case ast.ShapeTuple:
		var parts []string
		for _, f := range fields {
			parts = append(parts, cx.patString(f))
		}
		return name + "(" + strings.Join(parts, ", ") + ")"
	}
	var parts []string
	for i, f := range fields {
		if f.c.kind == ckWild {
			continue
		}
		parts = append(parts, v.Fields[i].Name+": "+cx.patString(f))
	}
	if len(parts) < len(fields) {
		parts = append(parts, "..")
	}
	return name + " { " + strings.Join(parts, ", ") + " }"
}

func (cx *patCx) scalarString(n *big.Int, ty Ty) string {
	if ty.Kind == TChar {
		r := rune(n.Int64())
		if r >= 0x20 && r < 0x7f && r != '\'' && r != '\\' {
			return "'" + string(r) + "'"
		}
		return fmt.Sprintf("'\\u{%x}'", n.Int64())
	}
	name := ty.IntKind().String()
	if n.Cmp(tyMax(ty)) == 0 {
		return name + "::MAX"
	}
	if ty.IntKind().Signed() && n.Cmp(tyMin(ty)) == 0 {
		return name + "::MIN"
	}
	return n.String() + "_" + name
}

// postCheck runs the checks done on a type-checked body (rustc's MIR build
// and THIR checks): pattern exhaustiveness, unsafety.
func (t *Tcx) postCheck(r *Results, def *Def) {
	t.checkPatterns(r, def)
	t.checkUnsafety(r, def)
	t.checkBodyMisc(r, def)
	t.checkConstContext(r, def)
}

func plural(n int) string {
	if n == 1 {
		return ""
	}
	return "s"
}
