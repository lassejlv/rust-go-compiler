package borrowck

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
	"rustgo/compiler/typeck"
)

// declarePat declares the bindings of a pattern in the current scope.
func (b *builder) declarePat(p ast.Pat) {
	ast.Walk(p, func(n any) bool {
		if ip, ok := n.(*ast.IdentPat); ok && b.r.Paths[ip.ID] == nil {
			if l := b.r.Locals[ip.ID]; l != nil {
				b.userLocal(l)
			}
		}
		if op, ok := n.(*ast.OrPat); ok && len(op.Alts) > 0 {
			ast.Walk(op.Alts[0], func(m any) bool {
				if ip, ok := m.(*ast.IdentPat); ok && b.r.Paths[ip.ID] == nil {
					if l := b.r.Locals[ip.ID]; l != nil {
						b.userLocal(l)
					}
				}
				return true
			})
			return false
		}
		return true
	})
}

func (b *builder) patDerefs(p ast.Pat, tp tplace) tplace {
	for _, rt := range b.r.PatDerefs[p.GetID()] {
		k, elem, ok := derefKindOf(rt)
		if !ok {
			continue
		}
		tp = tplace{p: tp.p.project(Proj{Kind: PDeref, Deref: k}), ty: elem}
	}
	return tp
}

// bindPat binds the variables of p from the place tp. With guard, by-value
// bindings copy instead of moving (the moves happen in bindPatInit once
// the arm's guard holds).
func (b *builder) bindPat(p ast.Pat, tp tplace, guard bool) {
	b.walkPat(p, tp, func(ip *ast.IdentPat, l *typeck.Local, src tplace) {
		id := b.userLocal(l)
		sp := ip.Span
		if l.ByRef && src.ty != l.Ty {
			b.emit(Stmt{Kind: SAssign, Place: Place{Local: id}, Rv: Rvalue{Kind: RRef, Place: src.p, Mut: l.RefMut}, Span: sp, Init: true})
			return
		}
		op := b.operandOf(src)
		if guard && op.Kind == OMove {
			op.Kind = OCopy
		}
		b.emit(Stmt{Kind: SAssign, Place: Place{Local: id}, Rv: Rvalue{Kind: RUse, Ops: []Operand{op}}, Span: sp, Init: true})
	})
}

// bindPatInit performs the moves of by-value bindings after a guard.
func (b *builder) bindPatInit(p ast.Pat, tp tplace) {
	b.walkPat(p, tp, func(ip *ast.IdentPat, l *typeck.Local, src tplace) {
		if l.ByRef && src.ty != l.Ty {
			return
		}
		op := b.operandOf(src)
		if op.Kind != OMove {
			return
		}
		id := b.userLocal(l)
		b.emit(Stmt{Kind: SAssign, Place: Place{Local: id}, Rv: Rvalue{Kind: RUse, Ops: []Operand{op}}, Span: ip.Span, Init: true})
	})
}

func (b *builder) walkPat(p ast.Pat, tp tplace, bind func(*ast.IdentPat, *typeck.Local, tplace)) {
	if p == nil {
		return
	}
	tp = b.patDerefs(p, tp)
	switch x := p.(type) {
	case *ast.ParenPat:
		b.walkPat(x.Pat, tp, bind)
	case *ast.IdentPat:
		if b.r.Paths[x.ID] != nil {
			return
		}
		l := b.r.Locals[x.ID]
		if l == nil {
			return
		}
		if _, isRest := x.Sub.(*ast.RestPat); isRest {
			// `rest @ ..` in a slice pattern: by-reference subslice
			id := b.userLocal(l)
			if l.Ty != nil && l.Ty.Kind == TRef {
				b.emit(Stmt{Kind: SAssign, Place: Place{Local: id}, Rv: Rvalue{Kind: RRef, Place: tp.p, Mut: l.Ty.Mut}, Span: x.Span, Init: true})
			} else {
				b.emit(Stmt{Kind: SAssign, Place: Place{Local: id}, Rv: Rvalue{Kind: RUse, Ops: []Operand{b.operandOf(tp)}}, Span: x.Span, Init: true})
			}
			return
		}
		if x.Sub != nil {
			// `x @ sub`: sub-bindings first (by reference they must not
			// see a moved value)
			b.walkPat(x.Sub, tp, bind)
		}
		bind(x, l, tp)
	case *ast.TuplePat:
		if tp.ty == nil || tp.ty.Kind != TTuple {
			return
		}
		seqFields(x.Elems, len(tp.ty.Elems), func(pi, fi int) {
			b.walkPat(x.Elems[pi], tplace{p: tp.p.project(Proj{Kind: PField, Field: fi}), ty: tp.ty.Elems[fi]}, bind)
		})
	case *ast.TupleStructPat:
		v := b.r.Variants[x.ID]
		if v == nil || tp.ty == nil || tp.ty.Kind != TAdt {
			return
		}
		fts := b.fieldTypes(tp.ty, v.Index)
		vn := variantNum(tp.ty, v)
		dt := b.dropTy(tp.ty)
		seqFields(x.Elems, len(fts), func(pi, fi int) {
			b.walkPat(x.Elems[pi], tplace{p: tp.p.project(Proj{Kind: PField, Field: fi, Variant: vn, DropTy: dt}), ty: fts[fi]}, bind)
		})
	case *ast.StructPat:
		v := b.r.Variants[x.ID]
		if v == nil || tp.ty == nil || tp.ty.Kind != TAdt {
			return
		}
		fts := b.fieldTypes(tp.ty, v.Index)
		vn := variantNum(tp.ty, v)
		for _, f := range x.Fields {
			fi := v.FieldIndex(f.Name.Name)
			if fi < 0 || fi >= len(fts) {
				continue
			}
			b.walkPat(f.Pat, tplace{p: tp.p.project(Proj{Kind: PField, Field: fi, Variant: vn, Name: f.Name.Name, DropTy: b.dropTy(tp.ty)}), ty: fts[fi]}, bind)
		}
	case *ast.RefPat:
		b.walkPat(x.Pat, tp.deref(), bind)
	case *ast.BoxPat:
		b.walkPat(x.Pat, tp.deref(), bind)
	case *ast.SlicePat:
		var et Ty
		if tp.ty != nil && (tp.ty.Kind == TArray || tp.ty.Kind == TSlice) {
			et = tp.ty.Elem
		}
		for i, e := range x.Elems {
			if _, ok := e.(*ast.RestPat); ok {
				continue
			}
			ip, isIdent := e.(*ast.IdentPat)
			if isIdent {
				if _, isRest := ip.Sub.(*ast.RestPat); isRest {
					// the rest subslice is disjoint from the named elements
					b.walkPat(e, tplace{p: tp.p.project(Proj{Kind: PIndex, Index: -2, Field: 100000 + i}), ty: tp.ty}, bind)
					continue
				}
			}
			b.walkPat(e, tplace{p: tp.p.project(Proj{Kind: PIndex, Index: -2, Field: i}), ty: et}, bind)
		}
	case *ast.OrPat:
		if len(x.Alts) > 0 {
			b.walkPat(x.Alts[0], tp, bind)
		}
	}
}

// variantNum is the PField Variant of enum variant v (0 for structs).
func variantNum(t Ty, v *VariantDef) int {
	if t.Def != nil && t.Def.Adt != nil && t.Def.Adt.Kind == AdtEnum {
		return v.Index + 1
	}
	return 0
}

func seqFields(pats []ast.Pat, n int, f func(pi, fi int)) {
	rest := -1
	for i, p := range pats {
		if _, ok := p.(*ast.RestPat); ok {
			rest = i
		}
	}
	if rest < 0 {
		for i := range pats {
			if i < n {
				f(i, i)
			}
		}
		return
	}
	for i := 0; i < rest; i++ {
		f(i, i)
	}
	after := len(pats) - rest - 1
	for i := 0; i < after; i++ {
		f(rest+1+i, n-after+i)
	}
}

// ---------------------------------------------------------------------------
// Bodies

// buildFn builds the MIR of a function body.
func buildFn(tcx *typeck.Tcx, d *Def, r *typeck.Results) *builder {
	b := newBuilder(tcx, r, d.Name, d.Span)
	b.name = d.Name
	b.newLocal(&LocalDecl{Kind: LReturn, Ty: r.RetTy, Mut: true})
	b.retBlock = b.newBlock()
	b.pushScope()
	sig := tcx.FnSig(d)
	for i, l := range r.Params {
		ld := &LocalDecl{Kind: LArg, Ty: l.Ty, Mut: l.Mut, Name: l.Name, Span: l.Span, User: l, FnBound: -1}
		if sig != nil && i < len(sig.Inputs) {
			b.genericParamInfo(d, sig.Inputs[i], ld)
		}
		id := b.newLocal(ld)
		b.locals[l] = id
		b.scopes[0].locals = append(b.scopes[0].locals, id)
	}
	fa := d.Fn.Ast
	pi := 0
	if fa.Decl.SelfParam != nil {
		pi = 1
	}
	for _, p := range fa.Decl.Params {
		if pi >= len(r.Params) {
			break
		}
		l := r.Params[pi]
		pi++
		if l.Name == "__arg" {
			b.declarePat(p.Pat)
			b.bindPat(p.Pat, tplace{p: Place{Local: b.locals[l]}, ty: l.Ty}, false)
		}
	}
	b.retCoerce = true
	b.block(fa.Body, tplace{p: Place{Local: 0}, ty: r.RetTy, coerce: true})
	b.popScope(fa.Body.Span)
	b.gotoBlock(b.retBlock)
	b.cur = b.retBlock
	b.terminate(Term{Kind: TReturn, Span: fa.Body.Span})
	b.fixupMutability()
	b.checkGuards()
	return b
}

// genericParamInfo records the bounds of a parameter declared with a type
// parameter type (rustc checks the generic body, not an instance).
func (b *builder) genericParamInfo(d *Def, decl Ty, ld *LocalDecl) {
	if decl == nil || decl.Kind != TParam || decl.Sub == 1 {
		return
	}
	copyTr := b.tcx.LangOpt("copy")
	isCopy := false
	for _, p := range b.tcx.Generics(d).Predicates {
		if p.Trait == nil || len(p.Trait.Args) == 0 {
			continue
		}
		a := p.Trait.Args[0]
		if a.Kind != TParam || a.Index != decl.Index {
			continue
		}
		tr := p.Trait.Trait
		if tr == copyTr {
			isCopy = true
		}
		switch tr.LangItem {
		case "fn":
			if ld.FnBound < 0 || ld.FnBound > 0 {
				ld.FnBound = 0
			}
		case "fn_mut":
			if ld.FnBound < 0 || ld.FnBound > 1 {
				ld.FnBound = 1
			}
		case "fn_once":
			if ld.FnBound < 0 {
				ld.FnBound = 2
			}
		}
		// supertraits of the bound (e.g. a trait with a Copy supertrait)
		for _, st := range b.tcx.AllSupertraits(p.Trait) {
			if st.Trait == copyTr {
				isCopy = true
			}
		}
	}
	ld.GenericMove = !isCopy
}

// buildClosure builds the MIR of a closure body.
func buildClosure(tcx *typeck.Tcx, r *typeck.Results, ci *typeck.ClosureInfo, name string) *builder {
	return buildClosureKind(tcx, r, ci, name, ci.Kind)
}

// buildClosureKind builds a closure body checked as a closure of the given
// kind (the kind an Fn/FnMut bound requires may be stricter than the one
// inferred from the body).
func buildClosureKind(tcx *typeck.Tcx, r *typeck.Results, ci *typeck.ClosureInfo, name string, kind int) *builder {
	b := newBuilder(tcx, r, name, ci.Expr.Span)
	b.name = name
	b.body.ClosureKind = kind
	b.newLocal(&LocalDecl{Kind: LReturn, Ty: ci.Output, Mut: true})
	b.retBlock = b.newBlock()
	b.pushScope()
	b.closure = ci
	for _, c := range ci.Captures {
		ty := c.Ty
		if c.ByRef {
			ty = Ref(c.Ty, c.Mut)
		}
		name := c.Local.Name
		for _, pr := range c.Path {
			if pr.Deref {
				name = "*" + name
			} else {
				name += "." + pr.Name
			}
		}
		id := b.newLocal(&LocalDecl{Kind: LUpvar, Ty: ty, Mut: c.Mut || c.Local.Mut, Name: name, Span: c.Local.Span, User: c.Local, UpvarByRef: c.ByRef})
		u := &upvar{local: id, byRef: c.ByRef, mutRef: c.Mut}
		b.capUpvars = append(b.capUpvars, u)
		if len(c.Path) == 0 {
			b.upvars[c.Local] = u
		}
	}
	for _, l := range ci.Params {
		id := b.newLocal(&LocalDecl{Kind: LArg, Ty: l.Ty, Mut: l.Mut, Name: l.Name, Span: l.Span, User: l})
		b.locals[l] = id
		b.scopes[0].locals = append(b.scopes[0].locals, id)
	}
	for i, l := range ci.Params {
		if l.Name == "__arg" && i < len(ci.Expr.Params) {
			p := ci.Expr.Params[i].Pat
			b.declarePat(p)
			b.bindPat(p, tplace{p: Place{Local: b.locals[l]}, ty: l.Ty}, false)
		}
	}
	b.pushTemps()
	b.retCoerce = ci.Expr.Ret != nil
	b.exprInto(ci.Expr.Body, tplace{p: Place{Local: 0}, ty: ci.Output, coerce: b.retCoerce})
	b.popTemps(ci.Expr.Span)
	b.popScope(ci.Expr.Span)
	b.gotoBlock(b.retBlock)
	b.cur = b.retBlock
	b.terminate(Term{Kind: TReturn, Span: ci.Expr.Span})
	b.fixupMutability()
	b.checkGuards()
	return b
}

var _ = lexer.Span{}
