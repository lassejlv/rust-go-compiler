package typeck

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
)

type methodCand struct {
	item    *Def
	impl    *Def      // inherent impl (nil for trait candidates)
	trait   *TraitRef // trait candidate (Self is a fresh var bound during matching)
	object  bool      // trait object candidate
	inherentFromStep int
}

// pick is a successful method lookup.
type pick struct {
	item      *Def
	parent    []Ty // container args (impl args or trait args incl. Self)
	adjust    []Adjustment
	recvTy    Ty // adjusted receiver type
	virtual   bool
}

func (fcx *FnCtxt) checkMethodCall(x *ast.MethodCallExpr, expected Ty) Ty {
	rt := fcx.checkExpr(x.Receiver, nil)
	rs := fcx.structurally(rt)
	if rs.Kind == TInfer {
		switch rs.Sub {
		case 1:
			fcx.ic.unify(rs, TyI32)
		case 2:
			fcx.ic.unify(rs, TyF64)
		default:
			fcx.errorf(x.Span, "type annotations needed: cannot call method `%s` on unknown type", x.Method.Name)
			for _, a := range x.Args {
				fcx.checkExpr(a, nil)
			}
			return TyErr
		}
	}
	p := fcx.lookupMethod(rt, x.Method.Name, x.Span)
	if p == nil {
		fcx.errorf(x.Method.Span, "no method named `%s` found for `%s`", x.Method.Name, fcx.resolve(rt))
		for _, a := range x.Args {
			fcx.checkExpr(a, nil)
		}
		return TyErr
	}
	fcx.addAdjust(x.Receiver, p.adjust...)
	// markMutPlace for &mut autoref
	for _, a := range p.adjust {
		if a.Kind == AdjBorrow && a.Mut {
			fcx.markAdjustMut(x.Receiver)
			fcx.markMutPlace(x.Receiver)
		}
	}
	seg := &ast.PathSegment{Ident: x.Method, Args: x.Generics}
	args := fcx.ownArgs(p.item, seg, p.parent, x.Span)
	ins, out := fcx.instantiateSig(p.item, args, x.Span)
	fcx.registerPredicates(p.item, args, x.Span)
	if len(ins) > 0 {
		if !fcx.ic.unify(ins[0], p.recvTy) {
			fcx.errorf(x.Span, "method receiver mismatch: expected `%s`, found `%s`", fcx.resolve(ins[0]), fcx.resolve(p.recvTy))
		}
		ins = ins[1:]
	}
	// resolve what the receiver determines (e.g. `Box<T>: AsRef<?U>`)
	// before arguments and the result are coerced against expectations
	fcx.selectWherePossible()
	fcx.checkArgs(x.Args, ins, false, x.Span)
	if o := fcx.shallow(out); o.Kind == TInfer && o.Sub == 0 {
		// the result is a pending projection: resolve it now that the
		// arguments are known, before it meets the caller's expectation
		fcx.selectWherePossible()
	}
	kind := CallFn
	if p.virtual {
		kind = CallVirtual
	}
	callee := &Callee{Kind: kind, Def: p.item, Args: args}
	fcx.r.Callees[x.ID] = callee
	if p.item.InTrait() {
		fcx.deferred = append(fcx.deferred, callee)
	}
	return out
}

// lookupMethod probes for a method with autoderef/autoref.
func (fcx *FnCtxt) lookupMethod(recv Ty, name string, sp lexer.Span) *pick {
	steps := fcx.autoderef(recv, sp)
	// array -> slice unsizing as a final step
	last := steps[len(steps)-1].ty
	unsizeStep := -1
	if last.Kind == TArray {
		steps = append(steps, autoderefStep{ty: Slice(last.Elem)})
		unsizeStep = len(steps) - 1
	}
	// Assemble candidates from all steps.
	var inherent []methodCand
	seenImpl := map[*Def]bool{}
	for _, st := range steps {
		key := SimplifyKey(st.ty)
		if key == "" {
			continue
		}
		for _, impl := range fcx.t.inherent[key] {
			if seenImpl[impl] {
				continue
			}
			if item, ok := impl.Impl.Items[name]; ok && item.Kind == DefFn && item.Fn.HasSelf {
				seenImpl[impl] = true
				inherent = append(inherent, methodCand{item: item, impl: impl})
			}
		}
	}
	var traitCands []methodCand
	inScope := fcx.curMod().TraitsInScope(fcx.t.R.Prelude)
	for _, tr := range fcx.t.traitsByItem[name] {
		if !inScope[tr] {
			continue
		}
		item := tr.Trait.Items[name]
		if item.Kind != DefFn || !item.Fn.HasSelf {
			continue
		}
		traitCands = append(traitCands, methodCand{item: item})
	}
	// object candidates
	var objCands []methodCand
	for _, st := range steps {
		if st.ty.Kind == TDyn && st.ty.Def != nil {
			obj := &TraitRef{Trait: st.ty.Def, Args: append([]Ty{st.ty}, st.ty.Args...)}
			for _, s := range fcx.t.AllSupertraits(obj) {
				if item, ok := s.Trait.Trait.Items[name]; ok && item.Kind == DefFn && item.Fn.HasSelf {
					objCands = append(objCands, methodCand{item: item, trait: s, object: true})
				}
			}
		}
	}
	// Where-clause candidates first: in the generic source the receiver
	// was likely a type parameter whose methods come from its bounds.
	if p := fcx.envMethod(steps, name, sp); p != nil {
		return p
	}
	for si, st := range steps {
		u := st.ty
		if u.Kind == TInfer && u.Sub == 0 {
			break
		}
		for mode := 0; mode < 3; mode++ {
			var adjusted Ty
			switch mode {
			case 0:
				adjusted = u
			case 1:
				adjusted = Ref(u, false)
			case 2:
				adjusted = Ref(u, true)
			}
			mkAdjust := func() []Adjustment {
				var adj []Adjustment
				if unsizeStep >= 0 && si == unsizeStep {
					adj = adjustmentsFor(steps, si-1)
				} else {
					adj = adjustmentsFor(steps, si)
				}
				if mode > 0 {
					inner := u
					if unsizeStep >= 0 && si == unsizeStep {
						inner = steps[si-1].ty
					}
					adj = append(adj, Adjustment{Kind: AdjBorrow, Mut: mode == 2, Target: Ref(inner, mode == 2)})
					if unsizeStep >= 0 && si == unsizeStep {
						adj = append(adj, Adjustment{Kind: AdjUnsize, Target: adjusted})
					}
				} else if unsizeStep >= 0 && si == unsizeStep {
					return nil // can't unsize by value
				}
				return adj
			}
			// object candidates (dyn receivers)
			for _, c := range objCands {
				if u.Kind != TDyn {
					continue
				}
				selfTy := fcx.methodSelfTy(c.item, c.trait.Args)
				s := fcx.ic.snapshot()
				if fcx.ic.unify(selfTy, adjusted) {
					adj := mkAdjust()
					if adj == nil && mode == 0 && unsizeStep == si {
						fcx.ic.rollback(s)
						continue
					}
					return &pick{item: c.item, parent: c.trait.Args, adjust: adj, recvTy: adjusted, virtual: true}
				}
				fcx.ic.rollback(s)
			}
			// inherent
			for _, c := range inherent {
				g := fcx.t.Generics(c.impl)
				iargs := make([]Ty, len(g.Params))
				for i := range iargs {
					iargs[i] = fcx.ic.newVar()
				}
				selfTy := fcx.methodSelfTy(c.item, iargs)
				s := fcx.ic.snapshot()
				if fcx.ic.unify(selfTy, adjusted) {
					adj := mkAdjust()
					if adj == nil && unsizeStep == si && mode == 0 {
						fcx.ic.rollback(s)
						continue
					}
					return &pick{item: c.item, parent: iargs, adjust: adj, recvTy: adjusted}
				}
				fcx.ic.rollback(s)
			}
			// traits
			var found *pick
			var foundKind SelKind
			for _, c := range traitCands {
				tr := c.item.Parent
				g := fcx.t.Generics(tr)
				targs := make([]Ty, len(g.Params))
				for i := range targs {
					targs[i] = fcx.ic.newVar()
				}
				selfTy := fcx.methodSelfTy(c.item, targs)
				s := fcx.ic.snapshot()
				if !fcx.ic.unify(selfTy, adjusted) {
					fcx.ic.rollback(s)
					continue
				}
				if st := fcx.shallow(targs[0]); st.Kind == TInfer && st.Sub == 0 {
					fcx.ic.rollback(s)
					continue
				}
				k := fcx.ic.evaluate(&Predicate{Trait: &TraitRef{Trait: tr, Args: targs}}, 0)
				if k == SelNone {
					fcx.ic.rollback(s)
					continue
				}
				if found != nil {
					// prefer a definite match over an ambiguous one
					if foundKind != SelAmbig || k == SelAmbig {
						fcx.ic.rollback(s)
						continue
					}
				}
				fcx.ic.rollback(s)
				found = &pick{item: c.item, parent: targs, recvTy: adjusted}
				foundKind = k
			}
			if found != nil {
				// commit
				selfTy := fcx.methodSelfTy(found.item, found.parent)
				fcx.ic.unify(selfTy, adjusted)
				adj := mkAdjust()
				if adj == nil && unsizeStep == si && mode == 0 {
					continue
				}
				found.adjust = adj
				tr := found.item.Parent
				fcx.ic.obligations = append(fcx.ic.obligations, &Obligation{Pred: &Predicate{Trait: &TraitRef{Trait: tr, Args: found.parent}}, Span: sp})
				if st := fcx.shallow(found.parent[0]); st.Kind == TDyn && fcx.t.IsObjectMethod(found.item, st) {
					found.virtual = true
				}
				return found
			}
		}
	}
	return nil
}

// methodSelfTy returns the declared type of the method's self parameter
// with the container args substituted (own method params left as Params
// beyond the container count are not used by self types).
func (fcx *FnCtxt) methodSelfTy(item *Def, parent []Ty) Ty {
	sig := fcx.t.FnSig(item)
	g := fcx.t.Generics(item)
	args := make([]Ty, len(g.Params))
	copy(args, parent)
	for i := len(parent); i < len(args); i++ {
		args[i] = fcx.ic.newVar()
	}
	return fcx.normalizeInBody(Subst(sig.Inputs[0], args), item.Span)
}

// envMethod probes methods of traits from the body's where-clauses.
func (fcx *FnCtxt) envMethod(steps []autoderefStep, name string, sp lexer.Span) *pick {
	var envs []*TraitRef
	for _, e := range fcx.ic.env {
		for _, st := range fcx.t.AllSupertraits(e) {
			if it, ok := st.Trait.Trait.Items[name]; ok && it.Kind == DefFn && it.Fn.HasSelf {
				envs = append(envs, st)
			}
		}
	}
	if len(envs) == 0 {
		return nil
	}
	for si, st := range steps {
		u := st.ty
		if u.Kind == TInfer {
			break
		}
		for mode := 0; mode < 3; mode++ {
			adjusted := u
			if mode > 0 {
				adjusted = Ref(u, mode == 2)
			}
			for _, e := range envs {
				if e.Args[0] != u && !(mode == 0 && u.Kind == TRef) {
					continue
				}
				item := e.Trait.Trait.Items[name]
				selfTy := fcx.methodSelfTy(item, e.Args)
				s := fcx.ic.snapshot()
				if !fcx.ic.unify(selfTy, adjusted) {
					fcx.ic.rollback(s)
					continue
				}
				adj := adjustmentsFor(steps, si)
				if mode > 0 {
					adj = append(adj, Adjustment{Kind: AdjBorrow, Mut: mode == 2, Target: adjusted})
				}
				fcx.ic.obligations = append(fcx.ic.obligations, &Obligation{Pred: &Predicate{Trait: e}, Span: sp})
				return &pick{item: item, parent: e.Args, adjust: adj, recvTy: adjusted, virtual: e.Args[0].Kind == TDyn}
			}
		}
	}
	return nil
}
