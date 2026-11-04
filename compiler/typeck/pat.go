package typeck

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
	"strings"
)

type bindingMode int

const (
	bmByValue bindingMode = iota
	bmByRef
	bmByRefMut
)

// orBindings, when non-nil, makes bindings with the same name reuse the
// same local (alternatives of an or-pattern).
type orCtx struct {
	locals map[string]*Local
	first  bool
}

func (fcx *FnCtxt) checkPat(p ast.Pat, expected Ty, bm bindingMode, top bool) {
	fcx.checkPatInner(p, expected, bm, nil)
}

// isNonRefPat reports whether match ergonomics should peel references
// before matching p.
func isNonRefPat(p ast.Pat) bool {
	switch x := p.(type) {
	case *ast.TuplePat, *ast.TupleStructPat, *ast.StructPat, *ast.SlicePat, *ast.RangePat, *ast.PathPat:
		return true
	case *ast.LitPat:
		if l, ok := x.Expr.(*ast.LitExpr); ok && (l.Kind == ast.LitStr || l.Kind == ast.LitByteStr) {
			return false
		}
		return true
	}
	return false
}

func (fcx *FnCtxt) checkPatInner(p ast.Pat, expected Ty, bm bindingMode, oc *orCtx) {
	// Identifiers that resolve to unit structs/variants/consts act as paths.
	if ip, ok := p.(*ast.IdentPat); ok && !ip.ByRef && !ip.Mut && ip.Sub == nil {
		if vr, ty, ok := fcx.identPatAsPath(ip); ok {
			isRefConst := vr.Kind == VConst && fcx.shallow(ty).Kind == TRef
			expected = fcx.peelForPat(p, expected, &bm, isRefConst)
			fcx.r.Paths[ip.ID] = vr
			fcx.record(p.GetID(), expected)
			fcx.demandEq(ip.Span, expected, ty)
			return
		}
	}
	if pp, ok := p.(*ast.PathPat); ok {
		// a const of reference type (`record::NAME: &str`) matches the
		// reference itself: don't peel it like other non-ref patterns
		vr, ty := fcx.resolveValuePath(pp.QSelf, pp.Path, pp.Span, true, nil)
		if vr == nil {
			return
		}
		isRefConst := vr.Kind == VConst && fcx.shallow(ty).Kind == TRef
		expected = fcx.peelForPat(p, expected, &bm, isRefConst)
		fcx.record(p.GetID(), expected)
		fcx.r.Paths[pp.ID] = vr
		if vr.Kind == VFn {
			fcx.errorf(pp.Span, "expected unit struct, unit variant or constant, found tuple variant")
			return
		}
		fcx.demandEq(pp.Span, expected, ty)
		return
	}
	if isNonRefPat(p) {
		expected = fcx.peelForPat(p, expected, &bm, false)
	} else if lp, ok := p.(*ast.LitPat); ok {
		// string literal patterns: peel down to &str
		_ = lp
		for {
			et := fcx.structurally(expected)
			if et.Kind == TRef && fcx.structurally(et.Elem).Kind == TRef {
				fcx.r.PatDerefs[p.GetID()] = append(fcx.r.PatDerefs[p.GetID()], et)
				expected = et.Elem
				continue
			}
			break
		}
	}
	fcx.record(p.GetID(), expected)
	switch x := p.(type) {
	case *ast.WildPat, *ast.RestPat:
	case *ast.ParenPat:
		fcx.checkPatInner(x.Pat, expected, bm, oc)
	case *ast.IdentPat:
		if x.Sub == nil && x.Name.Hyg == 0 && fcx.def != nil && fcx.def.Crate != nil && fcx.def.Crate.Local {
			// a binding named like a unit variant of the matched enum
			// (deny-by-default lint bindings_with_variant_name)
			if et := fcx.shallow(expected); et.Kind == TAdt && et.Def.Adt != nil && et.Def.Adt.Kind == AdtEnum {
				for _, v := range et.Def.Adt.Variants {
					if v.Name == x.Name.Name && v.Shape == ast.ShapeUnit {
						fcx.t.ErrorCode(x.Span, "E0170", "pattern binding `%s` is named the same as one of the variants of the type `%s`", v.Name, et.Def.Name)
					}
				}
			}
		}
		mode := bm
		if x.ByRef {
			if x.Mut {
				mode = bmByRefMut
			} else {
				mode = bmByRef
			}
		} else if x.Mut {
			mode = bmByValue
		}
		var lty Ty
		switch mode {
		case bmByValue:
			lty = expected
		case bmByRef:
			lty = Ref(expected, false)
		case bmByRefMut:
			lty = Ref(expected, true)
		}
		var l *Local
		if oc != nil && !oc.first {
			l = oc.locals[hygKey(x.Name.Name, x.Name.Hyg)]
			if l != nil {
				fcx.demandEq(x.Span, l.Ty, lty)
				fcx.bindLocal(x.Name.Name, x.Name.Hyg, l)
			}
		}
		if l == nil {
			l = fcx.newLocal(x.Name.Name, x.Name.Hyg, lty, x.Mut && mode == bmByValue, x.Name.Span, x.ID)
			if oc != nil {
				oc.locals[hygKey(x.Name.Name, x.Name.Hyg)] = l
			}
		}
		l.ByRef = mode != bmByValue
		l.RefMut = mode == bmByRefMut
		fcx.r.Locals[x.ID] = l
		if x.Sub != nil {
			fcx.checkPatInner(x.Sub, expected, bm, oc)
		}
	case *ast.LitPat:
		lt := fcx.checkExpr(x.Expr, expected)
		if l, ok := x.Expr.(*ast.LitExpr); ok && l.Kind == ast.LitByteStr {
			// b"..." patterns also match `&[u8]` scrutinees (unsize)
			if et := fcx.shallow(expected); et.Kind == TRef && fcx.shallow(et.Elem).Kind == TSlice {
				if fcx.coerce(x.Expr, lt, expected) {
					break
				}
			}
		}
		fcx.demandEq(x.Span, expected, lt)
	case *ast.RangePat:
		if x.Lo != nil {
			fcx.demandEq(x.Span, expected, fcx.checkExpr(x.Lo, expected))
		}
		if x.Hi != nil {
			fcx.demandEq(x.Span, expected, fcx.checkExpr(x.Hi, expected))
		}
	case *ast.TuplePat:
		et := fcx.structurally(expected)
		n := countNonRest(x.Elems)
		var elems []Ty
		if et.Kind == TTuple {
			elems = et.Elems
		} else {
			if hasRest(x.Elems) {
				fcx.errorf(x.Span, "type annotations needed for tuple pattern with `..`")
				return
			}
			elems = make([]Ty, n)
			for i := range elems {
				elems[i] = fcx.ic.newVar()
			}
			fcx.demandEq(x.Span, expected, Tuple(elems))
		}
		fcx.checkSeqPats(x.Elems, elems, bm, oc, x.Span)
	case *ast.TupleStructPat:
		v, adt := fcx.resolvePatCtor(x.QSelf, x.Path, x.Span, expected)
		if v == nil {
			return
		}
		fcx.r.Variants[x.ID] = v
		fcx.demandEq(x.Span, expected, adt)
		fts := make([]Ty, len(v.Fields))
		for i, f := range v.Fields {
			fts[i] = fcx.normalizeInBody(Subst(f.Ty, adt.Args), x.Span)
		}
		fcx.checkSeqPats(x.Elems, fts, bm, oc, x.Span)
	case *ast.StructPat:
		v, adt := fcx.resolveStructPath(x.QSelf, x.Path, x.Span)
		if v == nil {
			return
		}
		fcx.r.Variants[x.ID] = v
		fcx.demandEq(x.Span, expected, adt)
		mentioned := map[string]bool{}
		defer func() {
			if x.Rest || isUnion(fcx.shallow(adt)) {
				return
			}
			var miss []string
			for _, f := range v.Fields {
				if !mentioned[f.Name] {
					miss = append(miss, "`"+f.Name+"`")
				}
			}
			if len(miss) == 1 {
				fcx.t.ErrorCode(x.Span, "E0027", "pattern does not mention field %s", miss[0])
			} else if len(miss) > 1 {
				fcx.t.ErrorCode(x.Span, "E0027", "pattern does not mention fields %s", strings.Join(miss, ", "))
			}
		}()
		for _, f := range x.Fields {
			fi := v.FieldIndex(f.Name.Name)
			if fi < 0 {
				fcx.errorf(f.Span, "variant `%s` does not have a field named `%s`", v.Name, f.Name.Name)
				continue
			}
			fcx.checkFieldPrivacy(f.Name, adt, v.Fields[fi], v)
			if mentioned[f.Name.Name] {
				fcx.t.ErrorCode(f.Name.Span, "E0025", "field `%s` bound multiple times in the pattern", f.Name.Name)
			}
			mentioned[f.Name.Name] = true
			ft := fcx.normalizeInBody(Subst(v.Fields[fi].Ty, adt.Args), f.Span)
			fcx.checkPatInner(f.Pat, ft, bm, oc)
		}
	case *ast.PathPat:
		vr, ty := fcx.resolveValuePath(x.QSelf, x.Path, x.Span, true, expected)
		if vr == nil {
			return
		}
		fcx.r.Paths[x.ID] = vr
		if vr.Kind == VFn {
			fcx.errorf(x.Span, "expected unit struct, unit variant or constant, found tuple variant")
			return
		}
		fcx.demandEq(x.Span, expected, ty)
	case *ast.RefPat:
		et := fcx.structurally(expected)
		if et.Kind != TRef {
			if et.Kind == TInfer {
				inner := fcx.ic.newVar()
				fcx.demandEq(x.Span, expected, Ref(inner, x.Mut))
				fcx.checkPatInner(x.Pat, inner, bmByValue, oc)
				return
			}
			fcx.errorf(x.Span, "mismatched types: expected `%s`, found `&_`", fcx.resolve(expected))
			return
		}
		fcx.checkPatInner(x.Pat, et.Elem, bmByValue, oc)
	case *ast.BoxPat:
		et := fcx.structurally(expected)
		if et.Kind == TAdt && et.Def.LangItem == "owned_box" {
			fcx.checkPatInner(x.Pat, et.Args[0], bm, oc)
			return
		}
		fcx.errorf(x.Span, "box pattern on non-Box type")
	case *ast.SlicePat:
		et := fcx.structurally(expected)
		var elem Ty
		isArray := false
		switch et.Kind {
		case TArray:
			elem = et.Elem
			isArray = true
		case TSlice:
			elem = et.Elem
		default:
			fcx.errorf(x.Span, "expected an array or slice, found `%s`", fcx.resolve(expected))
			return
		}
		n := countNonRest(x.Elems)
		if isArray && et.Len != nil && et.Len.Kind == TConst {
			if n == len(x.Elems) && uint64(n) != et.Len.Val {
				fcx.t.ErrorCode(x.Span, "E0527", "pattern requires %d element%s but array has %d", n, plural(n), et.Len.Val)
			} else if n < len(x.Elems) && uint64(n) > et.Len.Val {
				fcx.t.ErrorCode(x.Span, "E0528", "pattern requires at least %d element%s but array has %d", n, plural(n), et.Len.Val)
			}
		}
		for _, e := range x.Elems {
			if isRestLike(e) {
				// `rest @ ..` binds a subslice
				if ip, ok := e.(*ast.IdentPat); ok {
					var st Ty
					if isArray && et.Len.Kind == TConst {
						st = ArrayN(elem, et.Len.Val-uint64(n))
					} else {
						st = Slice(elem)
					}
					fcx.record(e.GetID(), st)
					mode := bm
					if ip.ByRef {
						mode = bmByRef
						if ip.Mut {
							mode = bmByRefMut
						}
					}
					lty := st
					if mode == bmByRef {
						lty = Ref(st, false)
					} else if mode == bmByRefMut {
						lty = Ref(st, true)
					} else if !isArray {
						fcx.errorf(ip.Span, "cannot bind a slice by value")
					}
					var l *Local
					if oc != nil && !oc.first {
						// later or-pattern alternative: same local
						if l = oc.locals[hygKey(ip.Name.Name, ip.Name.Hyg)]; l != nil {
							fcx.demandEq(ip.Span, l.Ty, lty)
							fcx.bindLocal(ip.Name.Name, ip.Name.Hyg, l)
						}
					}
					if l == nil {
						l = fcx.newLocal(ip.Name.Name, ip.Name.Hyg, lty, ip.Mut && mode == bmByValue, ip.Name.Span, ip.ID)
						if oc != nil {
							oc.locals[hygKey(ip.Name.Name, ip.Name.Hyg)] = l
						}
						l.ByRef = mode != bmByValue
						l.RefMut = mode == bmByRefMut
					}
					fcx.r.Locals[ip.ID] = l
					if ip.Sub != nil {
						fcx.record(ip.Sub.GetID(), st)
					}
				} else {
					fcx.record(e.GetID(), Slice(elem))
				}
				continue
			}
			fcx.checkPatInner(e, elem, bm, oc)
		}
	case *ast.OrPat:
		noc := oc
		if noc == nil {
			noc = &orCtx{locals: map[string]*Local{}, first: true}
		}
		for i, alt := range x.Alts {
			if i > 0 {
				noc.first = false
			}
			fcx.checkPatInner(alt, expected, bm, noc)
		}
		if oc == nil {
			noc.first = true
		}
	case *ast.MacPat:
		fcx.errorf(x.Span, "macro in pattern position was not expanded")
	default:
		fcx.errorf(p.GetSpan(), "unsupported pattern")
	}
}

// peelForPat implements default binding modes: peel references off the
// expected type before matching a non-reference pattern.
func (fcx *FnCtxt) peelForPat(p ast.Pat, expected Ty, bm *bindingMode, isPath bool) Ty {
	for {
		et := fcx.structurally(expected)
		if et.Kind != TRef {
			return et
		}
		if isPath {
			// constants of reference type (e.g. &str consts) match directly
			return et
		}
		fcx.r.PatDerefs[p.GetID()] = append(fcx.r.PatDerefs[p.GetID()], et)
		if et.Mut {
			if *bm == bmByValue {
				*bm = bmByRefMut
			}
		} else {
			*bm = bmByRef
		}
		expected = et.Elem
	}
}

func isRestLike(p ast.Pat) bool {
	switch x := p.(type) {
	case *ast.RestPat:
		return true
	case *ast.IdentPat:
		_, ok := x.Sub.(*ast.RestPat)
		return ok
	}
	return false
}

func countNonRest(ps []ast.Pat) int {
	n := 0
	for _, p := range ps {
		if !isRestLike(p) {
			n++
		}
	}
	return n
}

func hasRest(ps []ast.Pat) bool {
	for _, p := range ps {
		if isRestLike(p) {
			return true
		}
	}
	return false
}

// checkSeqPats matches tuple-like subpatterns (with optional `..`) against
// field types.
func (fcx *FnCtxt) checkSeqPats(pats []ast.Pat, tys []Ty, bm bindingMode, oc *orCtx, sp lexer.Span) {
	rest := -1
	for i, p := range pats {
		if _, ok := p.(*ast.RestPat); ok {
			rest = i
		}
	}
	if rest < 0 {
		if len(pats) != len(tys) {
			fcx.errorf(sp, "this pattern has %d fields, but the corresponding type has %d fields", len(pats), len(tys))
			return
		}
		for i, p := range pats {
			fcx.checkPatInner(p, tys[i], bm, oc)
		}
		return
	}
	after := len(pats) - rest - 1
	if rest+after > len(tys) {
		fcx.errorf(sp, "this pattern has too many fields")
		return
	}
	for i := 0; i < rest; i++ {
		fcx.checkPatInner(pats[i], tys[i], bm, oc)
	}
	for i := 0; i < after; i++ {
		fcx.checkPatInner(pats[rest+1+i], tys[len(tys)-after+i], bm, oc)
	}
	fcx.record(pats[rest].GetID(), TyUnit)
}

// identPatAsPath checks whether a lone identifier pattern names a unit
// struct/variant or a constant.
func (fcx *FnCtxt) identPatAsPath(ip *ast.IdentPat) (*ValueRes, Ty, bool) {
	name := ip.Name.Name
	fcx.ac.scope = fcx.curMod()
	res, ok := fcx.ac.firstLookup(name, ValueNS)
	if !ok {
		res, ok = fcx.t.R.LookupLexical(fcx.curMod(), name, ValueNS)
	}
	if !ok {
		return nil, nil, false
	}
	switch res.Kind {
	case ResCtor:
		v := variantOf(res.Def)
		adt := res.Def
		if res.Def.Kind == DefVariant {
			adt = res.Def.Parent
		}
		fcx.t.AdtFields(adt)
		if v.Shape != ast.ShapeUnit {
			if ip.Name.Hyg == 0 && fcx.def != nil && fcx.def.Crate != nil && fcx.def.Crate.Local {
				what := "tuple structs"
				if res.Def.Kind == DefVariant {
					what = "tuple variants"
				}
				fcx.t.ErrorCode(ip.Span, "E0530", "bindings cannot shadow %s", what)
			}
			return nil, nil, false
		}
		args := fcx.ownArgs(adt, nil, nil, ip.Span)
		return &ValueRes{Kind: VUnitCtor, Def: res.Def, Args: args}, Adt(adt, args), true
	case ResDef:
		if res.Def.Kind == DefConst {
			return &ValueRes{Kind: VConst, Def: res.Def}, fcx.normalizeInBody(fcx.t.ConstTy(res.Def), ip.Span), true
		}
		if res.Def.Kind == DefStatic && ip.Name.Hyg == 0 && fcx.def != nil && fcx.def.Crate != nil && fcx.def.Crate.Local {
			fcx.t.ErrorCode(ip.Span, "E0530", "bindings cannot shadow statics")
		}
	}
	return nil, nil, false
}

// resolvePatCtor resolves a tuple-struct pattern path.
func (fcx *FnCtxt) resolvePatCtor(qself *ast.QSelf, p *ast.Path, sp lexer.Span, expected Ty) (*VariantDef, Ty) {
	vr, ty := fcx.resolveValuePath(qself, p, sp, true, expected)
	if vr == nil {
		return nil, nil
	}
	if vr.Kind != VFn || (vr.Def.Kind != DefStruct && vr.Def.Kind != DefVariant) {
		fcx.errorf(sp, "expected tuple struct or tuple variant, found `%s`", ast.PathString(p))
		return nil, nil
	}
	_ = ty
	adt := vr.Def
	if adt.Kind == DefVariant {
		adt = adt.Parent
	}
	return variantOf(vr.Def), Adt(adt, vr.Args)
}
