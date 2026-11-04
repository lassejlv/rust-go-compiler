package typeck

import (
	. "rustgo/compiler/middle"
)

// InferCtxt holds inference variable bindings with snapshot support.
type InferCtxt struct {
	t    *Tcx
	bind map[int]Ty
	// tab interns the types mentioning this context's variables.
	tab  *InferTable
	undo []int
	// pending obligations (owned by FnCtxt but stored here so selection can
	// add nested ones)
	obligations []*Obligation
	// env holds the (substituted) where-clauses of the body being checked;
	// they guide inference when impl selection is ambiguous.
	env []*TraitRef
	// evalCache memoizes evaluate() by canonical query (see canonicalKey).
	evalCache map[string]SelKind
	// queryCache memoizes selection/normalization over unsealed closures
	// by canonical query (see canon.go).
	queryCache map[string]*canonResp
	// paramNormActive guards normalizeParamByImpl against re-entry.
	paramNormActive map[string]bool
	// normNest counts nested tryNormalize calls (recursion bound).
	normNest int
	// envBindings: associated-type bindings of where-clauses
	// (`I: IntoIterator<Item = (K, V)>`), for normalizing in generic bodies.
	envBindings []*Predicate
	// pickFirst makes ambiguous impl selection commit to the first
	// candidate (last-resort fallback after inference).
	pickFirst bool
}

func newInferCtxt(t *Tcx) *InferCtxt {
	return &InferCtxt{t: t, bind: map[int]Ty{}, tab: NewInferTable()}
}

func (ic *InferCtxt) newVar() Ty      { return NewInferIn(ic.tab, 0) }
func (ic *InferCtxt) newIntVar() Ty   { return NewInferIn(ic.tab, 1) }
func (ic *InferCtxt) newFloatVar() Ty { return NewInferIn(ic.tab, 2) }

type snapshot struct {
	undo int
	obl  int
}

func (ic *InferCtxt) snapshot() snapshot { return snapshot{len(ic.undo), len(ic.obligations)} }

func (ic *InferCtxt) rollback(s snapshot) {
	for i := len(ic.undo) - 1; i >= s.undo; i-- {
		delete(ic.bind, ic.undo[i])
	}
	ic.undo = ic.undo[:s.undo]
	ic.obligations = ic.obligations[:s.obl]
}

func (ic *InferCtxt) setVar(id int, t Ty) {
	ic.bind[id] = t
	ic.undo = append(ic.undo, id)
}

// shallow follows variable bindings at the top level.
func (ic *InferCtxt) shallow(t Ty) Ty {
	for t != nil && t.Kind == TInfer {
		b, ok := ic.bind[t.Index]
		if !ok {
			return t
		}
		t = b
	}
	return t
}

// resolve replaces all bound inference variables (deep).
func (ic *InferCtxt) resolve(t Ty) Ty {
	if t == nil || !t.HasInfer() {
		return t
	}
	return Fold(t, func(x Ty) Ty {
		if !x.HasInfer() {
			return x
		}
		if x.Kind == TInfer {
			s := ic.shallow(x)
			if s == x {
				return x
			}
			return ic.resolve(s)
		}
		return nil
	})
}

func (ic *InferCtxt) resolveList(ts []Ty) []Ty {
	out := make([]Ty, len(ts))
	for i, t := range ts {
		out[i] = ic.resolve(t)
	}
	return out
}

func (ic *InferCtxt) isUnresolvedVar(t Ty) bool {
	t = ic.shallow(t)
	return t.Kind == TInfer
}

func (ic *InferCtxt) occurs(id int, t Ty) bool {
	found := false
	Walk(t, func(x Ty) bool {
		if found {
			return false
		}
		if x.Kind == TInfer {
			if x.Index == id {
				found = true
				return false
			}
			if b, ok := ic.bind[x.Index]; ok {
				if ic.occurs(id, b) {
					found = true
				}
			}
		}
		return x.HasInfer()
	})
	return found
}

// unify makes a and b equal, binding variables. Returns false on mismatch
// (bindings made before the failure are NOT rolled back; use a snapshot
// when probing).
func (ic *InferCtxt) unify(a, b Ty) bool {
	a = ic.shallow(a)
	b = ic.shallow(b)
	if a == b {
		return true
	}
	if a.Kind == TError || b.Kind == TError {
		return true
	}
	if a.Kind == TInfer || b.Kind == TInfer {
		if a.Kind != TInfer {
			a, b = b, a
		}
		// a is a var
		if b.Kind == TInfer {
			// var-var: prefer keeping the more specific kind
			if a.Sub == 0 {
				ic.setVar(a.Index, b)
				return true
			}
			if b.Sub == 0 {
				ic.setVar(b.Index, a)
				return true
			}
			if a.Sub != b.Sub {
				return false
			}
			ic.setVar(a.Index, b)
			return true
		}
		switch a.Sub {
		case 1:
			if b.Kind != TInt {
				return false
			}
		case 2:
			if b.Kind != TFloat {
				return false
			}
		}
		if ic.occurs(a.Index, b) {
			return false
		}
		ic.setVar(a.Index, b)
		return true
	}
	if a.Kind == TProjection || b.Kind == TProjection {
		na := ic.tryNormalize(a)
		nb := ic.tryNormalize(b)
		if na != a || nb != b {
			return ic.unify(na, nb)
		}
		return false
	}
	if a.Kind != b.Kind {
		return false
	}
	switch a.Kind {
	case TInt, TFloat:
		return a.Sub == b.Sub
	case TBool, TChar, TStr, TNever:
		return true
	case TTuple:
		return ic.unifyList(a.Elems, b.Elems)
	case TArray:
		return ic.unify(a.Elem, b.Elem) && ic.unify(a.Len, b.Len)
	case TSlice:
		return ic.unify(a.Elem, b.Elem)
	case TRef, TPtr:
		return a.Mut == b.Mut && ic.unify(a.Elem, b.Elem)
	case TAdt, TFnDef, TOpaque:
		return a.Def == b.Def && a.Index == b.Index && ic.unifyList(a.Args, b.Args)
	case TFnPtr:
		return a.Sub == b.Sub && a.Name == b.Name && ic.unifyList(a.Elems, b.Elems) && ic.unify(a.Ret, b.Ret)
	case TClosure:
		return a.Index == b.Index && ic.unifyList(a.Args, b.Args)
	case TDyn:
		if a.Def != b.Def || len(a.Bindings) != len(b.Bindings) || a.Sub != b.Sub {
			return false
		}
		if !ic.unifyList(a.Args, b.Args) {
			return false
		}
		for i := range a.Bindings {
			if a.Bindings[i].Name != b.Bindings[i].Name || !ic.unify(a.Bindings[i].Ty, b.Bindings[i].Ty) {
				return false
			}
		}
		return true
	case TParam:
		return a.Index == b.Index
	case TConst:
		return a.Val == b.Val
	}
	return false
}

func (ic *InferCtxt) unifyList(a, b []Ty) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if !ic.unify(a[i], b[i]) {
			return false
		}
	}
	return true
}

// probeUnify checks whether a and b can unify without keeping bindings.
func (ic *InferCtxt) probeUnify(a, b Ty) bool {
	s := ic.snapshot()
	ok := ic.unify(a, b)
	ic.rollback(s)
	return ok
}

// tryNormalize normalizes projections (deeply) where possible.
func (ic *InferCtxt) tryNormalize(t Ty) Ty {
	if t == nil || !t.HasProj() {
		return t
	}
	// normalization can re-enter selection (impl where-clauses with
	// projections, e.g. typenum's recursive arithmetic impls): bound the
	// nesting so it can't recurse without limit
	if ic.normNest > maxSelectDepth {
		return t
	}
	ic.normNest++
	defer func() { ic.normNest-- }()
	return Fold(t, func(x Ty) Ty {
		if !x.HasProj() {
			return x
		}
		if x.Kind == TProjection {
			args := make([]Ty, len(x.Args))
			for i, a := range x.Args {
				args[i] = ic.tryNormalize(ic.resolve(a))
			}
			if n := ic.normalizeProjection(x.Def, args, x.Name, ic.normNest); n != nil {
				return ic.tryNormalize(n)
			}
			return Projection(x.Def, args, x.Name)
		}
		return nil
	})
}
