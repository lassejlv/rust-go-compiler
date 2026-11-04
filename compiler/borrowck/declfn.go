package borrowck

import (
	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

// declaredFnSig returns the signature of the Fn* trait object or `impl
// Fn*` type the callee place (a parameter of the function, through
// references, boxes and struct fields) is declared with, and the type
// parameters in scope of that declaration.
func (b *builder) declaredFnSig(p Place) (inputs []ast.Type, output ast.Type, hr map[string]bool, params map[string]bool, ok bool) {
	if b.closure != nil || p.Local <= 0 || int(p.Local) >= len(b.body.Locals) {
		return
	}
	ld := b.body.Locals[p.Local]
	d := b.r.Inst.Def
	pi := int(p.Local) - 1 // locals 1.. are the parameters (buildFn)
	if ld.Kind != LArg || d == nil || d.Fn == nil || d.Fn.Ast == nil || d.Fn.Ast.Decl == nil || pi >= len(b.r.Params) || ld.User != b.r.Params[pi] {
		return
	}
	decl := sigDecl(d)
	var selfAst ast.Type
	if par := d.Parent; par != nil && par.Item != nil {
		if im, isImpl := par.Item.Kind.(*ast.ImplItem); isImpl {
			selfAst = im.SelfTy
		}
	}
	selfPath := &ast.PathType{Path: &ast.Path{Segments: []*ast.PathSegment{{Ident: ast.Ident{Name: "Self"}}}}}
	var t ast.Type
	if sp := decl.SelfParam; sp != nil {
		if pi == 0 {
			switch sp.Kind {
			case ast.SelfValue:
				t = selfPath
			case ast.SelfRef, ast.SelfRefMut:
				t = &ast.RefType{Lifetime: sp.Lifetime, Mut: sp.Kind == ast.SelfRefMut, Elem: selfPath}
			default:
				t = sp.Ty
			}
		}
		pi--
	}
	if t == nil {
		if pi < 0 || pi >= len(decl.Params) {
			return
		}
		t = decl.Params[pi].Ty
	}
	params = map[string]bool{}
	for _, g := range astGenerics(d) {
		genericNames(g, params)
	}
	peel := func(t ast.Type) ast.Type {
		for n := 0; n < 8; n++ {
			switch x := t.(type) {
			case *ast.ParenType:
				t = x.Elem
				continue
			case *ast.PathType:
				if selfAst != nil && x.QSelf == nil && x.Path != nil && len(x.Path.Segments) == 1 && x.Path.Segments[0].Ident.Name == "Self" && x.Path.Segments[0].Args == nil {
					t = selfAst
					continue
				}
			}
			break
		}
		return t
	}
	// the single type argument of `Box<X>`
	boxArg := func(t ast.Type) ast.Type {
		pt, isPath := t.(*ast.PathType)
		if !isPath || pt.QSelf != nil || pt.Path == nil || len(pt.Path.Segments) == 0 {
			return nil
		}
		seg := pt.Path.Segments[len(pt.Path.Segments)-1]
		if seg.Ident.Name != "Box" || seg.Args == nil || seg.Args.Parenthesized || len(seg.Args.Args) != 1 || seg.Args.Args[0].Type == nil {
			return nil
		}
		return seg.Args.Args[0].Type
	}
	ty := ld.Ty
	for _, pr := range p.Proj {
		t = peel(t)
		if t == nil || ty == nil {
			return
		}
		switch pr.Kind {
		case PDeref:
			switch {
			case pr.Deref == DBox && ty.Kind == TAdt && len(ty.Args) > 0:
				if t = boxArg(t); t == nil {
					return
				}
				ty = ty.Args[0]
			case (pr.Deref == DRef || pr.Deref == DRefMut) && ty.Kind == TRef:
				r, isRef := t.(*ast.RefType)
				if !isRef {
					return
				}
				t, ty = r.Elem, ty.Elem
			default:
				return
			}
		case PField:
			switch ty.Kind {
			case TTuple:
				tt, isTuple := t.(*ast.TupleType)
				if !isTuple || pr.Field >= len(tt.Elems) || pr.Field >= len(ty.Elems) {
					return
				}
				t, ty = tt.Elems[pr.Field], ty.Elems[pr.Field]
			case TAdt:
				ad := ty.Def
				b.tcx.AdtFields(ad)
				v := 0
				if pr.Variant > 0 {
					v = pr.Variant - 1
				}
				if ad.Adt == nil || v >= len(ad.Adt.Variants) || pr.Field >= len(ad.Adt.Variants[v].Fields) {
					return
				}
				f := ad.Adt.Variants[v].Fields[pr.Field]
				if f.Ast == nil || f.Ty == nil || ad.Item == nil {
					return
				}
				// the field's declaration, in terms of the ADT's generics
				t = f.Ast.Ty
				ty = b.tcx.Normalize(Subst(f.Ty, ty.Args))
				selfAst = nil
				params = map[string]bool{}
				switch k := ad.Item.Kind.(type) {
				case *ast.StructItem:
					genericNames(k.Generics, params)
				case *ast.EnumItem:
					genericNames(k.Generics, params)
				default:
					return
				}
			default:
				return
			}
		default:
			return
		}
	}
	// the callee itself may be the reference or box holding the closure
	for n := 0; n < 4; n++ {
		t = peel(t)
		if r, isRef := t.(*ast.RefType); isRef {
			t = r.Elem
		} else if a := boxArg(t); a != nil {
			t = a
		} else {
			break
		}
	}
	var bounds []*ast.Bound
	switch x := t.(type) {
	case *ast.DynTraitType:
		bounds = x.Bounds
	case *ast.ImplTraitType:
		bounds = x.Bounds
	default:
		return
	}
	n := 0
	for _, bd := range bounds {
		if bd.Path == nil || len(bd.Path.Segments) == 0 {
			continue
		}
		seg := bd.Path.Segments[len(bd.Path.Segments)-1]
		switch seg.Ident.Name {
		case "Fn", "FnMut", "FnOnce":
		default:
			continue
		}
		n++
		if seg.Args == nil || !seg.Args.Parenthesized {
			return
		}
		inputs, output = seg.Args.Inputs, seg.Args.Output
		hr = map[string]bool{}
		for _, l := range bd.ForLifetimes {
			hr[l] = true
		}
	}
	if n != 1 {
		return nil, nil, nil, nil, false
	}
	return inputs, output, hr, params, true
}
