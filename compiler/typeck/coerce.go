package typeck

import (
	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

// coerce tries to coerce expression e of type from to type to, recording
// adjustments. Returns false (without error) on failure.
func (fcx *FnCtxt) coerce(e ast.Expr, from, to Ty) bool {
	a := fcx.shallow(from)
	b := fcx.shallow(to)
	if a == b {
		return true
	}
	if a.Kind == TError || b.Kind == TError {
		return true
	}
	if a.Kind == TNever {
		fcx.addAdjust(e, Adjustment{Kind: AdjNeverToAny, Target: to})
		if b.Kind == TInfer && b.Sub == 0 {
			fcx.diverging = append(fcx.diverging, b)
		}
		return true
	}
	if a.Kind == TInfer || b.Kind == TInfer {
		return fcx.ic.unify(a, b)
	}
	switch b.Kind {
	case TRef:
		if a.Kind == TRef {
			return fcx.coerceRef(e, a, b)
		}
	case TPtr:
		if a.Kind == TRef {
			if b.Mut && !a.Mut {
				return false
			}
			s := fcx.ic.snapshot()
			if fcx.ic.unify(a.Elem, b.Elem) {
				fcx.addAdjust(e, Adjustment{Kind: AdjDeref, Target: a.Elem}, Adjustment{Kind: AdjBorrowRaw, Mut: b.Mut, Target: b})
				return true
			}
			fcx.ic.rollback(s)
			if fcx.canUnsize(a.Elem, b.Elem) {
				fcx.addAdjust(e, Adjustment{Kind: AdjDeref, Target: a.Elem}, Adjustment{Kind: AdjBorrowRaw, Mut: b.Mut, Target: Ptr(a.Elem, b.Mut)}, Adjustment{Kind: AdjUnsize, Target: b})
				return true
			}
			return false
		}
		if a.Kind == TPtr && a.Mut && !b.Mut {
			if fcx.ic.unify(a.Elem, b.Elem) {
				fcx.addAdjust(e, Adjustment{Kind: AdjMutToConst, Target: b})
				return true
			}
			return false
		}
		if a.Kind == TPtr && a.Mut == b.Mut {
			s := fcx.ic.snapshot()
			if fcx.ic.unify(a, b) {
				return true
			}
			fcx.ic.rollback(s)
			if fcx.canUnsize(a.Elem, b.Elem) {
				fcx.addAdjust(e, Adjustment{Kind: AdjUnsize, Target: b})
				return true
			}
			return false
		}
	case TFnPtr:
		switch a.Kind {
		case TFnDef:
			ins, out := fcx.instantiateSig(a.Def, a.Args, e.GetSpan())
			if fcx.ic.unify(FnPtr(ins, out, false), b) {
				fcx.addAdjust(e, Adjustment{Kind: AdjReifyFnPtr, Target: b})
				return true
			}
			return false
		case TClosure:
			ci := fcx.t.Closures[a]
			if ci != nil && fcx.ic.unify(FnPtr(ci.Inputs, ci.Output, false), b) {
				fcx.addAdjust(e, Adjustment{Kind: AdjClosureFnPtr, Target: b})
				return true
			}
			return false
		}
	case TAdt:
		if a.Kind == TAdt && a.Def == b.Def && isUnsizableSmartPtr(a.Def) {
			s := fcx.ic.snapshot()
			if fcx.ic.unify(a, b) {
				return true
			}
			fcx.ic.rollback(s)
			ae := fcx.shallow(a.Args[0])
			be := fcx.shallow(b.Args[0])
			if fcx.canUnsize(ae, be) || fcx.canUnsizeNested(ae, be) {
				fcx.addAdjust(e, Adjustment{Kind: AdjUnsize, Target: b})
				return true
			}
			return false
		}
	}
	return fcx.ic.unify(a, b)
}

func isUnsizableSmartPtr(d *Def) bool {
	return d.LangItem == "owned_box" || d.HasAttr("rustc_coerce_unsized")
}

// coerceRef handles &T -> &U coercions: reborrow, deref coercion, unsizing.
func (fcx *FnCtxt) coerceRef(e ast.Expr, a, b Ty) bool {
	if b.Mut && !a.Mut {
		return false
	}
	// direct
	s := fcx.ic.snapshot()
	if fcx.ic.unify(a.Elem, b.Elem) {
		return true
	}
	fcx.ic.rollback(s)
	be := fcx.shallow(b.Elem)
	// unsizing (&[T; N] -> &[T], &T -> &dyn Trait)
	if fcx.canUnsize(fcx.shallow(a.Elem), be) {
		fcx.addAdjust(e, Adjustment{Kind: AdjUnsize, Target: b})
		return true
	}
	// deref coercion: autoderef a.Elem
	steps := fcx.autoderef(a.Elem, e.GetSpan())
	for i := 1; i < len(steps); i++ {
		st := steps[i]
		s := fcx.ic.snapshot()
		if fcx.ic.unify(st.ty, b.Elem) {
			adj := []Adjustment{{Kind: AdjDeref, Target: a.Elem}}
			for _, x := range adjustmentsFor(steps, i) {
				if x.Kind == AdjDerefOverloaded && b.Mut {
					x.Mut = true
				}
				adj = append(adj, x)
			}
			adj = append(adj, Adjustment{Kind: AdjBorrow, Mut: b.Mut, Target: b})
			fcx.addAdjust(e, adj...)
			return true
		}
		fcx.ic.rollback(s)
		// deref then unsize (e.g. &Vec<[T;N]>? rare) / &Box<[T;N]> -> &[T]
		if fcx.canUnsize(st.ty, be) {
			adj := []Adjustment{{Kind: AdjDeref, Target: a.Elem}}
			adj = append(adj, adjustmentsFor(steps, i)...)
			adj = append(adj, Adjustment{Kind: AdjBorrow, Mut: b.Mut, Target: Ref(st.ty, b.Mut)})
			adj = append(adj, Adjustment{Kind: AdjUnsize, Target: b})
			fcx.addAdjust(e, adj...)
			return true
		}
	}
	return false
}

// canUnsize reports whether src can be unsized to dst ([T; N] -> [T],
// T -> dyn Trait), unifying element types.
func (fcx *FnCtxt) canUnsize(src, dst Ty) bool {
	src = fcx.shallow(src)
	dst = fcx.shallow(dst)
	switch dst.Kind {
	case TSlice:
		if src.Kind == TArray {
			return fcx.ic.unify(src.Elem, dst.Elem)
		}
	case TAdt:
		// Struct with an unsized tail: Foo<..., T> -> Foo<..., U> when T -> U.
		if src.Kind == TAdt && src.Def == dst.Def && dst.Def.Adt != nil && dst.Def.Adt.TailKnown && dst.Def.Adt.TailParam >= 0 {
			p := dst.Def.Adt.TailParam
			for i := range src.Args {
				if i != p && !fcx.ic.unify(src.Args[i], dst.Args[i]) {
					return false
				}
			}
			return fcx.canUnsize(src.Args[p], dst.Args[p])
		}
	case TDyn:
		if src.Kind == TDyn {
			// dyn A -> dyn B (auto trait drop / supertrait upcast); auto
			// traits can be dropped but not added
			if dst.Sub&^src.Sub != 0 {
				return false
			}
			if src.Def == dst.Def {
				return fcx.ic.unify(Dyn(src.Def, src.Args, src.Bindings, 0), Dyn(dst.Def, dst.Args, dst.Bindings, 0))
			}
			if src.Def != nil && dst.Def != nil {
				obj := &TraitRef{Trait: src.Def, Args: append([]Ty{src}, src.Args...)}
				for _, st := range fcx.t.AllSupertraits(obj) {
					if st.Trait == dst.Def {
						return fcx.ic.unifyList(st.Args[1:], dst.Args)
					}
				}
			}
			return false
		}
		if src.Kind == TInfer || src.IsUnsized() {
			return false
		}
		if dst.Def == nil {
			return true
		}
		tr := &TraitRef{Trait: dst.Def, Args: append([]Ty{src}, dst.Args...)}
		p := &Predicate{Trait: tr, Bindings: dst.Bindings}
		k := fcx.ic.evaluate(p, 0)
		if k == SelNone {
			return false
		}
		fcx.ic.obligations = append(fcx.ic.obligations, &Obligation{Pred: p})
		return true
	}
	return false
}

// canUnsizeNested handles smart pointers wrapping smart pointers, e.g.
// Pin<Box<T>> -> Pin<Box<dyn Trait>>.
func (fcx *FnCtxt) canUnsizeNested(a, b Ty) bool {
	if a.Kind != TAdt || b.Kind != TAdt || a.Def != b.Def || !isUnsizableSmartPtr(a.Def) || len(a.Args) == 0 {
		return false
	}
	ae := fcx.shallow(a.Args[0])
	be := fcx.shallow(b.Args[0])
	return fcx.canUnsize(ae, be) || fcx.canUnsizeNested(ae, be)
}
