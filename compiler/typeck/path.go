package typeck

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
	"rustgo/compiler/resolve"
)

func (fcx *FnCtxt) resolvePath(p *ast.Path, ns Namespace) (resolve.PathResult, error) {
	fcx.ac.scope = fcx.curMod()
	pr, err := fcx.t.R.ResolvePath(fcx.curMod(), pathSegs(p), p.Global, ns, fcx.ac.firstLookup)
	if err == nil {
		fcx.t.checkGlobAmbiguity(p, fcx.curMod(), ns, fcx.ac.firstLookup)
	}
	if err == nil && pr.Private {
		fcx.t.reportPrivatePath(p, pr)
	}
	return pr, err
}

// ownArgs lowers explicit generic args for d's own params (fresh vars for
// missing ones) given parent args.
func (fcx *FnCtxt) ownArgs(d *Def, seg *ast.PathSegment, parent []Ty, sp lexer.Span) []Ty {
	fcx.ac.scope = fcx.curMod()
	fcx.ac.exprCtx = true
	args := fcx.ac.genericArgsFor(d, seg, parent, sp, nil)
	fcx.ac.exprCtx = false
	for i, a := range args {
		args[i] = fcx.normalizeInBody(a, sp)
	}
	return args
}

// fnDefArgsFromParent builds args for an assoc fn of an impl/trait.
func (fcx *FnCtxt) checkPathExpr(x *ast.PathExpr, expected Ty) Ty {
	vr, ty := fcx.resolveValuePath(x.QSelf, x.Path, x.Span, false, expected)
	if vr == nil {
		return TyErr
	}
	fcx.r.Paths[x.ID] = vr
	if vr.Kind == VLocal {
		fcx.noteLocalUse(vr.Local, x)
	}
	return ty
}

// resolveValuePath resolves a path in value position. For patterns, locals
// are not considered.
func (fcx *FnCtxt) resolveValuePath(qself *ast.QSelf, p *ast.Path, sp lexer.Span, pattern bool, expected Ty) (*ValueRes, Ty) {
	segs := p.Segments
	if qself == nil && !pattern && len(segs) == 1 && segs[0].Args == nil && !p.Global {
		if l := fcx.lookupLocal(segs[0].Ident.Name, segs[0].Ident.Hyg); l != nil {
			return &ValueRes{Kind: VLocal, Local: l}, l.Ty
		}
	}
	if qself != nil {
		fcx.ac.scope = fcx.curMod()
		self := fcx.lowerTy(qself.Ty)
		if qself.Position == 0 {
			if len(segs) != 1 {
				fcx.errorf(sp, "unsupported qualified path")
				return nil, TyErr
			}
			return fcx.typeRelativeValue(self, segs[0], sp, expected)
		}
		traitPath := &ast.Path{Global: p.Global, Segments: segs[:qself.Position], Span: p.Span}
		tr, _ := fcx.ac.lowerTraitPath(traitPath, self, sp)
		if tr == nil {
			return nil, TyErr
		}
		for i, a := range tr.Args {
			tr.Args[i] = fcx.normalizeInBody(a, sp)
		}
		rest := segs[qself.Position:]
		if len(rest) != 1 {
			fcx.errorf(sp, "unsupported qualified path")
			return nil, TyErr
		}
		return fcx.traitItemValue(tr, rest[0], sp)
	}
	pr, err := fcx.resolvePath(p, ValueNS)
	if err != nil {
		fcx.errorf(sp, "%v", err)
		return nil, TyErr
	}
	res := pr.Res
	if pr.Consumed == len(segs) {
		return fcx.valueFromRes(res, segs, sp, expected)
	}
	// type-relative
	last := segs[len(segs)-1]
	baseSeg := segs[pr.Consumed-1]
	if len(segs)-pr.Consumed > 1 {
		// T::Assoc::item: resolve the associated types first
		if res.Kind == ResDef && res.Def.Kind == DefTrait {
			fcx.errorf(sp, "unsupported path `%s`", ast.PathString(p))
			return nil, TyErr
		}
		fcx.ac.scope = fcx.curMod()
		fcx.ac.exprCtx = res.Kind == ResDef
		base := fcx.ac.resToTy(res, baseSeg, sp)
		fcx.ac.exprCtx = false
		base = fcx.ac.assocTypeOn(base, segs[pr.Consumed:len(segs)-1], sp)
		return fcx.typeRelativeValue(fcx.normalizeInBody(base, sp), last, sp, expected)
	}
	if res.Kind == ResDef && res.Def.Kind == DefTrait {
		g := fcx.t.Generics(res.Def)
		self := fcx.ic.newVar()
		var args []Ty
		if baseSeg.Args != nil {
			fcx.ac.scope = fcx.curMod()
			tr, _ := fcx.ac.lowerTraitSeg(res.Def, baseSeg, self, sp)
			args = tr.Args
		} else {
			// omitted trait params are inferred in expressions
			args = []Ty{self}
			for len(args) < len(g.Params) {
				args = append(args, fcx.ic.newVar())
			}
		}
		return fcx.traitItemValue(&TraitRef{Trait: res.Def, Args: args}, last, sp)
	}
	fcx.ac.scope = fcx.curMod()
	// omitted defaulted params of a named type are inferred
	// (`HashMap::with_hasher(..)`), but `Self` keeps the impl's types
	fcx.ac.exprCtx = res.Kind == ResDef
	baseRaw := fcx.ac.resToTy(res, baseSeg, sp)
	fcx.ac.exprCtx = false
	base := fcx.normalizeInBody(baseRaw, sp)
	// `T::item` with T a type parameter (or Self in a trait): only T's
	// bounds apply, even if the substituted type has an inherent item
	// of that name (`F::from_bits` with F = f32)
	fcx.relBaseIsParam = (res.Kind == ResDef && res.Def != nil && res.Def.Kind == DefTyParam) ||
		(res.Kind == ResSelfTy && res.Def != nil && res.Def.Kind == DefTrait)
	defer func() { fcx.relBaseIsParam = false }()
	return fcx.typeRelativeValue(base, last, sp, expected)
}

func (fcx *FnCtxt) valueFromRes(res Res, segs []*ast.PathSegment, sp lexer.Span, expected Ty) (*ValueRes, Ty) {
	last := segs[len(segs)-1]
	switch res.Kind {
	case ResDef:
		d := res.Def
		switch d.Kind {
		case DefFn:
			var parent []Ty
			if d.IsAssoc() {
				fcx.errorf(sp, "cannot refer to associated function `%s` without its type", d.Name)
				return nil, TyErr
			}
			args := fcx.ownArgs(d, last, parent, sp)
			return &ValueRes{Kind: VFn, Def: d, Args: args}, FnDefTy(d, args)
		case DefConst:
			return &ValueRes{Kind: VConst, Def: d}, fcx.normalizeInBody(fcx.t.ConstTy(d), sp)
		case DefStatic:
			return &ValueRes{Kind: VStatic, Def: d}, fcx.normalizeInBody(fcx.t.ConstTy(d), sp)
		case DefConstParam:
			gp := fcx.t.paramOf[d]
			v := fcx.inst.Args[gp.Index]
			ty := fcx.normalizeInBody(fcx.t.Generics(gp.Owner).Params[gp.Index].ConstTy, sp)
			if ty == nil {
				ty = TyUsize
			}
			return &ValueRes{Kind: VConstParam, Value: v.Val}, Subst(ty, fcx.inst.Args)
		}
		fcx.errorf(sp, "expected value, found %s `%s`", d.Kind, d.Name)
		return nil, TyErr
	case ResCtor:
		return fcx.ctorValue(res.Def, segs, sp, expected)
	case ResSelfCtor:
		d := res.Def
		var self Ty
		if d.Kind == DefImpl {
			fcx.t.ImplHeader(d)
			self = fcx.normalizeInBody(Subst(d.Impl.SelfTy, fcx.inst.Args), sp)
		} else if d.Kind == DefStruct {
			self = Adt(d, fcx.inst.Args)
		}
		s := fcx.shallow(self)
		if s.Kind == TAdt && s.Def.Adt.Kind == AdtStruct {
			fcx.t.AdtFields(s.Def)
			v := s.Def.Adt.Variants[0]
			if v.Shape == ast.ShapeTuple {
				return &ValueRes{Kind: VFn, Def: s.Def, Args: s.Args}, FnDefTy(s.Def, s.Args)
			}
			if v.Shape == ast.ShapeUnit {
				return &ValueRes{Kind: VUnitCtor, Def: s.Def, Args: s.Args}, s
			}
		}
		fcx.errorf(sp, "`Self` is not a tuple or unit struct here")
		return nil, TyErr
	case ResLocal:
	}
	fcx.errorf(sp, "expected value")
	return nil, TyErr
}

// ctorValue handles paths to struct/variant constructors.
func (fcx *FnCtxt) ctorValue(d *Def, segs []*ast.PathSegment, sp lexer.Span, expected Ty) (*ValueRes, Ty) {
	adt := d
	if d.Kind == DefVariant {
		adt = d.Parent
	}
	fcx.t.AdtFields(adt)
	// generic args may be on the type segment (Option::<T>::None) or the
	// variant segment (None::<T>).
	var seg *ast.PathSegment
	last := segs[len(segs)-1]
	if last.Args != nil {
		seg = last
	} else if d.Kind == DefVariant && len(segs) >= 2 && segs[len(segs)-2].Args != nil {
		seg = segs[len(segs)-2]
	} else if d.Kind == DefStruct {
		seg = last
	}
	args := fcx.ownArgs(adt, seg, nil, sp)
	return fcx.ctorWithArgs(d, args)
}

func (fcx *FnCtxt) ctorWithArgs(d *Def, args []Ty) (*ValueRes, Ty) {
	adt := d
	if d.Kind == DefVariant {
		adt = d.Parent
	}
	fcx.t.AdtFields(adt)
	v := variantOf(d)
	if v.Shape == ast.ShapeUnit {
		return &ValueRes{Kind: VUnitCtor, Def: d, Args: args}, Adt(adt, args)
	}
	return &ValueRes{Kind: VFn, Def: d, Args: args}, FnDefTy(d, args)
}

// typeRelativeValue resolves `Base::name` in value position.
func (fcx *FnCtxt) typeRelativeValue(base Ty, seg *ast.PathSegment, sp lexer.Span, expected Ty) (*ValueRes, Ty) {
	name := seg.Ident.Name
	b := fcx.structurally(base)
	// enum variants
	if b.Kind == TAdt && b.Def.Adt.Kind == AdtEnum {
		if bm := b.Def.Mod.Lookup(name, ValueNS); bm != nil && bm.Res.Kind == ResCtor {
			return fcx.ctorWithArgs(bm.Res.Def, b.Args)
		}
		if bm := b.Def.Mod.Lookup(name, TypeNS); bm != nil && bm.Res.Def != nil && bm.Res.Def.Kind == DefVariant {
			fcx.errorf(sp, "struct variant `%s` used as a value", name)
			return nil, TyErr
		}
	}
	if fcx.relBaseIsParam {
		fcx.relBaseIsParam = false
		if v, t, ok := fcx.envTraitItem(b, seg, sp); ok {
			return v, t
		}
	}
	// inherent impls
	fcx.checkPriv, fcx.privMethod = seg.Ident.Hyg == 0, nil
	item, implArgs := fcx.findInherentItem(b, name, false)
	privItem := fcx.privMethod
	fcx.checkPriv, fcx.privMethod = false, nil
	if item != nil {
		switch item.Kind {
		case DefFn:
			args := fcx.ownArgs(item, seg, implArgs, sp)
			return &ValueRes{Kind: VFn, Def: item, Args: args}, FnDefTy(item, args)
		case DefConst:
			return &ValueRes{Kind: VConst, Def: item, Args: implArgs}, fcx.normalizeInBody(Subst(fcx.t.ConstTy(item), implArgs), sp)
		}
	}
	// trait items
	if b.Kind == TInfer && b.Sub == 0 {
		fcx.errorf(sp, "type annotations needed to resolve `%s`", name)
		return nil, TyErr
	}
	// where-clauses on this type first (`E::from_error_kind(..)` with
	// `E: ParseError<I>` fixes the trait args; in monomorphized bodies the
	// env holds the substituted bounds)
	if v, t, ok := fcx.envTraitItem(b, seg, sp); ok {
		return v, t
	}
	var best *TraitRef
	var bestKind SelKind
	inScope := fcx.curMod().TraitsInScope(fcx.t.R.Prelude)
	for _, tr := range fcx.t.traitsByItem[name] {
		it := tr.Trait.Items[name]
		if it.Kind != DefFn && it.Kind != DefConst {
			continue
		}
		// like methods, `Type::item` only sees traits in scope
		if !inScope[tr] && !fcx.traitInScopeOfImpl(tr) {
			continue
		}
		g := fcx.t.Generics(tr)
		args := make([]Ty, len(g.Params))
		args[0] = b
		for i := 1; i < len(args); i++ {
			args[i] = fcx.ic.newVar()
		}
		ref := &TraitRef{Trait: tr, Args: args}
		k := fcx.ic.evaluate(&Predicate{Trait: ref}, 0)
		if k == SelNone {
			continue
		}
		if best == nil || (bestKind == SelAmbig && k != SelAmbig) {
			best = ref
			bestKind = k
		}
	}
	if best != nil {
		return fcx.traitItemValue(best, seg, sp)
	}
	if privItem != nil {
		what := "associated function"
		if privItem.Kind == DefConst {
			what = "associated constant"
		} else if privItem.Fn != nil && privItem.Fn.HasSelf {
			what = "method"
		}
		fcx.t.ErrorCode(seg.Ident.Span, "E0624", "%s `%s` is private", what, name)
		return nil, TyErr
	}
	fcx.errorf(sp, "no function or associated item named `%s` found for `%s`", name, fcx.resolve(base))
	return nil, TyErr
}

// findInherentItem finds an inherent assoc item on type b; returns the item
// and the impl args (committed unification).
func (fcx *FnCtxt) findInherentItem(b Ty, name string, needSelf bool) (*Def, []Ty) {
	key := SimplifyKey(b)
	if key == "" {
		return nil, nil
	}
	for _, impl := range fcx.t.inherent[key] {
		item, ok := impl.Impl.Items[name]
		if !ok || item.Kind == DefAssocTy {
			continue
		}
		if needSelf && (item.Kind != DefFn || !item.Fn.HasSelf) {
			continue
		}
		if fcx.checkPriv && !fcx.t.assocItemAccessible(item, fcx.curMod()) {
			fcx.privMethod = item
			continue
		}
		g := fcx.t.Generics(impl)
		args := make([]Ty, len(g.Params))
		for i := range args {
			args[i] = fcx.ic.newVar()
		}
		s := fcx.ic.snapshot()
		if fcx.ic.unify(Subst(impl.Impl.SelfTy, args), b) {
			return item, args
		}
		fcx.ic.rollback(s)
	}
	return nil, nil
}

// opaqueFnSig returns the call signature of an inference variable standing
// for a pending `impl Fn*(..) -> R` value.
func (fcx *FnCtxt) opaqueFnSig(v Ty) ([]Ty, Ty, bool) {
	ic := fcx.ic
	fnOnce := fcx.t.LangOpt("fn_once")
	if fnOnce == nil {
		return nil, nil, false
	}
	for _, o := range ic.obligations {
		if o.Done || o.Proj == nil || o.Proj.Kind != TOpaque || ic.shallow(o.Var) != v {
			continue
		}
		op := Opaque(o.Proj.Def, ic.resolveList(o.Proj.Args), o.Proj.Index)
		b := ic.opaqueBound(op, fnOnce)
		if b == nil || len(b.Trait.Args) < 2 {
			continue
		}
		a := ic.shallow(b.Trait.Args[1])
		if a.Kind != TTuple {
			continue
		}
		out := TyUnit
		for _, bd := range b.Bindings {
			if bd.Name == "Output" {
				out = bd.Ty
			}
		}
		return a.Elems, out, true
	}
	return nil, nil, false
}

// traitItemValue resolves Trait::name with the given trait ref.
func (fcx *FnCtxt) traitItemValue(tr *TraitRef, seg *ast.PathSegment, sp lexer.Span) (*ValueRes, Ty) {
	name := seg.Ident.Name
	for _, st := range fcx.t.AllSupertraits(tr) {
		item, ok := st.Trait.Trait.Items[name]
		if !ok {
			continue
		}
		fcx.ic.obligations = append(fcx.ic.obligations, &Obligation{Pred: &Predicate{Trait: st}, Span: sp})
		switch item.Kind {
		case DefFn:
			args := fcx.ownArgs(item, seg, st.Args, sp)
			return &ValueRes{Kind: VFn, Def: item, Args: args}, FnDefTy(item, args)
		case DefConst:
			return &ValueRes{Kind: VConst, Def: item, Args: st.Args}, fcx.normalizeInBody(Subst(fcx.t.ConstTy(item), st.Args), sp)
		}
	}
	fcx.errorf(sp, "no associated item `%s` in trait `%s`", name, tr.Trait.Name)
	return nil, TyErr
}

// ---------------------------------------------------------------------------
// Calls

func (fcx *FnCtxt) checkCall(x *ast.CallExpr, expected Ty) Ty {
	ct := fcx.checkExpr(x.Fn, nil)
	steps := fcx.autoderef(ct, x.Span)
	for i, st := range steps {
		t := st.ty
		switch t.Kind {
		case TFnDef:
			fcx.addAdjust(x.Fn, adjustmentsFor(steps, i)...)
			d := t.Def
			ins, out := fcx.instantiateSig(d, t.Args, x.Span)
			kind := CallFn
			if d.Kind == DefStruct || d.Kind == DefVariant {
				kind = CallCtor
			} else {
				fcx.registerPredicates(d, t.Args, x.Span)
			}
			if expected != nil && !out.HasError() {
				// use the expectation to guide inference of generic returns
				expVar := fcx.shallow(expected).Kind == TInfer // before unify resolves it
				s := fcx.ic.snapshot()
				if expVar || !fcx.ic.unify(out, expected) {
					fcx.ic.rollback(s)
				} else {
					es := fcx.shallow(expected)
					// only keep if no coercion could be intended
					if es.Kind == TRef || es.Kind == TAdt && isUnsizableSmartPtr(es.Def) || es.Kind == TFnPtr {
						fcx.ic.rollback(s)
					}
				}
			}
			variadic := false
			if d.Kind == DefFn {
				variadic = fcx.t.FnSig(d).Variadic
			}
			fcx.nextHints = fcx.expectedInputs(ins, out, expected)
			fcx.checkArgs(x.Args, ins, variadic, x.Span)
			callee := &Callee{Kind: kind, Def: d, Args: t.Args}
			fcx.r.Callees[x.ID] = callee
			return out
		case TFnPtr:
			fcx.addAdjust(x.Fn, adjustmentsFor(steps, i)...)
			fcx.checkArgs(x.Args, t.Elems, t.Sub == 1, x.Span)
			fcx.r.Callees[x.ID] = &Callee{Kind: CallFnPtr, Ty: t}
			return t.Ret
		case TClosure:
			ci := fcx.t.Closures[t]
			if ci == nil {
				fcx.errorf(x.Span, "closure called before its definition was checked")
				return TyErr
			}
			fcx.addAdjust(x.Fn, adjustmentsFor(steps, i)...)
			fcx.checkArgs(x.Args, ci.Inputs, false, x.Span)
			fcx.r.Callees[x.ID] = &Callee{Kind: CallClosure, Ty: t}
			return ci.Output
		case TDyn:
			if t.Def != nil && (t.Def.LangItem == "fn" || t.Def.LangItem == "fn_mut" || t.Def.LangItem == "fn_once") {
				fcx.addAdjust(x.Fn, adjustmentsFor(steps, i)...)
				var ins []Ty
				if len(t.Args) > 0 {
					a := fcx.shallow(t.Args[0])
					if a.Kind == TTuple {
						ins = a.Elems
					}
				}
				out := TyUnit
				for _, b := range t.Bindings {
					if b.Name == "Output" {
						out = b.Ty
					}
				}
				fcx.checkArgs(x.Args, ins, false, x.Span)
				fcx.r.Callees[x.ID] = &Callee{Kind: CallDynFn, Ty: t}
				return out
			}
		case TInfer:
			// an `impl Fn*` value whose hidden type is not revealed yet
			// (its generic args are still being inferred): call through
			// its declared bound; the callee kind is fixed in writeback
			if ins, out, ok := fcx.opaqueFnSig(t); ok && i == 0 {
				fcx.checkArgs(x.Args, ins, false, x.Span)
				fcx.r.Callees[x.ID] = &Callee{Kind: CallClosure, Ty: t}
				return out
			}
			fcx.errorf(x.Span, "type annotations needed: cannot call value of unknown type")
			return TyErr
		}
	}
	fcx.errorf(x.Span, "expected function, found `%s`", fcx.resolve(ct))
	for _, a := range x.Args {
		fcx.checkExpr(a, nil)
	}
	return TyErr
}

// checkArgs checks call arguments against parameter types (closures last).
func (fcx *FnCtxt) checkArgs(args []ast.Expr, ins []Ty, variadic bool, sp lexer.Span) {
	hints := fcx.nextHints
	fcx.nextHints = nil
	saved := fcx.argHints
	fcx.argHints = hints
	defer func() { fcx.argHints = saved }()
	if len(args) != len(ins) && !(variadic && len(args) >= len(ins)) {
		fcx.errorf(sp, "this function takes %d arguments but %d were supplied", len(ins), len(args))
	}
	for pass := 0; pass < 2; pass++ {
		for i, a := range args {
			_, isClosure := unparen(a).(*ast.ClosureExpr)
			if (pass == 0) == isClosure {
				continue
			}
			if pass == 1 {
				fcx.selectWherePossible()
			}
			if i < len(ins) {
				if ce, ok := unparen(a).(*ast.ClosureExpr); ok && i < len(fcx.argHints) && fcx.argHints[i] != nil {
					if eins, _ := fcx.expectedClosureSig(ins[i]); eins == nil {
						if hins, _ := fcx.expectedClosureSig(fcx.argHints[i]); hins != nil {
							ct := fcx.checkClosure(ce, fcx.argHints[i])
							fcx.record(ce.ID, ct)
							if a != ce {
								fcx.record(a.GetID(), ct)
							}
							if !fcx.coerce(a, ct, ins[i]) {
								fcx.errorf(a.GetSpan(), "mismatched types: expected `%s`, found `%s`", fcx.resolve(ins[i]), fcx.resolve(ct))
							}
							continue
						}
					}
				}
				if fcx.coerceToHint(a, i, ins[i]) {
					continue
				}
				if ce := blockTailClosure(a); ce != nil && i < len(fcx.argHints) && fcx.argHints[i] != nil {
					if eins, _ := fcx.expectedClosureSig(ins[i]); eins == nil {
						if hins, _ := fcx.expectedClosureSig(fcx.argHints[i]); hins != nil {
							if fcx.closureHints == nil {
								fcx.closureHints = map[ast.NodeID]Ty{}
							}
							fcx.closureHints[ce.ID] = fcx.argHints[i]
						}
					}
				}
				fcx.checkExprCoerce(a, ins[i])
			} else {
				fcx.checkExpr(a, nil)
			}
		}
	}
}

// blockTailClosure returns the closure at the tail of a block expression
// (through nested blocks), or nil.
func blockTailClosure(e ast.Expr) *ast.ClosureExpr {
	b, ok := unparen(e).(*ast.BlockExpr)
	if !ok || b.Async {
		return nil
	}
	for {
		t := blockTail(b.Block)
		if t == nil {
			return nil
		}
		switch x := unparen(t).(type) {
		case *ast.ClosureExpr:
			return x
		case *ast.BlockExpr:
			if x.Async {
				return nil
			}
			b = x
		default:
			return nil
		}
	}
}

func unparen(e ast.Expr) ast.Expr {
	for {
		p, ok := e.(*ast.ParenExpr)
		if !ok {
			return e
		}
		e = p.X
	}
}

// ---------------------------------------------------------------------------
// Struct literals

// resolveStructPath resolves the path of a struct literal / struct pattern
// to a variant and the ADT type.
func (fcx *FnCtxt) resolveStructPath(qself *ast.QSelf, p *ast.Path, sp lexer.Span) (*VariantDef, Ty) {
	var base Ty
	var variantName *ast.PathSegment
	if qself != nil {
		base = fcx.lowerTy(qself.Ty)
		if len(p.Segments) == 1 {
			variantName = p.Segments[0]
		}
	} else {
		pr, err := fcx.resolvePath(p, TypeNS)
		if err != nil {
			fcx.errorf(sp, "%v", err)
			return nil, TyErr
		}
		segs := p.Segments
		if pr.Consumed == len(segs) {
			res := pr.Res
			if res.Kind == ResDef {
				d := res.Def
				switch d.Kind {
				case DefVariant:
					var seg *ast.PathSegment
					if len(segs) >= 2 && segs[len(segs)-2].Args != nil {
						seg = segs[len(segs)-2]
					} else if segs[len(segs)-1].Args != nil {
						seg = segs[len(segs)-1]
					}
					args := fcx.ownArgs(d.Parent, seg, nil, sp)
					fcx.t.AdtFields(d.Parent)
					return d.Variant, Adt(d.Parent, args)
				}
			}
			fcx.ac.scope = fcx.curMod()
			fcx.ac.exprCtx = res.Kind == ResDef // `S { .. }` infers omitted params
			bt := fcx.ac.resToTy(res, segs[len(segs)-1], sp)
			fcx.ac.exprCtx = false
			base = fcx.normalizeInBody(bt, sp)
		} else if len(segs)-pr.Consumed == 1 {
			fcx.ac.scope = fcx.curMod()
			base = fcx.normalizeInBody(fcx.ac.resToTy(pr.Res, segs[pr.Consumed-1], sp), sp)
			variantName = segs[len(segs)-1]
		} else {
			fcx.errorf(sp, "unsupported struct path")
			return nil, TyErr
		}
	}
	b := fcx.structurally(base)
	if b.Kind != TAdt {
		fcx.errorf(sp, "expected struct, variant or union type, found `%s`", fcx.resolve(base))
		return nil, TyErr
	}
	fcx.t.AdtFields(b.Def)
	if variantName != nil {
		if b.Def.Adt.Kind != AdtEnum {
			fcx.errorf(sp, "`%s` is not an enum", b)
			return nil, TyErr
		}
		for _, v := range b.Def.Adt.Variants {
			if v.Name == variantName.Ident.Name {
				return v, b
			}
		}
		fcx.errorf(sp, "no variant `%s` in enum `%s`", variantName.Ident.Name, b.Def.Name)
		return nil, TyErr
	}
	if b.Def.Adt.Kind == AdtEnum {
		fcx.errorf(sp, "expected struct, found enum `%s`", b.Def.Name)
		return nil, TyErr
	}
	return b.Def.Adt.Variants[0], b
}

func (fcx *FnCtxt) checkStructExpr(x *ast.StructExpr, expected Ty) Ty {
	v, adt := fcx.resolveStructPath(x.QSelf, x.Path, x.Span)
	if v == nil {
		for _, f := range x.Fields {
			fcx.checkExpr(f.Expr, nil)
		}
		return TyErr
	}
	if expected != nil {
		s := fcx.ic.snapshot()
		if !fcx.ic.unify(expected, adt) {
			fcx.ic.rollback(s)
		}
	}
	fcx.r.Variants[x.ID] = v
	seen := map[string]bool{}
	for _, f := range x.Fields {
		fi := v.FieldIndex(f.Name.Name)
		if fi < 0 {
			fcx.errorf(f.Name.Span, "struct `%s` has no field named `%s`", v.Name, f.Name.Name)
			fcx.checkExpr(f.Expr, nil)
			continue
		}
		seen[f.Name.Name] = true
		fcx.checkFieldPrivacy(f.Name, adt, v.Fields[fi], v)
		ft := fcx.normalizeInBody(Subst(v.Fields[fi].Ty, adt.Args), f.Span)
		fcx.checkExprCoerce(f.Expr, ft)
	}
	if x.Base != nil {
		if adt.Kind == TAdt && adt.Def != nil && adt.Def.Kind == DefEnum {
			fcx.t.ErrorCode(x.Base.GetSpan(), "E0436", "functional record update syntax requires a struct")
		}
		fcx.checkExprCoerce(x.Base, adt)
	} else if len(seen) < len(v.Fields) && !isUnion(adt) {
		for _, f := range v.Fields {
			if !seen[f.Name] {
				fcx.errorf(x.Span, "missing field `%s` in initializer of `%s`", f.Name, v.Name)
				break
			}
		}
	}
	return adt
}

func isUnion(t Ty) bool { return t.Kind == TAdt && t.Def.Adt.Kind == AdtUnion }

// coerceToHint handles an argument whose formal type still has inference
// variables but whose expectation (from the expected return type) is fully
// known: like rustc, the argument is coerced to the expected input type and
// that type is then required to equal the formal one (e.g. `from_ref(&box)`
// with expected `&[H]` deref-coerces `&Box<H>` to `&H`). Falls back to the
// formal type when that fails. Reports whether the argument was handled.
func (fcx *FnCtxt) coerceToHint(a ast.Expr, i int, formal Ty) bool {
	if i >= len(fcx.argHints) || fcx.argHints[i] == nil {
		return false
	}
	h := fcx.resolve(fcx.argHints[i])
	if h.HasInfer() || h.HasError() || h.IsUnsized() || !fcx.resolve(formal).HasInfer() {
		return false
	}
	switch unparen(a).(type) {
	case *ast.BlockExpr, *ast.IfExpr, *ast.MatchExpr:
		return false
	}
	ty := fcx.checkExpr(a, h)
	s := fcx.ic.snapshot()
	prev := append([]Adjustment(nil), fcx.r.Adjust[a.GetID()]...)
	if fcx.coerce(a, ty, h) && fcx.ic.unify(formal, h) {
		return true
	}
	fcx.ic.rollback(s)
	fcx.r.Adjust[a.GetID()] = prev
	if !fcx.coerce(a, ty, formal) {
		fcx.errorf(a.GetSpan(), "mismatched types: expected `%s`, found `%s`", fcx.resolve(formal), fcx.resolve(ty))
	}
	return true
}

// expectedInputs derives hint types for arguments from the expected
// result type (without committing inference).
func (fcx *FnCtxt) expectedInputs(ins []Ty, out, expected Ty) []Ty {
	if expected == nil {
		return nil
	}
	s := fcx.ic.snapshot()
	defer fcx.ic.rollback(s)
	if !fcx.ic.unify(out, expected) {
		return nil
	}
	return fcx.ic.resolveList(ins)
}

// envTraitItem resolves `T::name` through a unique where-clause on T (in
// monomorphized bodies the env holds the substituted bounds).
func (fcx *FnCtxt) envTraitItem(b Ty, seg *ast.PathSegment, sp lexer.Span) (*ValueRes, Ty, bool) {
	name := seg.Ident.Name
	var envRef *TraitRef
	for _, e := range fcx.ic.env {
		if fcx.resolve(e.Args[0]) != fcx.resolve(b) {
			continue
		}
		for _, st := range fcx.t.AllSupertraits(e) {
			if it, ok := st.Trait.Trait.Items[name]; ok && (it.Kind == DefFn || it.Kind == DefConst) {
				if envRef != nil && !sameArgs(envRef.Args, st.Args) {
					return nil, nil, false
				}
				envRef = st
			}
		}
	}
	if envRef == nil {
		return nil, nil, false
	}
	v, t := fcx.traitItemValue(&TraitRef{Trait: envRef.Trait, Args: append([]Ty{}, envRef.Args...)}, seg, sp)
	return v, t, true
}

// traitInScopeOfImpl: inside `impl Trait for X`, Trait counts as in scope.
func (fcx *FnCtxt) traitInScopeOfImpl(tr *Def) bool {
	for d := fcx.def; d != nil; d = d.Parent {
		if d.Kind == DefImpl && d.Impl != nil && d.Impl.TraitRef != nil && d.Impl.TraitRef.Trait == tr {
			return true
		}
		if d.Kind == DefTrait && d == tr {
			return true
		}
	}
	return false
}
