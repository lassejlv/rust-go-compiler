package typeck

import (
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
)

// Obligation is a pending trait predicate or projection equality.
type Obligation struct {
	Pred  *Predicate // trait predicate (with optional bindings)
	Proj  Ty         // projection to normalize (if non-nil)
	Var   Ty         // ... and the variable to unify the result with
	Span  lexer.Span
	Depth int
	Done  bool
}

type SelKind int

const (
	SelNone SelKind = iota
	SelImpl
	SelBuiltin
	SelObject
	SelAmbig
)

// Selection is the result of trait selection.
type Selection struct {
	Kind SelKind
	Impl *Def
	Args []Ty // impl args (for SelImpl)
}

const maxSelectDepth = 40

func (ic *InferCtxt) lang(name string) *Def { return ic.t.R.LangItems[name] }

func (ic *InferCtxt) isLang(d *Def, name string) bool {
	return d != nil && d.LangItem == name
}

// evaluate checks whether a predicate holds (without committing bindings).
// Returns SelImpl/SelBuiltin/SelObject (holds), SelAmbig, or SelNone.
func (ic *InferCtxt) evaluate(p *Predicate, depth int) SelKind {
	s := ic.snapshot()
	defer ic.rollback(s)
	sel := ic.selectTrait(p.Trait, depth)
	if sel.Kind == SelNone || sel.Kind == SelAmbig {
		return sel.Kind
	}
	for _, b := range p.Bindings {
		n := ic.normalizeBinding(p.Trait, b.Name, depth)
		if n == nil {
			return SelAmbig
		}
		if !ic.unify(n, b.Ty) {
			return SelNone
		}
	}
	return sel.Kind
}

// normalizeBinding normalizes <tr as Trait>::name, looking through
// supertraits for the trait declaring the assoc type.
func (ic *InferCtxt) normalizeBinding(tr *TraitRef, name string, depth int) Ty {
	for _, st := range ic.t.AllSupertraits(tr) {
		if it, ok := st.Trait.Trait.Items[name]; ok && it.Kind == DefAssocTy {
			return ic.normalizeProjection(st.Trait, ic.resolveList(st.Args), name, depth)
		}
	}
	return nil
}

// selectTrait finds the source of a trait impl. On a unique impl match the
// impl's trait ref is unified with tr (committing inference).
func (ic *InferCtxt) selectTrait(tr *TraitRef, depth int) Selection {
	if depth > maxSelectDepth {
		return Selection{Kind: SelAmbig}
	}
	t := ic.t
	args := ic.resolveList(tr.Args)
	for i, a := range args {
		if a.HasProj() {
			args[i] = ic.tryNormalize(a)
		}
	}
	tr = &TraitRef{Trait: tr.Trait, Args: args}
	self := ic.shallow(args[0])
	trait := tr.Trait

	if self.Kind == TError {
		return Selection{Kind: SelBuiltin}
	}
	// Auto / marker traits that we treat as always satisfied.
	if trait.Trait.Auto {
		return Selection{Kind: SelBuiltin}
	}
	switch trait.LangItem {
	case "sized":
		if self.Kind == TInfer && self.Sub == 0 {
			return Selection{Kind: SelAmbig}
		}
		if self.IsUnsized() {
			return Selection{Kind: SelNone}
		}
		return Selection{Kind: SelBuiltin}
	case "copy":
		if r, ok := ic.builtinCopy(self, depth); ok {
			return r
		}
	case "clone":
		if r, ok := ic.builtinClone(self, depth); ok {
			return r
		}
	case "fn", "fn_mut", "fn_once":
		if r, ok := ic.builtinFn(tr, self, depth); ok {
			return r
		}
	case "tuple_trait":
		if self.Kind == TTuple {
			return Selection{Kind: SelBuiltin}
		}
		if self.Kind == TInfer {
			return Selection{Kind: SelAmbig}
		}
		return Selection{Kind: SelNone}
	case "any_trait", "destruct":
		return Selection{Kind: SelBuiltin}
	case "discriminant_kind", "pointee":
		return Selection{Kind: SelBuiltin}
	}
	if self.Kind == TInfer && self.Sub == 0 {
		return Selection{Kind: SelAmbig}
	}
	// Trait objects implement their principal trait and its supertraits.
	if self.Kind == TDyn && self.Def != nil {
		obj := &TraitRef{Trait: self.Def, Args: append([]Ty{self}, self.Args...)}
		for _, st := range t.AllSupertraits(obj) {
			if st.Trait == trait {
				if ic.unifyList(st.Args, tr.Args) {
					return Selection{Kind: SelObject}
				}
			}
		}
	}
	// Impl candidates.
	impls := t.traitImpls[trait]
	selfKey := ""
	if self.Kind != TInfer {
		selfKey = SimplifyKey(self)
	}
	var cands []*Def
	var candArgs [][]Ty
	ambig := false
	for _, impl := range impls {
		if impl.Impl.Negative {
			continue
		}
		ist := impl.Impl.TraitRef.Args[0]
		if selfKey != "" && ist.Kind != TParam {
			if k := SimplifyKey(ist); k != "" && k != selfKey {
				continue
			}
		}
		s := ic.snapshot()
		iargs, ok, amb := ic.matchImpl(impl, tr, depth)
		ic.rollback(s)
		if !ok {
			continue
		}
		cands = append(cands, impl)
		candArgs = append(candArgs, iargs)
		if amb {
			ambig = true
		}
	}
	if len(cands) == 0 {
		if self.Kind == TInfer {
			return Selection{Kind: SelAmbig}
		}
		return Selection{Kind: SelNone}
	}
	if len(cands) > 1 {
		return Selection{Kind: SelAmbig}
	}
	_ = ambig
	// Commit the unique candidate.
	iargs, _, _ := ic.matchImpl(cands[0], tr, depth)
	return Selection{Kind: SelImpl, Impl: cands[0], Args: ic.resolveList(iargs)}
}

// matchImpl unifies an impl's trait ref with tr and checks its where clauses.
func (ic *InferCtxt) matchImpl(impl *Def, tr *TraitRef, depth int) ([]Ty, bool, bool) {
	t := ic.t
	g := t.Generics(impl)
	args := make([]Ty, len(g.Params))
	for i := range args {
		args[i] = ic.newVar()
	}
	itr := impl.Impl.TraitRef.Subst(args)
	for i := range itr.Args {
		itr.Args[i] = ic.tryNormalize(itr.Args[i])
	}
	if !ic.unifyList(itr.Args, tr.Args) {
		return nil, false, false
	}
	ambig := false
	for _, p := range g.Predicates {
		if p.Maybe {
			continue
		}
		sp := p.Subst(args)
		switch ic.evaluate(sp, depth+1) {
		case SelNone:
			return nil, false, false
		case SelAmbig:
			ambig = true
		}
	}
	return args, true, ambig
}

// builtinCopy handles Copy for structural types.
func (ic *InferCtxt) builtinCopy(self Ty, depth int) (Selection, bool) {
	switch self.Kind {
	case TInt, TFloat, TBool, TChar, TNever, TPtr, TFnDef, TFnPtr:
		return Selection{Kind: SelBuiltin}, true
	case TRef:
		if self.Mut {
			return Selection{Kind: SelNone}, true
		}
		return Selection{Kind: SelBuiltin}, true
	case TInfer:
		if self.Sub != 0 {
			return Selection{Kind: SelBuiltin}, true
		}
		return Selection{Kind: SelAmbig}, true
	case TTuple:
		return ic.allImpl(self.Elems, "copy", depth), true
	case TArray:
		return ic.allImpl([]Ty{self.Elem}, "copy", depth), true
	case TClosure:
		ci := ic.t.Closures[self]
		if ci == nil {
			return Selection{Kind: SelAmbig}, true
		}
		var tys []Ty
		for _, c := range ci.Captures {
			if c.ByRef {
				if c.Mut {
					return Selection{Kind: SelNone}, true
				}
				continue
			}
			tys = append(tys, c.Ty)
		}
		return ic.allImpl(tys, "copy", depth), true
	case TStr, TSlice, TDyn:
		return Selection{Kind: SelNone}, true
	}
	return Selection{}, false
}

// builtinClone handles Clone for types whose clone is a bitwise copy or
// that have no user impls (fn items, closures).
func (ic *InferCtxt) builtinClone(self Ty, depth int) (Selection, bool) {
	switch self.Kind {
	case TFnDef, TFnPtr, TNever:
		return Selection{Kind: SelBuiltin}, true
	case TClosure:
		r, _ := ic.builtinCopy(self, depth)
		return r, true
	case TInfer:
		if self.Sub != 0 {
			return Selection{Kind: SelBuiltin}, true
		}
	}
	return Selection{}, false
}

func (ic *InferCtxt) allImpl(tys []Ty, lang string, depth int) Selection {
	tr := ic.lang(lang)
	if tr == nil {
		return Selection{Kind: SelBuiltin}
	}
	res := SelBuiltin
	for _, e := range tys {
		switch ic.evaluate(&Predicate{Trait: &TraitRef{Trait: tr, Args: []Ty{e}}}, depth+1) {
		case SelNone:
			return Selection{Kind: SelNone}
		case SelAmbig:
			res = SelAmbig
		}
	}
	return Selection{Kind: res}
}

// fnKindRank: Fn=0, FnMut=1, FnOnce=2.
func fnKindRank(lang string) int {
	switch lang {
	case "fn":
		return 0
	case "fn_mut":
		return 1
	}
	return 2
}

// FnSigOf returns the callable signature of closures, fn items and fn
// pointers (inputs, output) or ok=false.
func (ic *InferCtxt) fnSigOf(self Ty) ([]Ty, Ty, int, bool) {
	t := ic.t
	switch self.Kind {
	case TFnDef:
		ins, out := t.InstantiateFnSig(self.Def, self.Args)
		return ins, out, 0, true
	case TFnPtr:
		return self.Elems, self.Ret, 0, true
	case TClosure:
		ci := t.Closures[self]
		if ci == nil {
			return nil, nil, 0, false
		}
		return ci.Inputs, ci.Output, ci.Kind, true
	}
	return nil, nil, 0, false
}

func (ic *InferCtxt) builtinFn(tr *TraitRef, self Ty, depth int) (Selection, bool) {
	want := fnKindRank(tr.Trait.LangItem)
	switch self.Kind {
	case TFnDef, TFnPtr, TClosure:
		ins, _, kind, ok := ic.fnSigOf(self)
		if !ok {
			return Selection{Kind: SelAmbig}, true
		}
		if kind > want {
			return Selection{Kind: SelNone}, true
		}
		if len(tr.Args) > 1 && !ic.unify(tr.Args[1], Tuple(ins)) {
			return Selection{Kind: SelNone}, true
		}
		return Selection{Kind: SelBuiltin}, true
	case TRef:
		inner := ic.shallow(self.Elem)
		if inner.Kind == TInfer {
			return Selection{Kind: SelAmbig}, true
		}
		// &F: Fn if F: Fn; &mut F: FnMut if F: FnMut
		need := "fn"
		if self.Mut {
			need = "fn_mut"
		}
		if fnKindRank(need) > want {
			return Selection{Kind: SelNone}, true
		}
		sub := &TraitRef{Trait: ic.lang(need), Args: append([]Ty{inner}, tr.Args[1:]...)}
		r := ic.selectTrait(sub, depth+1)
		if r.Kind == SelNone || r.Kind == SelAmbig {
			return Selection{Kind: r.Kind}, true
		}
		return Selection{Kind: SelBuiltin}, true
	case TAdt:
		if self.Def.LangItem == "owned_box" {
			sub := &TraitRef{Trait: tr.Trait, Args: append([]Ty{self.Args[0]}, tr.Args[1:]...)}
			r := ic.selectTrait(sub, depth+1)
			if r.Kind == SelNone || r.Kind == SelAmbig {
				return Selection{Kind: r.Kind}, true
			}
			return Selection{Kind: SelBuiltin}, true
		}
	}
	return Selection{}, false
}

// normalizeProjection computes <args[0] as trait<args[1:]>>::name, or nil
// if it cannot (yet) be determined.
func (ic *InferCtxt) normalizeProjection(trait *Def, args []Ty, name string, depth int) Ty {
	if depth > maxSelectDepth {
		return nil
	}
	t := ic.t
	self := ic.shallow(args[0])
	if self.Kind == TError {
		return TyErr
	}
	if self.Kind == TInfer && self.Sub == 0 {
		return nil
	}
	if self.Kind == TParam {
		return nil
	}
	// Fn-family Output.
	if trait.LangItem == "fn_once" && name == "Output" {
		cur := self
		for {
			if cur.Kind == TRef {
				cur = ic.shallow(cur.Elem)
				continue
			}
			if cur.Kind == TAdt && cur.Def.LangItem == "owned_box" {
				cur = ic.shallow(cur.Args[0])
				continue
			}
			break
		}
		if _, out, _, ok := ic.fnSigOf(cur); ok {
			if len(args) > 1 {
				ins, _, _, _ := ic.fnSigOf(cur)
				ic.unify(args[1], Tuple(ins))
			}
			return out
		}
		if cur.Kind == TDyn {
			self = cur
		} else if cur.Kind == TInfer {
			return nil
		}
	}
	if self.Kind == TDyn {
		for _, b := range self.Bindings {
			if b.Name == name {
				return b.Ty
			}
		}
	}
	tr := &TraitRef{Trait: trait, Args: args}
	sel := ic.selectTrait(tr, depth+1)
	switch sel.Kind {
	case SelImpl:
		if it, ok := sel.Impl.Impl.Items[name]; ok && it.Kind == DefAssocTy {
			return ic.tryNormalize(Subst(t.ImplAssocTy(it), sel.Args))
		}
		// default in trait
		if it, ok := trait.Trait.Items[name]; ok && it.AssocTy.Default != nil {
			ac := t.itemConv(it)
			t.Generics(it)
			dt := ac.lowerTy(it.AssocTy.Default)
			return ic.tryNormalize(Subst(dt, args))
		}
	case SelObject:
		for _, b := range self.Bindings {
			if b.Name == name {
				return b.Ty
			}
		}
	}
	return nil
}

// SelectImplSimple selects an impl for a fully concrete trait ref (no
// inference context needed). Returns nil if none.
func (t *Tcx) SelectImplSimple(tr *TraitRef) (*Def, []Ty) {
	ic := newInferCtxt(t)
	sel := ic.selectTrait(tr, 0)
	if sel.Kind == SelImpl {
		return sel.Impl, ic.resolveList(sel.Args)
	}
	return nil, nil
}

// Implements reports whether ty implements the trait (with given extra
// args), for concrete types.
func (t *Tcx) Implements(ty Ty, trait *Def, extra ...Ty) bool {
	if trait == nil {
		return false
	}
	ic := newInferCtxt(t)
	args := append([]Ty{ty}, extra...)
	g := t.Generics(trait)
	for len(args) < len(g.Params) {
		args = append(args, ic.newVar())
	}
	k := ic.evaluate(&Predicate{Trait: &TraitRef{Trait: trait, Args: args}}, 0)
	return k == SelImpl || k == SelBuiltin || k == SelObject
}

// IsCopy reports whether a concrete type is Copy.
func (t *Tcx) IsCopy(ty Ty) bool {
	return t.Implements(ty, t.LangOpt("copy"))
}

// Normalize fully normalizes projections in a concrete type.
func (t *Tcx) Normalize(ty Ty) Ty {
	if !ty.HasProj() && !ty.HasOpaque() {
		return ty
	}
	ic := newInferCtxt(t)
	ty = ic.tryNormalize(ty)
	return t.RevealOpaque(ty)
}

// ResolveAssocFn maps a trait method (with args [Self, trait args...,
// method args...]) to the concrete implementation for codegen: the impl
// method def and its args, or the trait's default method def with the same
// args. ok=false if Self is a trait object (virtual call) or nothing found.
func (t *Tcx) ResolveAssocFn(d *Def, args []Ty) (*Def, []Ty, bool) {
	if !d.InTrait() {
		return d, args, true
	}
	trait := d.Parent
	tg := t.Generics(trait)
	ntrait := len(tg.Params)
	self := args[0]
	if self.Kind == TDyn {
		return d, args, false
	}
	tr := &TraitRef{Trait: trait, Args: args[:ntrait]}
	ic := newInferCtxt(t)
	sel := ic.selectTrait(tr, 0)
	switch sel.Kind {
	case SelImpl:
		iargs := ic.resolveList(sel.Args)
		if im, ok := sel.Impl.Impl.Items[d.Name]; ok {
			full := append(append([]Ty{}, iargs...), args[ntrait:]...)
			return im, full, true
		}
		// default method: keep trait def with trait args
		return d, args, true
	case SelBuiltin:
		return d, args, true
	case SelObject:
		return d, args, false
	}
	return nil, nil, false
}
