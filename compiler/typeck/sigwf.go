package typeck

// Well-formedness of the types in generic signatures: `S<T>` in a fn
// signature or impl header needs the where-clauses of `struct S<X: Bound>`
// to hold for T, which only the item's own bounds can provide (E0277;
// rustc implies no trait bounds from argument types).

import (
	. "rustgo/compiler/middle"
)

func (t *Tcx) checkSigAdtBounds(d *Def) {
	var tys []Ty
	ok := t.quietly(func() {
		switch d.Kind {
		case DefFn:
			if sig := t.FnSig(d); sig != nil {
				tys = append(append(tys, sig.Inputs...), sig.Output)
			}
		case DefImpl:
			t.ImplHeader(d)
			if d.Impl != nil {
				tys = append(tys, d.Impl.SelfTy)
				if d.Impl.TraitRef != nil {
					tys = append(tys, d.Impl.TraitRef.Args...)
				}
			}
		}
	})
	if !ok {
		return
	}
	g := t.Generics(d)
	var env []*TraitRef
	for _, p := range g.Predicates {
		if p.Maybe || p.Trait == nil {
			continue
		}
		env = append(env, p.Trait)
		var sups []*TraitRef
		if t.quietly(func() { sups = t.AllSupertraits(p.Trait) }) {
			env = append(env, sups...)
		}
	}
	holds := func(tr *TraitRef) bool {
		for _, e := range env {
			if e.Trait != tr.Trait || len(e.Args) != len(tr.Args) {
				continue
			}
			same := true
			for i := range e.Args {
				if e.Args[i] != tr.Args[i] {
					same = false
				}
			}
			if same {
				return true
			}
		}
		return false
	}
	seen := map[Ty]bool{}
	for _, ty := range tys {
		if ty == nil || ty.HasError() {
			continue
		}
		var bad *TraitRef
		Walk(ty, func(x Ty) bool {
			if bad != nil {
				return false
			}
			if x.Kind == TProjection || x.Kind == TOpaque || x.Kind == TDyn || x.Kind == TFnPtr {
				return false // (binders, projections: not modelled)
			}
			if x.Kind != TAdt || seen[x] || t.isStd(x.Def) || len(x.Args) == 0 {
				return true
			}
			seen[x] = true
			ag := t.Generics(x.Def)
			if ag == nil || len(ag.Params) != len(x.Args) {
				return true
			}
			for _, p := range ag.Predicates {
				if p.Maybe || p.Trait == nil || p.Trait.Trait == nil || len(p.Trait.Args) == 0 || p.Trait.Trait.LangItem == "sized" || t.autoKind(p.Trait.Trait) != "" {
					continue
				}
				ref := p.Trait.Subst(x.Args)
				self := ref.Args[0]
				if self == nil || self.Kind != TParam || self.Sub != 0 {
					continue
				}
				known := true
				for _, a := range ref.Args {
					if a == nil || a.HasInfer() || a.HasProj() || a.HasOpaque() || a.HasError() {
						known = false
					}
				}
				if !known || holds(ref) || t.hasBlanketImpl(ref.Trait) {
					continue
				}
				bad = ref
				return false
			}
			return true
		})
		if bad != nil {
			sp := d.Span
			if d.Item != nil {
				sp = d.Item.Name.Span
				if d.Kind == DefImpl {
					sp = d.Item.Span
				}
			}
			t.ErrorCode(sp, "E0277", "the trait bound `%s: %s` is not satisfied", bad.Args[0], bad.Trait.Name)
			return
		}
	}
}

// hasBlanketImpl reports an impl of trait for a bare type parameter
// (`impl<T> Trait for T`), which could make the bound hold.
func (t *Tcx) hasBlanketImpl(tr *Def) bool {
	t.BuildImpls()
	for _, impl := range t.traitImpls[tr] {
		if impl.Impl != nil && impl.Impl.TraitRef != nil && len(impl.Impl.TraitRef.Args) > 0 {
			if a := impl.Impl.TraitRef.Args[0]; a != nil && a.Kind == TParam {
				return true
			}
		}
	}
	return false
}
