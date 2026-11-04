package typeck

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
)

type methodCand struct {
	item             *Def
	impl             *Def      // inherent impl (nil for trait candidates)
	trait            *TraitRef // trait candidate (Self is a fresh var bound during matching)
	object           bool      // trait object candidate
	inherentFromStep int
}

// pick is a successful method lookup.
type pick struct {
	item    *Def
	parent  []Ty // container args (impl args or trait args incl. Self)
	adjust  []Adjustment
	recvTy  Ty // adjusted receiver type
	virtual bool
	// how the receiver was adjusted (set by lookupMethod/envMethod, see
	// genPick): autoderef step, 0 by value / 1 `&` / 2 `&mut` autoref,
	// array-to-slice unsizing
	step   int
	mode   int
	unsize bool
	probed bool
}

func (fcx *FnCtxt) checkMethodCall(x *ast.MethodCallExpr, expected Ty) Ty {
	rt := fcx.checkExpr(x.Receiver, nil)
	if fcx.resolve(rt).HasInfer() {
		// e.g. `P<?A, ?B>` from a call whose projections are pending
		fcx.selectWherePossible()
	}
	fcx.keepOpaque++
	rs := fcx.structurally(rt)
	fcx.keepOpaque--
	var early *pick
	if rs.Kind == TInfer && rs.Sub == 0 && fcx.pendingOpaque(rs) != nil {
		// a pending `impl Trait` value (its generic args are still being
		// inferred): its methods come from its declared bounds, like in
		// rustc, so later uses can still decide the args
		if p := fcx.opaqueMethod(rs, x.Method.Name, x.Span); p != nil {
			early = p
		} else if fcx.revealNumericOpaque(rs) {
			rs = fcx.structurally(rt)
		}
	}
	ambiguousNum := rs.Kind == TInfer && rs.Sub != 0
	if rs.Kind == TInfer && rs.Sub != 0 {
		// `{integer}`/`{float}` receiver: a trait method (e.g. Ord::max)
		// applies without fixing the type; inherent ones need the default
		s := fcx.ic.snapshot()
		fcx.intoIterSkip = x.Method.Span.Edition()
		p := fcx.lookupMethod(rt, x.Method.Name, x.Span)
		fcx.intoIterSkip = ""
		if p != nil && p.item.InTrait() && !p.virtual {
			early = p
		} else {
			fcx.ic.rollback(s)
		}
	}
	if early == nil && rs.Kind == TInfer {
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
	p := early
	if p == nil {
		// in an instance of a generic body: the generic body's pick
		if gp := fcx.instPick(x.ID); gp != nil {
			p = fcx.replayPick(rt, gp, x.Span)
		}
	}
	if p == nil {
		fcx.methodNode = x.ID
		fcx.privMethod = nil
		fcx.checkPriv = x.Method.Hyg == 0
		fcx.intoIterSkip = x.Method.Span.Edition()
		p = fcx.lookupMethod(rt, x.Method.Name, x.Span)
		fcx.checkPriv, fcx.intoIterSkip = false, ""
		fcx.methodNode = 0
	}
	if p != nil && early == nil && ambiguousNum && !p.item.InTrait() && x.Method.Hyg == 0 && fcx.def != nil && !fcx.t.isStd(fcx.def) {
		// rustc does not pick a numeric type for inherent methods
		num := "{integer}"
		if rs.Sub == 2 {
			num = "{float}"
		}
		fcx.t.ErrorCode(x.Method.Span, "E0689", "can't call method `%s` on ambiguous numeric type `%s`", x.Method.Name, num)
	}
	if p == nil {
		if fcx.privMethod != nil {
			fcx.t.ErrorCode(x.Method.Span, "E0624", "method `%s` is private", x.Method.Name)
		} else {
			fcx.polyNoItem(rt, x.Method.Name, false)
			fcx.errorf(x.Method.Span, "no method named `%s` found for `%s`", x.Method.Name, fcx.resolve(rt))
		}
		for _, a := range x.Args {
			fcx.checkExpr(a, nil)
		}
		return TyErr
	}
	if early == nil {
		fcx.recordPick(x.ID, p)
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
	// like calls: argument expectations from the expected result type
	fcx.nextHints = fcx.expectedInputs(ins, out, expected)
	fcx.checkArgs(x.Args, ins, false, x.Span)
	if fcx.resolve(out).HasInfer() {
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
				if fcx.checkPriv && !fcx.t.assocItemAccessible(item, fcx.curMod()) {
					// inaccessible methods are not candidates (E0624
					// if nothing else is found)
					fcx.privMethod = item
					continue
				}
				seenImpl[impl] = true
				inherent = append(inherent, methodCand{item: item, impl: impl})
			}
		}
	}
	var traitCands []methodCand
	inScope := fcx.curMod().TraitsInScope(fcx.t.R.PreludeOf(fcx.curMod().Crate))
	// the trait being implemented is in scope inside its impl
	var implTrait *Def
	for d := fcx.def; d != nil; d = d.Parent {
		if d.Kind == DefImpl {
			if d.Impl.TraitRef != nil {
				implTrait = d.Impl.TraitRef.Trait
			}
			break
		}
	}
	for _, tr := range fcx.t.traitsByItem[name] {
		if !inScope[tr] && tr != implTrait {
			continue
		}
		if fcx.unstableFnMethod(tr) {
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
					return &pick{item: c.item, parent: c.trait.Args, adjust: adj, recvTy: adjusted, virtual: true, step: si, mode: mode, unsize: si == unsizeStep, probed: true}
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
					return &pick{item: c.item, parent: iargs, adjust: adj, recvTy: adjusted, step: si, mode: mode, unsize: si == unsizeStep, probed: true}
				}
				fcx.ic.rollback(s)
			}
			// traits
			var found *pick
			var foundKind SelKind
			for _, c := range traitCands {
				tr := c.item.Parent
				if mode == 0 && fcx.intoIterSkip != "" && fcx.intoIterSkip < "2024" && tr == fcx.t.LangOpt("into_iterator") &&
					((u.Kind == TArray && fcx.intoIterSkip < "2021") || (isBoxTy(u) && len(u.Args) > 0 && u.Args[0].Kind == TSlice)) {
					continue
				}
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
					// several traits apply: in a monomorphized body, the
					// generic body's resolution decides (see methodTraitHints)
					if foundKind != SelAmbig && k != SelAmbig {
						if h := fcx.methodHint(); h != nil && h == tr && found.item.Parent != tr {
							fcx.ic.rollback(s)
							found = &pick{item: c.item, parent: targs, recvTy: adjusted}
							foundKind = k
							continue
						}
					}
					if foundKind == SelImpl && k == SelImpl && found.item.Parent != tr && !fcx.t.isStd(tr) && !fcx.t.isStd(found.item.Parent) &&
						fcx.def != nil && fcx.def.Crate != nil && fcx.def.Crate.Local && fcx.t.Generics(fcx.def).Count() == 0 {
						// two traits of non-std crates provide the method for
						// this receiver at the same autoref step (std traits
						// are left out: rustgo's std has methods that are
						// unstable, and so not candidates, in the real std)
						if r := fcx.resolve(adjusted); !r.HasInfer() && !r.HasParam() && !r.HasProj() {
							fcx.t.ErrorCode(sp, "E0034", "multiple applicable items in scope: `%s` is provided by traits `%s` and `%s`", name, found.item.Parent.Name, tr.Name)
						}
					}
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
				found.step, found.mode, found.unsize, found.probed = si, mode, si == unsizeStep, true
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
			if fcx.unstableFnMethod(st.Trait) {
				continue
			}
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
				item := e.Trait.Trait.Items[name]
				if e.Args[0] != u && !(mode == 0 && u.Kind == TRef) {
					// a smart-pointer receiver (`self: Pin<&mut Self>`,
					// `Box<Self>`, `Rc<Self>`) wrapping the bound type:
					// the step is the declared self type itself
					if mode != 0 || u.Kind != TAdt {
						continue
					}
					if st := fcx.shallow(fcx.methodSelfTy(item, e.Args)); st.Kind != TAdt || st.Def != u.Def {
						continue
					}
				}
				// an earlier autoderef step that itself implements the
				// trait wins (`it: &mut I` → `<&mut I as Iterator>::take`,
				// not a move out of `*it`)
				if si > 0 && fcx.earlierStepImplements(steps[:si], e) {
					return nil
				}
				if len(e.Args) > 1 {
					// several where-clauses of the trait with different
					// args (`T: Into<A> + Into<B>`) leave its args to
					// inference (the expected type), like rustc's
					// param-env candidates; in monomorphized bodies too
					for _, e2 := range envs {
						if e2.Trait == e.Trait && e2.Args[0] == e.Args[0] && !sameArgs(e2.Args, e.Args) {
							args := append([]Ty{e.Args[0]}, make([]Ty, len(e.Args)-1)...)
							for i := 1; i < len(args); i++ {
								args[i] = fcx.ic.newVar()
							}
							e = &TraitRef{Trait: e.Trait, Args: args}
							break
						}
					}
				}
				if len(e.Args) > 1 && fcx.envContradictsHint(envs, e) {
					// another where-clause of the trait for this type is
					// the one the generic body used
					continue
				}
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
				return &pick{item: item, parent: e.Args, adjust: adj, recvTy: adjusted, virtual: e.Args[0].Kind == TDyn, step: si, mode: mode, probed: true}
			}
		}
	}
	return nil
}

// unstableFnMethod reports whether tr is one of the Fn traits, whose
// methods (`call`, `call_mut`, `call_once`) are unstable: outside std they
// are not method candidates (`f.call(x)` finds a user trait's `call`).
func (fcx *FnCtxt) unstableFnMethod(tr *Def) bool {
	switch tr.LangItem {
	case "fn", "fn_mut", "fn_once":
		return !fcx.t.isStd(fcx.def)
	}
	return false
}

// envContradictsHint reports whether the where-clause e is one of several
// of its trait for the same self type (`T: Into<u64>` and the bound
// `type Unit: Into<f64>` when both are `u32: Into<..>` here) and the
// generic body's call used other (parameter-free) trait args.
func (fcx *FnCtxt) envContradictsHint(envs []*TraitRef, e *TraitRef) bool {
	several := false
	for _, e2 := range envs {
		if e2.Trait == e.Trait && e2.Args[0] == e.Args[0] && !sameArgs(e2.Args, e.Args) {
			several = true
			break
		}
	}
	if !several || fcx.methodNode == 0 || len(fcx.inst.Args) == 0 || fcx.t.quiet > 0 {
		return false
	}
	fcx.t.methodTraitHints(fcx.def)
	h := fcx.t.methodHintArgs[fcx.def][fcx.methodNode]
	if h == nil || len(h) != len(e.Args) || fcx.t.methodHints[fcx.def][fcx.methodNode] != e.Trait {
		return false
	}
	for i := 1; i < len(h); i++ {
		if !h[i].HasParam() && !h[i].HasProj() && !h[i].HasInfer() && !h[i].HasError() && h[i] != e.Args[i] {
			return true
		}
	}
	return false
}

// earlierStepImplements reports whether one of the autoderef steps
// implements e's trait (with e's other trait args).
func (fcx *FnCtxt) earlierStepImplements(steps []autoderefStep, e *TraitRef) bool {
	for _, st := range steps {
		if st.ty.Kind == TInfer {
			return false
		}
		args := append([]Ty{st.ty}, e.Args[1:]...)
		if k := fcx.ic.evaluate(&Predicate{Trait: &TraitRef{Trait: e.Trait, Args: args}}, 0); k == SelImpl || k == SelBuiltin {
			return true
		}
	}
	return false
}

// methodHint returns the trait the generic body resolved the current method
// call to (nil outside monomorphized generic bodies or if unknown).
func (fcx *FnCtxt) methodHint() *Def {
	if fcx.methodNode == 0 || len(fcx.inst.Args) == 0 || fcx.t.quiet > 0 {
		return nil
	}
	return fcx.t.methodTraitHints(fcx.def)[fcx.methodNode]
}

// opaqueMethod looks up a method of the pending `impl Trait` value v
// through the declared bounds of its opaque type: the probe runs on the
// opaque type, and the pick is then rebuilt with v as the receiver.
func (fcx *FnCtxt) opaqueMethod(v Ty, name string, sp lexer.Span) *pick {
	o := fcx.pendingOpaque(v)
	if o == nil {
		return nil
	}
	op := Opaque(o.Proj.Def, fcx.ic.resolveList(o.Proj.Args), o.Proj.Index)
	s := fcx.ic.snapshot()
	p := fcx.lookupMethod(op, name, sp)
	fcx.ic.rollback(s)
	if p == nil || !p.item.InTrait() || p.virtual {
		return nil
	}
	recv := v
	var adj []Adjustment
	for _, a := range p.adjust {
		if a.Kind != AdjBorrow {
			return nil
		}
		recv = Ref(v, a.Mut)
		adj = append(adj, Adjustment{Kind: AdjBorrow, Mut: a.Mut, Target: recv})
	}
	tr := p.item.Parent
	g := fcx.t.Generics(tr)
	targs := make([]Ty, len(g.Params))
	for i := range targs {
		targs[i] = fcx.ic.newVar()
	}
	if !fcx.ic.unify(fcx.methodSelfTy(p.item, targs), recv) {
		return nil
	}
	fcx.ic.obligations = append(fcx.ic.obligations, &Obligation{Pred: &Predicate{Trait: &TraitRef{Trait: tr, Args: targs}}, Span: sp})
	return &pick{item: p.item, parent: targs, adjust: adj, recvTy: recv}
}
