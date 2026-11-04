package borrowck

import (
	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

// Default object lifetime bounds (RFC 599/1156) of trait objects in
// signatures: `dyn Trait` without a lifetime bound inside `&'a _` is
// `dyn Trait + 'a`; inside a type argument of an ADT without lifetime
// parameters (Box, Vec, Rc, Option, ...) it is `dyn Trait + 'static`,
// when the trait itself declares no lifetimes. Only these unambiguous
// cases are modeled; ADTs with lifetime parameters (whose `T: 'a` bounds
// would decide) and traits with lifetime parameters or bounds are left
// alone.

// objectDefaultStatic reports whether trait object type t (the type of a
// `dyn` written without lifetime bound inside an ADT without lifetime
// parameters) defaults to 'static.
func objectDefaultStatic(t Ty) bool {
	if t == nil || t.Kind != TDyn || t.Def == nil || t.Def.Item == nil {
		return false
	}
	tr, ok := t.Def.Item.Kind.(*ast.TraitItem)
	if !ok {
		return false
	}
	if g := tr.Generics; g != nil {
		for _, p := range g.Params {
			if p.Kind == ast.GPLifetime {
				return false
			}
		}
		for _, wp := range g.Where {
			if wp.Lifetime != "" {
				return false
			}
			for _, bd := range wp.Bounds {
				if bd.Lifetime != "" {
					return false
				}
			}
		}
	}
	for _, bd := range tr.Supertraits {
		if bd.Lifetime != "" && bd.Lifetime != "'static" {
			return false
		}
	}
	return true
}

// placeThroughRef reports whether a place is reached through a reference
// or raw pointer (not a Box).
func placeThroughRef(p Place) bool {
	for _, pr := range p.Proj {
		if pr.Kind == PDeref && pr.Deref != DBox {
			return true
		}
	}
	return false
}

// closureReturnsLateBound reports whether a closure without an expected
// signature returns a value holding the region of a parameter declared
// `&T` (an elided, hence late-bound, lifetime its result type cannot
// name: "lifetime may not live long enough").
func closureReturnsLateBound(x *ast.ClosureExpr, sm *closSummary) bool {
	if x.Async || x.MoveParamsOf != nil || len(sm.params) != len(x.Params) {
		return false
	}
	for i, p := range x.Params {
		if p.Ty == nil {
			continue
		}
		a := p.Ty
		for {
			pt, ok := a.(*ast.ParenType)
			if !ok {
				break
			}
			a = pt.Elem
		}
		rt, ok := a.(*ast.RefType)
		if !ok || (rt.Lifetime != "" && rt.Lifetime != "'_") {
			continue
		}
		pl := sm.params[i]
		if pl == nil || pl.flat || pl.root == nil || pl.root.ty == nil || pl.root.ty.Kind != TRef || pl.n() == 0 {
			continue
		}
		for _, fa := range sm.fromArg {
			for _, a := range fa {
				if a[0] == i && a[1] == 0 {
					return true
				}
			}
		}
	}
	return false
}

// implSelfName returns the name of the self type of the impl a method is
// defined in ("" if not a path).
func implSelfName(d *Def) string {
	if d == nil || d.Parent == nil || d.Parent.Item == nil {
		return ""
	}
	im, ok := d.Parent.Item.Kind.(*ast.ImplItem)
	if !ok || im.Trait != nil {
		return ""
	}
	pt, ok := im.SelfTy.(*ast.PathType)
	if !ok || pt.QSelf != nil || pt.Path == nil || len(pt.Path.Segments) == 0 {
		return ""
	}
	return pt.Path.Segments[len(pt.Path.Segments)-1].Ident.Name
}

// dynLifetimeBound returns the explicit lifetime bound of a `dyn` type
// ("" if none).
func dynLifetimeBound(x *ast.DynTraitType) string {
	for _, bd := range x.Bounds {
		if bd.Lifetime != "" {
			return bd.Lifetime
		}
	}
	return ""
}

// staticObjects collects the reference depths of the trait objects in a
// parameter's declared type a (with instance type t) that are 'static,
// explicitly or by default: everything such an object holds must be
// 'static. inAdt: a is a type argument of an ADT without lifetime
// parameters.
func staticObjects(a ast.Type, t Ty, depth int, inAdt bool, out *[]int, nest int) {
	if a == nil || t == nil || nest > 12 {
		return
	}
	switch x := a.(type) {
	case *ast.ParenType:
		staticObjects(x.Elem, t, depth, inAdt, out, nest+1)
	case *ast.RefType:
		if t.Kind == TRef {
			staticObjects(x.Elem, t.Elem, depth+1, false, out, nest+1)
		}
	case *ast.SliceType:
		if t.Kind == TSlice {
			staticObjects(x.Elem, t.Elem, depth, false, out, nest+1)
		}
	case *ast.ArrayType:
		if t.Kind == TArray {
			staticObjects(x.Elem, t.Elem, depth, false, out, nest+1)
		}
	case *ast.TupleType:
		if t.Kind == TTuple && len(t.Elems) == len(x.Elems) {
			for i, e := range x.Elems {
				staticObjects(e, t.Elems[i], depth, false, out, nest+1)
			}
		}
	case *ast.DynTraitType:
		if t.Kind != TDyn {
			return
		}
		lt := dynLifetimeBound(x)
		if lt == "'static" || (lt == "" && inAdt && objectDefaultStatic(t)) {
			*out = append(*out, depth)
		}
	case *ast.PathType:
		if x.QSelf != nil || x.Path == nil || len(x.Path.Segments) == 0 || t.Kind != TAdt || t.Def == nil {
			return
		}
		seg := x.Path.Segments[len(x.Path.Segments)-1]
		if seg.Ident.Name != t.Def.Name || seg.Args == nil || seg.Args.Parenthesized {
			return
		}
		var tys []ast.Type
		for _, ga := range seg.Args.Args {
			if ga.Type != nil {
				tys = append(tys, ga.Type)
			}
		}
		if len(tys) > len(t.Args) {
			return
		}
		lts := adtHasLifetimes(t.Def)
		d := depth
		if lts {
			d++
		}
		for i, e := range tys {
			staticObjects(e, t.Args[i], d, !lts, out, nest+1)
		}
	}
}

// explicitStatics collects the reference depths in a declared type a
// (with type t) where the type explicitly requires 'static for all it
// holds: `&'static T` (T: 'static) and `dyn Trait + 'static`. Default
// object lifetimes do not apply: in bodies they are inferred.
func explicitStatics(a ast.Type, t Ty, depth int, out *[]int, nest int) {
	if a == nil || t == nil || nest > 12 {
		return
	}
	switch x := a.(type) {
	case *ast.ParenType:
		explicitStatics(x.Elem, t, depth, out, nest+1)
	case *ast.RefType:
		if t.Kind != TRef {
			return
		}
		if x.Lifetime == "'static" {
			*out = append(*out, depth)
			return
		}
		explicitStatics(x.Elem, t.Elem, depth+1, out, nest+1)
	case *ast.SliceType:
		if t.Kind == TSlice {
			explicitStatics(x.Elem, t.Elem, depth, out, nest+1)
		}
	case *ast.ArrayType:
		if t.Kind == TArray {
			explicitStatics(x.Elem, t.Elem, depth, out, nest+1)
		}
	case *ast.TupleType:
		if t.Kind == TTuple && len(t.Elems) == len(x.Elems) {
			for i, e := range x.Elems {
				explicitStatics(e, t.Elems[i], depth, out, nest+1)
			}
		}
	case *ast.DynTraitType:
		if t.Kind == TDyn && dynLifetimeBound(x) == "'static" {
			*out = append(*out, depth)
		}
	case *ast.PathType:
		if x.QSelf != nil || x.Path == nil || len(x.Path.Segments) == 0 || t.Kind != TAdt || t.Def == nil {
			return
		}
		seg := x.Path.Segments[len(x.Path.Segments)-1]
		if seg.Ident.Name != t.Def.Name || seg.Args == nil || seg.Args.Parenthesized {
			return
		}
		var tys []ast.Type
		for _, ga := range seg.Args.Args {
			if ga.Type != nil {
				tys = append(tys, ga.Type)
			}
		}
		if len(tys) > len(t.Args) {
			return
		}
		d := depth
		if adtHasLifetimes(t.Def) {
			d++
		}
		for i, e := range tys {
			explicitStatics(e, t.Args[i], d, out, nest+1)
		}
	}
}
