package typeck

import (
	"fmt"
	"strconv"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
	"rustgo/compiler/resolve"
)

// genericsAst returns the ast generics of an item def.
func genericsAst(d *Def) *ast.Generics {
	if d.Item == nil {
		return nil
	}
	switch k := d.Item.Kind.(type) {
	case *ast.FnItem:
		return k.Generics
	case *ast.StructItem:
		return k.Generics
	case *ast.EnumItem:
		return k.Generics
	case *ast.UnionItem:
		return k.Generics
	case *ast.TraitItem:
		return k.Generics
	case *ast.ImplItem:
		return k.Generics
	case *ast.TypeAliasItem:
		return k.Generics
	}
	return nil
}

// genericParent returns the def whose generics are inherited.
func genericParent(d *Def) *Def {
	switch d.Kind {
	case DefVariant:
		return d.Parent
	case DefFn, DefConst, DefAssocTy:
		if d.Parent != nil && (d.Parent.Kind == DefTrait || d.Parent.Kind == DefImpl) {
			return d.Parent
		}
	}
	return nil
}

// Generics returns (lowering on first use) the generics of a def.
func (t *Tcx) Generics(d *Def) *Generics {
	if d.Generics != nil {
		return d.Generics
	}
	if d.Kind == DefVariant {
		d.Generics = t.Generics(d.Parent)
		return d.Generics
	}
	if d.Kind == DefClosure {
		d.Generics = &Generics{}
		return d.Generics
	}
	g := &Generics{}
	if p := genericParent(d); p != nil {
		pg := t.Generics(p)
		g.Parent = pg
		g.Params = append(g.Params, pg.Params...)
		g.Predicates = append(g.Predicates, pg.Predicates...)
	}
	if d.Kind == DefTrait {
		sd := NewDef(DefTyParam, "Self", d)
		gp := &GenericParamDef{Name: "Self", Index: 0, Def: sd, Owner: d}
		t.paramOf[sd] = gp
		g.Params = append(g.Params, gp)
		g.HasSelf = true
	}
	g.OwnStart = len(g.Params)
	ag := genericsAst(d)
	if ag != nil {
		for _, p := range ag.Params {
			if p.Kind == ast.GPLifetime {
				continue
			}
			kind := DefTyParam
			if p.Kind == ast.GPConst {
				kind = DefConstParam
			}
			pd := NewDef(kind, p.Name.Name, d)
			pd.Span = p.Name.Span
			gp := &GenericParamDef{Name: p.Name.Name, Index: len(g.Params), Const: p.Kind == ast.GPConst, Default: p.Default, Def: pd, Owner: d, Ast: p}
			t.paramOf[pd] = gp
			g.Params = append(g.Params, gp)
		}
	}
	// impl Trait in argument position -> synthetic params
	if d.Kind == DefFn {
		decl := d.Fn.Ast.Decl
		for _, p := range decl.Params {
			ast.Walk(p.Ty, func(n any) bool {
				switch x := n.(type) {
				case *ast.ImplTraitType:
					pd := NewDef(DefTyParam, "impl Trait", d)
					gp := &GenericParamDef{Name: fmt.Sprintf("impl#%d", len(g.Params)), Index: len(g.Params), Def: pd, Owner: d, Synthetic: true}
					t.paramOf[pd] = gp
					t.synthetic[x.ID] = gp.Index
					g.Params = append(g.Params, gp)
					return true // nested impl Trait in its bounds are params too
				case *ast.FnPtrType:
					return false
				}
				return true
			})
		}
	}
	d.Generics = g
	// Predicates
	ac := t.itemConv(d)
	if d.Kind == DefTrait {
		self := Param(0, "Self")
		for _, b := range d.Item.Kind.(*ast.TraitItem).Supertraits {
			if p := ac.lowerBound(b, self); p != nil {
				g.Predicates = append(g.Predicates, p)
			}
		}
	}
	var deferred []deferredPred
	if ag != nil {
		for _, p := range ag.Params {
			if p.Kind == ast.GPConst {
				gp := g.Params[t.paramIndex(g, p.Name.Name)]
				gp.ConstTy = ac.lowerTy(p.ConstTy)
				continue
			}
			if p.Kind != ast.GPType {
				continue
			}
			idx := t.paramIndex(g, p.Name.Name)
			self := Param(idx, p.Name.Name)
			for _, b := range p.Bounds {
				if b.Maybe {
					g.Params[idx].MaybeUnsized = true
				}
				if mentionsParamAssoc(b, g) {
					b, self := b, self
					deferred = append(deferred, deferredPred{needs: paramAssocRefs(b, g), lower: func() {
						if pr := ac.lowerBound(b, self); pr != nil {
							g.Predicates = append(g.Predicates, pr)
						}
					}})
					continue
				}
				if pr := ac.lowerBound(b, self); pr != nil {
					g.Predicates = append(g.Predicates, pr)
				}
			}
		}
		for _, w := range ag.Where {
			if w.Ty == nil {
				continue
			}
			w := w
			lower := func() {
				self := ac.lowerTy(w.Ty)
				for _, b := range w.Bounds {
					if b.Maybe && self.Kind == TParam && self.Index < len(g.Params) {
						g.Params[self.Index].MaybeUnsized = true
					}
					if pr := ac.lowerBound(b, self); pr != nil {
						g.Predicates = append(g.Predicates, pr)
					}
				}
			}
			// bounds naming another param's associated type (`T::Item`)
			// are lowered after the plain bounds that make it resolvable
			if mentionsParamAssoc(w, g) {
				deferred = append(deferred, deferredPred{needs: paramAssocRefs(w, g), lower: lower})
				continue
			}
			lower()
		}
	}
	// lower deferred bounds once the params whose assoc types they name
	// have bounds of their own (dependency order); then the rest
	hasBound := func(idx int) bool {
		for _, p := range g.Predicates {
			if a := p.Trait.Args[0]; a.Kind == TParam && a.Index == idx {
				return true
			}
		}
		return false
	}
	for len(deferred) > 0 {
		progress := false
		rest := deferred[:0]
		for _, d := range deferred {
			ready := true
			for _, idx := range d.needs {
				if !hasBound(idx) {
					ready = false
				}
			}
			if ready {
				d.lower()
				progress = true
			} else {
				rest = append(rest, d)
			}
		}
		deferred = rest
		if !progress {
			for _, d := range deferred {
				d.lower()
			}
			break
		}
	}
	if d.Kind == DefFn {
		for _, p := range d.Fn.Ast.Decl.Params {
			ast.Walk(p.Ty, func(n any) bool {
				switch x := n.(type) {
				case *ast.ImplTraitType:
					idx := t.synthetic[x.ID]
					self := g.Params[idx]
					for _, b := range x.Bounds {
						if pr := ac.lowerBound(b, Param(idx, self.Name)); pr != nil {
							g.Predicates = append(g.Predicates, pr)
						}
					}
					return true
				case *ast.FnPtrType:
					return false
				}
				return true
			})
		}
	}
	// Assoc type bounds in traits: `type Item: Clone;` -> predicates on the
	// projection are recorded on the assoc type def.
	if d.Kind == DefAssocTy && d.InTrait() {
		ta := d.Item.Kind.(*ast.TypeAliasItem)
		proj := Projection(d.Parent, t.Generics(d.Parent).Identity(), d.Name)
		for _, b := range ta.Bounds {
			if pr := ac.lowerBound(b, proj); pr != nil {
				d.AssocTy.Bounds = append(d.AssocTy.Bounds, pr)
			}
		}
	}
	return g
}

func (t *Tcx) paramIndex(g *Generics, name string) int {
	for i := len(g.Params) - 1; i >= 0; i-- {
		if g.Params[i].Name == name && !g.Params[i].Synthetic {
			return i
		}
	}
	return -1
}

// itemConv returns an astConv for lowering types inside item d.
func (t *Tcx) itemConv(d *Def) *astConv {
	ac := &astConv{t: t, scope: d.Scope, owner: d}
	if d.Kind == DefVariant {
		ac.owner = d.Parent
		d = d.Parent
	}
	ac.gen = d.Generics
	switch {
	case d.Kind == DefTrait:
		ac.selfTy = Param(0, "Self")
		ac.selfDef = d
	case d.Kind == DefImpl:
		ac.selfDef = d
		ac.selfTy = d.Impl.SelfTy // may be nil while lowering the header
	case d.Parent != nil && d.Parent.Kind == DefTrait:
		ac.selfTy = Param(0, "Self")
		ac.selfDef = d.Parent
	case d.Parent != nil && d.Parent.Kind == DefImpl:
		t.ImplHeader(d.Parent)
		ac.selfTy = d.Parent.Impl.SelfTy
		ac.selfDef = d.Parent
	case d.Kind == DefStruct || d.Kind == DefEnum || d.Kind == DefUnion:
		if d.Generics != nil {
			ac.selfTy = Adt(d, d.Generics.Identity())
		}
		ac.selfDef = d
	}
	return ac
}

// ImplHeader lowers the impl's self type and trait ref.
func (t *Tcx) ImplHeader(d *Def) {
	if d.Impl.SelfTy != nil || d.Collecting {
		return
	}
	d.Collecting = true
	t.Generics(d)
	ac := t.itemConv(d)
	ia := d.Impl.Ast
	self := ac.lowerTy(ia.SelfTy)
	d.Impl.SelfTy = self
	if ia.Trait != nil {
		tr, _ := ac.lowerTraitPath(ia.Trait, self, ia.Trait.Span)
		d.Impl.TraitRef = tr
	}
	d.Collecting = false
	// Lower impl assoc types eagerly (they may refer to Self).
	for _, it := range d.Impl.Order {
		if it.Kind == DefAssocTy {
			t.ImplAssocTy(it)
		}
	}
}

// ImplAssocTy lowers an associated type definition in an impl.
func (t *Tcx) ImplAssocTy(d *Def) Ty {
	if d.AssocTy.Ty != nil {
		return d.AssocTy.Ty
	}
	if d.AssocTy.TyAst == nil {
		return TyErr
	}
	if d.Collecting {
		t.Errorf(d.Span, "cycle lowering associated type `%s`", d.Name)
		return TyErr
	}
	d.Collecting = true
	t.Generics(d)
	ac := t.itemConv(d)
	d.AssocTy.Ty = ac.lowerTy(d.AssocTy.TyAst)
	d.Collecting = false
	return d.AssocTy.Ty
}

// AdtFields lowers field types of an ADT (idempotent).
func (t *Tcx) AdtFields(d *Def) {
	if d.Kind == DefVariant {
		d = d.Parent
	}
	if d.Collected {
		return
	}
	d.Collected = true
	t.Generics(d)
	ac := t.itemConv(d)
	lowerFields := func(v *VariantDef, fields []*ast.FieldDef) {
		v.Fields = nil
		for i, f := range fields {
			name := f.Name.Name
			if name == "" {
				name = strconv.Itoa(i)
			}
			v.Fields = append(v.Fields, &FieldDef{Name: name, Ty: ac.lowerTy(f.Ty), Vis: f.Vis, Ast: f})
		}
	}
	switch k := d.Item.Kind.(type) {
	case *ast.StructItem:
		v := &VariantDef{Def: d, Name: d.Name, Shape: k.Shape}
		d.Adt.Variants = []*VariantDef{v}
		lowerFields(v, k.Fields)
		d.Adt.TailParam = -1
		if n := len(v.Fields); n > 0 {
			last := v.Fields[n-1].Ty
			switch last.Kind {
			case TParam:
				d.Adt.TailParam = last.Index
			case TStr, TSlice, TDyn:
				d.Adt.TailParam = -2
			case TAdt:
				if last.Def != d {
					t.AdtFields(last.Def)
					if la := last.Def.Adt; la.TailParam >= 0 && la.TailParam < len(last.Args) && last.Args[la.TailParam].Kind == TParam {
						d.Adt.TailParam = last.Args[la.TailParam].Index
					}
				}
			}
		}
		d.Adt.TailKnown = true
	case *ast.UnionItem:
		v := &VariantDef{Def: d, Name: d.Name, Shape: ast.ShapeNamed}
		d.Adt.Variants = []*VariantDef{v}
		lowerFields(v, k.Fields)
	case *ast.EnumItem:
		var next int64
		for _, a := range d.Attrs {
			if a.Path == "repr" && len(a.Tokens) > 0 && a.Tokens[0].Delim == '(' {
				for _, tt := range a.Tokens[0].Inner {
					if _, ok := IntTyByName(tt.Tok.Text); ok {
						d.Adt.Repr = tt.Tok.Text
					}
				}
			}
		}
		for i, va := range k.Variants {
			v := d.Adt.Variants[i]
			lowerFields(v, va.Fields)
			if va.Disc != nil {
				if cv, ok := t.evalConstInt(ac, va.Disc); ok {
					next = cv
				} else {
					t.Errorf(va.Disc.GetSpan(), "could not evaluate enum discriminant")
				}
			}
			v.Disc = next
			next++
		}
	}
}

// FnSig lowers a function signature.
func (t *Tcx) FnSig(d *Def) *FnSig {
	if d.Fn.Sig != nil {
		return d.Fn.Sig
	}
	t.Generics(d)
	ac := t.itemConv(d)
	decl := d.Fn.Ast.Decl
	sig := &FnSig{Variadic: decl.Variadic}
	if sp := decl.SelfParam; sp != nil {
		self := ac.selfTy
		if self == nil {
			t.Errorf(sp.Span, "`self` parameter is only allowed in associated functions")
			self = TyErr
		}
		switch sp.Kind {
		case ast.SelfValue:
			sig.Inputs = append(sig.Inputs, self)
		case ast.SelfRef:
			sig.Inputs = append(sig.Inputs, Ref(self, false))
		case ast.SelfRefMut:
			sig.Inputs = append(sig.Inputs, Ref(self, true))
		case ast.SelfTyped:
			sig.Inputs = append(sig.Inputs, ac.lowerTy(sp.Ty))
		}
	}
	for _, p := range decl.Params {
		sig.Inputs = append(sig.Inputs, ac.lowerTy(p.Ty))
	}
	if decl.Output != nil {
		ac.opaqueFn = d
		sig.Output = ac.lowerTy(decl.Output)
		ac.opaqueFn = nil
	} else {
		sig.Output = TyUnit
	}
	d.Fn.Sig = sig
	return sig
}

// ConstTy lowers the declared type of a const or static.
func (t *Tcx) ConstTy(d *Def) Ty {
	if d.Const.Ty != nil {
		return d.Const.Ty
	}
	t.Generics(d)
	ac := t.itemConv(d)
	if d.Const.TyAst == nil {
		d.Const.Ty = TyErr
	} else {
		d.Const.Ty = ac.lowerTy(d.Const.TyAst)
	}
	return d.Const.Ty
}

// AliasTy lowers a type alias (in terms of its own generics).
func (t *Tcx) AliasTy(d *Def) Ty {
	if d.Alias.Ty != nil {
		return d.Alias.Ty
	}
	if d.Collecting {
		t.Errorf(d.Span, "cycle in type alias `%s`", d.Name)
		return TyErr
	}
	d.Collecting = true
	t.Generics(d)
	ac := t.itemConv(d)
	d.Alias.Ty = ac.lowerTy(d.Alias.TyAst)
	d.Collecting = false
	return d.Alias.Ty
}

// Supertraits returns the direct supertrait refs of a trait (Self = Param 0).
func (t *Tcx) Supertraits(d *Def) []*TraitRef {
	if d.Trait.Supertraits != nil {
		return d.Trait.Supertraits
	}
	g := t.Generics(d)
	out := []*TraitRef{}
	for _, p := range g.Predicates {
		if p.Trait.Args[0].Kind == TParam && p.Trait.Args[0].Index == 0 && !p.Maybe {
			out = append(out, p.Trait)
		}
	}
	d.Trait.Supertraits = out
	return out
}

// AllSupertraits returns the trait itself and all transitive supertraits,
// substituted for the given trait ref.
func (t *Tcx) AllSupertraits(tr *TraitRef) []*TraitRef {
	var out []*TraitRef
	seen := map[*Def]bool{}
	var visit func(x *TraitRef)
	visit = func(x *TraitRef) {
		if seen[x.Trait] {
			return
		}
		seen[x.Trait] = true
		out = append(out, x)
		for _, s := range t.Supertraits(x.Trait) {
			visit(s.Subst(x.Args))
		}
	}
	visit(tr)
	return out
}

// ---------------------------------------------------------------------------
// astConv: lowering ast types to Ty

type astConv struct {
	t       *Tcx
	scope   *Module
	owner   *Def
	gen     *Generics
	selfTy  Ty
	selfDef *Def
	// infer creates inference variables for `_` (nil outside bodies).
	infer func() Ty
	// opaqueFn: when lowering a fn return type, impl Trait becomes Opaque.
	opaqueFn *Def
	// subst: instance args applied to Param types produced (bodies).
	subst []Ty
	// exprCtx: generic args of value/pattern paths (defaults are inferred).
	exprCtx bool
	// valueLookup lets bodies resolve const params / locals in const exprs.
	valueLookup func(name string) (Ty, bool)
}

func (ac *astConv) firstLookup(name string, ns Namespace) (Res, bool) {
	if ns == TypeNS {
		if name == "Self" && ac.selfDef != nil {
			return Res{Kind: ResSelfTy, Def: ac.selfDef}, true
		}
	}
	if ac.gen != nil {
		if i := ac.t.paramIndex(ac.gen, name); i >= 0 {
			p := ac.gen.Params[i]
			if (ns == TypeNS && !p.Const) || (ns == ValueNS && p.Const) {
				return Res{Kind: ResDef, Def: p.Def}, true
			}
		}
	}
	if name == "Self" && ns == ValueNS && ac.selfDef != nil {
		return Res{Kind: ResSelfCtor, Def: ac.selfDef}, true
	}
	return Res{}, false
}

func pathSegs(p *ast.Path) []string {
	out := make([]string, len(p.Segments))
	for i, s := range p.Segments {
		out[i] = s.Ident.Name
	}
	return out
}

func (ac *astConv) resolve(p *ast.Path, ns Namespace) (resolve.PathResult, error) {
	pr, err := ac.t.R.ResolvePath(ac.scope, pathSegs(p), p.Global, ns, ac.firstLookup)
	if err == nil {
		ac.t.checkGlobAmbiguity(p, ac.scope, ns, ac.firstLookup)
	}
	if err == nil && pr.Private {
		ac.t.reportPrivatePath(p, pr)
	}
	return pr, err
}

func (ac *astConv) paramTy(gp *GenericParamDef) Ty {
	var ty Ty
	if gp.Const {
		ty = ConstParam(gp.Index, gp.Name)
	} else {
		ty = Param(gp.Index, gp.Name)
	}
	if ac.subst != nil {
		return Subst(ty, ac.subst)
	}
	return ty
}

func (ac *astConv) newInfer(sp lexer.Span) Ty {
	if ac.infer != nil {
		return ac.infer()
	}
	ac.t.Errorf(sp, "the placeholder `_` is not allowed within types on item signatures")
	return TyErr
}

func (ac *astConv) sub(t Ty) Ty {
	if ac.subst != nil {
		return Subst(t, ac.subst)
	}
	return t
}

// lowerTy converts an AST type to a Ty.
func (ac *astConv) lowerTy(at ast.Type) Ty {
	t := ac.t
	switch x := at.(type) {
	case nil:
		return TyUnit
	case *ast.ParenType:
		return ac.lowerTy(x.Elem)
	case *ast.TupleType:
		elems := make([]Ty, len(x.Elems))
		for i, e := range x.Elems {
			elems[i] = ac.lowerTy(e)
		}
		return Tuple(elems)
	case *ast.RefType:
		return Ref(ac.lowerTy(x.Elem), x.Mut)
	case *ast.PtrType:
		return Ptr(ac.lowerTy(x.Elem), x.Mut)
	case *ast.SliceType:
		return Slice(ac.lowerTy(x.Elem))
	case *ast.ArrayType:
		elem := ac.lowerTy(x.Elem)
		n := ac.lowerConstArg(x.Len)
		if n.Kind == TConst && int64(n.Val) < 0 {
			if v, ok := ac.t.evalConstValue(ac, x.Len); ok {
				if b, ok := intOf(v); ok && b.Sign() < 0 {
					// an array length is a usize
					ac.t.ErrorCode(x.Len.GetSpan(), "E0080", "evaluation of constant value failed: attempt to compute a negative array length, which would overflow")
					return Array(elem, TyErr)
				}
			}
		}
		return Array(elem, n)
	case *ast.NeverType:
		return TyNever
	case *ast.InferType:
		return ac.newInfer(x.Span)
	case *ast.FnPtrType:
		// `impl Trait` is not allowed in fn pointer types, also in a
		// function's return type (E0562)
		if ac.opaqueFn != nil {
			saved := ac.opaqueFn
			ac.opaqueFn = nil
			defer func() { ac.opaqueFn = saved }()
		}
		ins := make([]Ty, len(x.Inputs))
		for i, e := range x.Inputs {
			ins[i] = ac.lowerTy(e)
		}
		out := TyUnit
		if x.Output != nil {
			out = ac.lowerTy(x.Output)
		}
		return FnPtrSig(ins, out, x.Variadic, x.Unsafe, NormalizeAbi(x.Abi))
	case *ast.ImplTraitType:
		if idx, ok := t.synthetic[x.ID]; ok {
			return ac.paramTy(ac.gen.Params[idx])
		}
		if ac.opaqueFn != nil {
			fd := ac.opaqueFn
			idx := len(fd.Fn.AstOpaques)
			fd.Fn.AstOpaques = append(fd.Fn.AstOpaques, x)
			fd.Fn.OpaqueBounds = append(fd.Fn.OpaqueBounds, nil) // reserve slot idx
			ot := Opaque(fd, t.Generics(fd).Identity(), idx)
			var preds []*Predicate
			for _, b := range x.Bounds {
				if p := ac.lowerBound(b, ot); p != nil {
					preds = append(preds, p)
				}
			}
			fd.Fn.OpaqueBounds[idx] = preds
			return ot
		}
		t.ErrorCode(x.Span, "E0562", "`impl Trait` is not allowed here")
		return TyErr
	case *ast.DynTraitType:
		return ac.lowerDyn(x.Bounds, x.Span)
	case *ast.PathType:
		return ac.lowerPathType(x)
	case *ast.MacroType:
		t.Errorf(x.Span, "macro in type position was not expanded")
		return TyErr
	}
	t.Errorf(at.GetSpan(), "unsupported type syntax")
	return TyErr
}

func (ac *astConv) lowerDyn(bounds []*ast.Bound, sp lexer.Span) Ty {
	t := ac.t
	var principal *TraitRef
	var bindings []Binding
	var auto uint8
	placeholder := Param(1<<20, "dyn-self")
	for _, b := range bounds {
		if b.Lifetime != "" || b.Maybe {
			continue
		}
		tr, binds := ac.lowerTraitPath(b.Path, placeholder, b.Span)
		if tr == nil {
			continue
		}
		if tr.Trait.Trait.Auto || tr.Trait.LangItem == "send" || tr.Trait.LangItem == "sync" || tr.Trait.Name == "Send" || tr.Trait.Name == "Sync" || tr.Trait.Name == "Unpin" {
			// auto traits are part of the object type's identity
			switch {
			case tr.Trait.LangItem == "send" || tr.Trait.Name == "Send":
				auto |= DynSend
			case tr.Trait.LangItem == "sync" || tr.Trait.Name == "Sync":
				auto |= DynSync
			case tr.Trait.Name == "Unpin":
				auto |= DynUnpin
			default:
				auto |= DynOtherAuto
			}
			continue
		}
		if principal != nil {
			t.Errorf(sp, "only auto traits can be used as additional traits in a trait object")
			continue
		}
		principal = tr
		bindings = binds
	}
	if principal == nil {
		// dyn Send etc: use a marker; we model as dyn Any-like with no methods
		if any := t.LangOpt("any_trait"); any != nil {
			return Dyn(any, nil, nil, auto)
		}
		t.Errorf(sp, "at least one trait is required for an object type")
		return TyErr
	}
	// Fill in assoc type bindings from supertrait defaults? Keep as given.
	if ac.scope != nil && !(t.R.Std != nil && ac.scope.Crate == t.R.Std) {
		t.checkDynTrait(principal, bindings, sp)
	}
	sortBindings(bindings)
	return Dyn(principal.Trait, principal.Args[1:], bindings, auto)
}

func sortBindings(bs []Binding) {
	for i := 1; i < len(bs); i++ {
		for j := i; j > 0 && bs[j].Name < bs[j-1].Name; j-- {
			bs[j], bs[j-1] = bs[j-1], bs[j]
		}
	}
}

// lowerConstArg lowers an array length / const generic argument.
func (ac *astConv) lowerConstArg(e ast.Expr) Ty {
	if e == nil {
		return TyErr
	}
	if b, ok := e.(*ast.BlockExpr); ok && len(b.Block.Stmts) == 1 {
		if es, ok := b.Block.Stmts[0].(*ast.ExprStmt); ok {
			return ac.lowerConstArg(es.Expr)
		}
	}
	if p, ok := e.(*ast.PathExpr); ok && p.QSelf == nil && len(p.Path.Segments) == 1 && ac.gen != nil {
		if i := ac.t.paramIndex(ac.gen, p.Path.Segments[0].Ident.Name); i >= 0 && ac.gen.Params[i].Const {
			return ac.paramTy(ac.gen.Params[i])
		}
	}
	if _, ok := e.(*ast.UnderscoreExpr); ok {
		return ac.newInfer(e.GetSpan())
	}
	v, ok := ac.t.evalConstInt(ac, e)
	if !ok {
		ac.t.ErrorCode(e.GetSpan(), "E0080", "evaluation of constant value failed")
		return TyErr
	}
	return ConstVal(uint64(v))
}

// genericArgsFor lowers the generic args of a path segment for def d (whose
// own params are expected), returning the full args (parent args must be
// supplied by the caller in prefix).
func (ac *astConv) genericArgsFor(d *Def, seg *ast.PathSegment, prefix []Ty, sp lexer.Span, self Ty) []Ty {
	t := ac.t
	g := t.Generics(d)
	args := make([]Ty, 0, len(g.Params))
	args = append(args, prefix...)
	if g.HasSelf && len(args) == 0 {
		args = append(args, self)
	}
	own := g.Own()
	var given []ast.GenericArg
	nlt := 0
	if seg != nil && seg.Args != nil && !seg.Args.Parenthesized {
		for _, a := range seg.Args.Args {
			if a.Lifetime != "" {
				if len(given) > 0 && !t.isStd(d) {
					t.ErrorCode(sp, "E0747", "lifetime provided when a type was expected")
				}
				nlt++
				continue
			}
			given = append(given, a)
		}
		if len(seg.Args.Bindings) > 0 && d.Kind != DefTrait && !t.isStd(d) {
			t.ErrorCode(sp, "E0229", "associated item constraints are not allowed here")
		}
	}
	if nlt > 0 && !t.isStd(d) {
		declared := 0
		if ga := genericsAst(d); ga != nil {
			for _, p := range ga.Params {
				if p.Kind == ast.GPLifetime {
					declared++
				}
			}
		}
		if nlt > declared && (d.Kind == DefStruct || d.Kind == DefEnum || d.Kind == DefUnion || d.Kind == DefTrait || d.Kind == DefTyAlias) {
			t.ErrorCode(sp, "E0107", "%s takes %d lifetime argument%s but %d lifetime arguments were supplied", d.Kind, declared, plural(declared), nlt)
		}
		if d.Kind == DefFn && d.Fn != nil && d.Fn.Ast != nil && ac.owner != nil && ac.owner.Crate != nil && ac.owner.Crate.Local && (seg == nil || seg.Ident.Hyg == 0) {
			if early, ok := earlyBoundLifetimes(d.Fn.Ast); ok && early < declared {
				t.ErrorCode(sp, "E0794", "cannot specify lifetime arguments explicitly if late bound lifetime parameters are present")
			}
		}
	}
	gi := 0
	for _, p := range own {
		if p.Synthetic {
			args = append(args, ac.newInferOrErr(sp))
			continue
		}
		if gi < len(given) {
			a := given[gi]
			gi++
			if p.Const {
				if a.Const != nil {
					args = append(args, ac.lowerConstArg(a.Const))
				} else if a.Type != nil {
					// N passed as a path type
					if pt, ok := a.Type.(*ast.PathType); ok && pt.QSelf == nil {
						args = append(args, ac.lowerConstArg(&ast.PathExpr{ExprBase: ast.ExprBase{Span: pt.Span, ID: pt.ID}, Path: pt.Path}))
					} else if _, ok := a.Type.(*ast.InferType); ok {
						args = append(args, ac.newInferOrErr(sp))
					} else {
						args = append(args, TyErr)
					}
				}
			} else {
				if a.Type != nil {
					// nested types are ordinary type positions: their own
					// omitted params take defaults (`collect::<HashMap<K, V>>`)
					saved := ac.exprCtx
					ac.exprCtx = false
					args = append(args, ac.lowerTy(a.Type))
					ac.exprCtx = saved
				} else {
					t.Errorf(sp, "expected type argument")
					args = append(args, TyErr)
				}
			}
			continue
		}
		if p.Const && p.Ast != nil && p.Ast.ConstDefault != nil && !(ac.exprCtx && ac.infer != nil) {
			// `const N: usize = 1`
			dac := t.itemConv(d)
			args = append(args, dac.lowerConstArg(p.Ast.ConstDefault))
			continue
		}
		if p.Default != nil && !(ac.exprCtx && ac.infer != nil) {
			// Defaults may refer to earlier params and Self.
			dac := t.itemConv(d)
			dt := dac.lowerTy(p.Default)
			args = append(args, Subst(dt, args))
			continue
		}
		args = append(args, ac.newInferOrErr(sp))
	}
	if gi < len(given) {
		t.ErrorCode(sp, "E0107", "too many generic arguments for `%s`", d.Name)
	}
	// implicit `Sized` bounds of an ADT's type parameters
	if (d.Kind == DefStruct || d.Kind == DefEnum || d.Kind == DefUnion) && ac.owner != nil && !t.isStd(ac.owner) {
		for _, p := range own {
			if p.Const || p.Synthetic || p.MaybeUnsized || p.Index >= len(args) {
				continue
			}
			if a := args[p.Index]; a != nil && a.IsUnsized() {
				t.ErrorCode(sp, "E0277", "the size for values of type `%s` cannot be known at compilation time", a)
			}
		}
	}
	return args
}

func (ac *astConv) newInferOrErr(sp lexer.Span) Ty {
	if ac.infer != nil {
		return ac.infer()
	}
	ac.t.ErrorCode(sp, "E0107", "missing generics")
	return TyErr
}

func (ac *astConv) lowerPathType(x *ast.PathType) Ty {
	t := ac.t
	if x.QSelf != nil {
		self := ac.lowerTy(x.QSelf.Ty)
		segs := x.Path.Segments
		if x.QSelf.Position == 0 {
			// <T>::Assoc
			return ac.assocTypeOn(self, segs, x.Span)
		}
		traitPath := &ast.Path{Global: x.Path.Global, Segments: segs[:x.QSelf.Position], Span: x.Path.Span}
		tr, _ := ac.lowerTraitPath(traitPath, self, x.Span)
		if tr == nil {
			return TyErr
		}
		rest := segs[x.QSelf.Position:]
		if len(rest) == 0 {
			t.Errorf(x.Span, "expected associated type")
			return TyErr
		}
		// `<u8 as Trait>::A` with no `impl Trait for u8` (a local trait
		// and fully known types)
		if t.implsIndexed && ac.owner != nil && ac.owner.Crate != nil && ac.owner.Crate.Local && !t.isStd(tr.Trait) && !fromMacro(x.Path) {
			known := true
			for _, a := range tr.Args {
				if a == nil || a.HasParam() || a.HasInfer() || a.HasProj() || a.HasOpaque() || a.HasError() {
					known = false
				}
			}
			if known && newInferCtxt(t).evaluate(&Predicate{Trait: tr}, 0) == SelNone {
				t.ErrorCode(x.Span, "E0277", "the trait bound `%s: %s` is not satisfied", tr.Args[0], tr.Trait.Name)
				return TyErr
			}
		}
		ty := ac.projectionFor(tr, rest[0].Ident.Name, x.Span)
		if len(rest) > 1 {
			return ac.assocTypeOn(ty, rest[1:], x.Span)
		}
		return ty
	}
	pr, err := ac.resolve(x.Path, TypeNS)
	if err != nil {
		if len(x.Path.Segments) == 1 && !x.Path.Global {
			t.ErrorCode(x.Span, "E0412", "cannot find type `%s` in this scope", x.Path.Segments[0].Ident.Name)
		} else {
			t.Errorf(x.Span, "%v", err)
		}
		return TyErr
	}
	segs := x.Path.Segments
	base := ac.resToTy(pr.Res, segs[pr.Consumed-1], x.Span)
	if pr.Consumed < len(segs) {
		// `Type::Assoc` on a concrete type (not `Self` or a type
		// parameter) needs `<Type as Trait>::Assoc`
		if d := pr.Res.Def; (pr.Res.Kind == ResPrim || (pr.Res.Kind == ResDef && d != nil && (d.Kind == DefStruct || d.Kind == DefEnum || d.Kind == DefUnion))) &&
			ac.owner != nil && !t.isStd(ac.owner) && !base.HasParam() && !fromMacro(x.Path) {
			t.ErrorCode(x.Span, "E0223", "ambiguous associated type: use fully-qualified syntax `<%s as Example>::%s`", base, segs[pr.Consumed].Ident.Name)
			return TyErr
		}
		return ac.assocTypeOn(base, segs[pr.Consumed:], x.Span)
	}
	return base
}

// resToTy converts a type-namespace resolution to a type.
func (ac *astConv) resToTy(res Res, seg *ast.PathSegment, sp lexer.Span) Ty {
	t := ac.t
	switch res.Kind {
	case ResPrim:
		if seg != nil && seg.Args != nil && (len(seg.Args.Args) > 0 || len(seg.Args.Bindings) > 0) {
			t.ErrorCode(sp, "E0109", "type arguments are not allowed on builtin type `%s`", res.Prim)
		}
		return primTy(res.Prim)
	case ResSelfTy:
		if ac.selfTy == nil {
			if res.Def != nil && res.Def.Kind == DefImpl {
				t.ImplHeader(res.Def)
				return ac.sub(res.Def.Impl.SelfTy)
			}
			t.Errorf(sp, "`Self` is not available here")
			return TyErr
		}
		return ac.sub(ac.selfTy)
	case ResDef:
		d := res.Def
		if (d.Kind == DefMod || d.Kind == DefCrate) && resolve.IsPrimName(seg.Ident.Name) {
			return primTy(seg.Ident.Name)
		}
		switch d.Kind {
		case DefStruct, DefEnum, DefUnion:
			return Adt(d, ac.genericArgsFor(d, seg, nil, sp, nil))
		case DefTyAlias:
			aty := t.AliasTy(d)
			args := ac.genericArgsFor(d, seg, nil, sp, nil)
			return Subst(aty, args)
		case DefTyParam:
			gp := t.paramOf[d]
			if gp == nil {
				t.Errorf(sp, "unknown type parameter `%s`", d.Name)
				return TyErr
			}
			return ac.paramTy(gp)
		case DefTrait:
			// bare trait object (2015/2018 style) -> dyn
			if !sp.Before2021() && sp.Hi.Off > sp.Lo.Off {
				t.ErrorCode(sp, "E0782", "expected a type, found a trait (trait objects must include the `dyn` keyword: `dyn %s`)", d.Name)
				return TyErr
			}
			tr, binds := ac.lowerTraitSeg(d, seg, Param(1<<20, "dyn-self"), sp)
			if tr == nil {
				return TyErr
			}
			return Dyn(d, tr.Args[1:], binds, 0)
		case DefAssocTy:
			if d.InTrait() && ac.selfTy != nil {
				tr := &TraitRef{Trait: d.Parent, Args: ac.traitArgsForSelf(d.Parent)}
				return ac.projectionFor(tr, d.Name, sp)
			}
		}
		t.Errorf(sp, "expected type, found %s `%s`", d.Kind, d.Name)
		return TyErr
	}
	t.Errorf(sp, "expected type")
	return TyErr
}

// traitArgsForSelf returns identity args of a trait whose Self is the
// current self type.
func (ac *astConv) traitArgsForSelf(tr *Def) []Ty {
	g := ac.t.Generics(tr)
	args := g.Identity()
	args[0] = ac.selfTy
	return args
}

func primTy(name string) Ty {
	switch name {
	case "bool":
		return TyBool
	case "char":
		return TyChar
	case "str":
		return TyStr
	case "f32":
		return TyF32
	case "f64":
		return TyF64
	}
	if it, ok := IntTyByName(name); ok {
		return IntT(it)
	}
	return TyErr
}

// assocTypeOn resolves `Base::Name` where Name is an associated type.
func (ac *astConv) assocTypeOn(base Ty, segs []*ast.PathSegment, sp lexer.Span) Ty {
	t := ac.t
	cur := base
	for _, seg := range segs {
		name := seg.Ident.Name
		next := ac.findAssocType(cur, name, sp)
		if next != nil {
			next = ac.applyGatArgs(next, cur, name, seg)
		}
		if next == nil {
			t.Errorf(sp, "associated type `%s` not found for `%s`", name, cur)
			return TyErr
		}
		cur = next
	}
	return cur
}

// gatArgs lowers the type arguments of a generic associated type segment
// (`Self::Member<T>`); lifetimes are erased.
func (ac *astConv) gatArgs(seg *ast.PathSegment) []Ty {
	if seg == nil || seg.Args == nil || seg.Args.Parenthesized {
		return nil
	}
	var out []Ty
	for _, a := range seg.Args.Args {
		if a.Type != nil {
			out = append(out, ac.lowerTy(a.Type))
		}
	}
	return out
}

// applyGatArgs attaches GAT arguments to a resolved associated type: a
// projection carries them after the trait's args; an impl's own definition
// has its GAT params substituted.
func (ac *astConv) applyGatArgs(at Ty, cur Ty, name string, seg *ast.PathSegment) Ty {
	gat := ac.gatArgs(seg)
	if len(gat) == 0 {
		return at
	}
	if at.Kind == TProjection {
		args := append(append([]Ty{}, at.Args...), gat...)
		return Projection(at.Def, args, at.Name)
	}
	// the impl's own `type Name<..> = ..` (Self inside the impl)
	if ac.selfDef != nil && ac.selfDef.Kind == DefImpl {
		if it, ok := ac.selfDef.Impl.Items[name]; ok && it.Kind == DefAssocTy {
			full := append(ac.t.Generics(ac.selfDef).Identity(), gat...)
			return ac.sub(Subst(ac.t.ImplAssocTy(it), full))
		}
	}
	return at
}

// findAssocType finds `<cur as SomeTrait>::name`.
func (ac *astConv) findAssocType(cur Ty, name string, sp lexer.Span) Ty {
	t := ac.t
	// Bounds on params (and Self in traits).
	if ac.gen != nil {
		unsub := cur
		var found Ty
		var foundTr *Def
		for _, p := range ac.gen.Predicates {
			pself := p.Trait.Args[0]
			if pself != unsub && ac.sub(pself) != cur {
				continue
			}
			for _, st := range t.AllSupertraits(p.Trait) {
				if it, ok := st.Trait.Trait.Items[name]; ok && it.Kind == DefAssocTy {
					pr := ac.sub(Projection(st.Trait, st.Args, name))
					if found == nil {
						found, foundTr = pr, st.Trait
					} else if pr != found && cur.Kind == TParam && ac.scope != nil && ac.scope.Crate != nil && ac.scope.Crate.Local && !pr.HasInfer() && !found.HasInfer() {
						// two bounds of a type parameter declare the
						// associated type
						t.ErrorCode(sp, "E0221", "ambiguous associated type `%s` in bounds of `%s` (from `%s` and `%s`)", name, cur, foundTr.Name, st.Trait.Name)
					}
					break
				}
			}
		}
		if found != nil {
			return found
		}
	}
	// Self inside an impl of a trait: use the impl's own definition.
	if ac.selfDef != nil && ac.selfDef.Kind == DefImpl && ac.selfTy != nil && (cur == ac.selfTy || cur == ac.sub(ac.selfTy)) {
		if it, ok := ac.selfDef.Impl.Items[name]; ok && it.Kind == DefAssocTy {
			return ac.sub(t.ImplAssocTy(it))
		}
		if tr := ac.selfDef.Impl.TraitRef; tr != nil {
			for _, st := range t.AllSupertraits(tr) {
				if it, ok := st.Trait.Trait.Items[name]; ok && it.Kind == DefAssocTy {
					return ac.sub(Projection(st.Trait, st.Args, name))
				}
			}
		}
	}
	// Self in a trait: the trait's own assoc types.
	if ac.selfDef != nil && ac.selfDef.Kind == DefTrait && cur.Kind == TParam && cur.Index == 0 {
		tr := &TraitRef{Trait: ac.selfDef, Args: t.Generics(ac.selfDef).Identity()}
		for _, st := range t.AllSupertraits(tr) {
			if it, ok := st.Trait.Trait.Items[name]; ok && it.Kind == DefAssocTy {
				return Projection(st.Trait, st.Args, name)
			}
		}
	}
	// Concrete types: search traits declaring this assoc type that the type implements.
	if !cur.HasParam() {
		for _, tr := range t.traitsByItem[name] {
			if tr.Trait.Items[name].Kind != DefAssocTy {
				continue
			}
			if len(t.Generics(tr).Params) != 1 {
				continue
			}
			ref := &TraitRef{Trait: tr, Args: []Ty{cur}}
			if impl, args := t.SelectImplSimple(ref); impl != nil {
				_ = args
				return Projection(tr, ref.Args, name)
			}
		}
	}
	return nil
}

func (ac *astConv) projectionFor(tr *TraitRef, name string, sp lexer.Span) Ty {
	for _, st := range ac.t.AllSupertraits(tr) {
		if it, ok := st.Trait.Trait.Items[name]; ok && it.Kind == DefAssocTy {
			return Projection(st.Trait, st.Args, name)
		}
	}
	ac.t.Errorf(sp, "associated type `%s` not found in trait `%s`", name, tr.Trait.Name)
	return TyErr
}

// lowerTraitPath resolves a trait path and lowers its args with the given Self.
func (ac *astConv) lowerTraitPath(p *ast.Path, self Ty, sp lexer.Span) (*TraitRef, []Binding) {
	t := ac.t
	pr, err := ac.resolve(p, TypeNS)
	if err != nil {
		t.Errorf(sp, "%v", err)
		return nil, nil
	}
	if pr.Consumed != len(p.Segments) || pr.Res.Kind != ResDef || pr.Res.Def.Kind != DefTrait {
		t.Errorf(sp, "expected trait, found `%s`", ast.PathString(p))
		return nil, nil
	}
	return ac.lowerTraitSeg(pr.Res.Def, p.Segments[len(p.Segments)-1], self, sp)
}

func (ac *astConv) lowerTraitSeg(d *Def, seg *ast.PathSegment, self Ty, sp lexer.Span) (*TraitRef, []Binding) {
	var bindings []Binding
	if seg.Args != nil && seg.Args.Parenthesized {
		// Fn(A, B) -> C
		ins := make([]Ty, len(seg.Args.Inputs))
		for i, in := range seg.Args.Inputs {
			ins[i] = ac.lowerTy(in)
		}
		out := TyUnit
		if seg.Args.Output != nil {
			out = ac.lowerTy(seg.Args.Output)
		}
		tr := &TraitRef{Trait: d, Args: []Ty{self, Tuple(ins)}}
		if len(ins) == 0 {
			tr.Args[1] = TyUnit
		}
		return tr, []Binding{{Name: "Output", Ty: out}}
	}
	args := ac.genericArgsFor(d, seg, nil, sp, self)
	if len(args) > 0 {
		args[0] = self
	}
	tr := &TraitRef{Trait: d, Args: args}
	if seg.Args != nil {
		for _, b := range seg.Args.Bindings {
			if ac.scope != nil && ac.scope.Crate != nil && ac.scope.Crate.Local {
				ac.t.pendingBindings = append(ac.t.pendingBindings, pendingBinding{trait: tr, name: b.Name.Name, sp: b.Name.Span})
			}
			if b.Ty != nil {
				nb := Binding{Name: b.Name.Name, Ty: ac.lowerTy(b.Ty)}
				if b.Args != nil {
					nb.Args = ac.gatArgs(&ast.PathSegment{Args: b.Args})
				}
				bindings = append(bindings, nb)
			}
		}
	}
	return tr, bindings
}

// lowerBound lowers a trait bound on self into a predicate (nil for
// lifetime bounds and ?Sized).
func (ac *astConv) lowerBound(b *ast.Bound, self Ty) *Predicate {
	if b.Lifetime != "" || b.Path == nil || self == nil {
		return nil
	}
	tr, binds := ac.lowerTraitPath(b.Path, self, b.Span)
	if tr == nil {
		return nil
	}
	return &Predicate{Trait: tr, Bindings: binds, Maybe: b.Maybe}
}

// mentionsParamAssoc reports whether n contains a type path `P::Name...`
// whose first segment is one of g's type parameters.
func mentionsParamAssoc(n any, g *Generics) bool {
	found := false
	ast.Walk(n, func(x any) bool {
		if found {
			return false
		}
		if pt, ok := x.(*ast.PathType); ok && pt.QSelf == nil && pt.Path != nil && len(pt.Path.Segments) >= 2 && !pt.Path.Global {
			name := pt.Path.Segments[0].Ident.Name
			for _, gp := range g.Params {
				if gp.Name == name && !gp.Const {
					found = true
					return false
				}
			}
		}
		return true
	})
	return found
}

type deferredPred struct {
	needs []int // params whose associated types the bound names
	lower func()
}

// paramAssocRefs lists the indices of g's type params P used as `P::Name`.
func paramAssocRefs(n any, g *Generics) []int {
	var out []int
	ast.Walk(n, func(x any) bool {
		if pt, ok := x.(*ast.PathType); ok && pt.QSelf == nil && pt.Path != nil && len(pt.Path.Segments) >= 2 && !pt.Path.Global {
			name := pt.Path.Segments[0].Ident.Name
			for _, gp := range g.Params {
				if gp.Name == name && !gp.Const {
					out = append(out, gp.Index)
				}
			}
		}
		return true
	})
	return out
}
