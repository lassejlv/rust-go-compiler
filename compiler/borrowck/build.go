package borrowck

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
	"rustgo/compiler/typeck"
)

// tplace is a place with its type.
type tplace struct {
	p  Place
	ty Ty
}

type scope struct {
	locals []LocalID
}

type loopCtx struct {
	label    string
	brk      int // block to continue at after break
	cont     int // -1 for labeled blocks
	depth    int // scope depth at loop entry
	dest     *tplace
	isBlock  bool
	tempMark int
}

type upvar struct {
	local  LocalID
	byRef  bool
	mutRef bool
}

type builder struct {
	tcx  *typeck.Tcx
	r    *typeck.Results
	body *Body
	cur  int // current block (-1: unreachable code)

	locals map[*typeck.Local]LocalID
	upvars map[*typeck.Local]*upvar // whole-variable captures
	// closure bodies: the closure and the upvar of each of its captures
	closure   *typeck.ClosureInfo
	capUpvars []*upvar
	scopes    []*scope
	loops     []*loopCtx
	// temps holds the temporaries of the enclosing statements (stack of
	// marks); they die at the end of their statement.
	temps     []LocalID
	tempMarks []int
	retBlock  int

	closures []*Body // closure bodies built from this body (and nested)
	name     string
	inGuard  bool
	// overloaded deref/index results: result temp -> the borrow of the base
	ovl map[LocalID]*ovlInfo
	// statics referenced by the body
	statics map[*Def]LocalID
}

type ovlInfo struct {
	block, stmt int
	baseTy      Ty
	trait       string // lang item needed for mutable access
}

func newBuilder(tcx *typeck.Tcx, r *typeck.Results, name string, sp lexer.Span) *builder {
	b := &builder{tcx: tcx, r: r, locals: map[*typeck.Local]LocalID{}, upvars: map[*typeck.Local]*upvar{}, ovl: map[LocalID]*ovlInfo{}}
	b.body = &Body{Name: name, Span: sp, ClosureKind: -1}
	b.cur = b.newBlock()
	return b
}

// ---------------------------------------------------------------------------
// Blocks, locals, statements

func (b *builder) newBlock() int {
	b.body.Blocks = append(b.body.Blocks, &Block{Term: Term{Kind: TUnreachable}})
	return len(b.body.Blocks) - 1
}

func (b *builder) reachable() bool { return b.cur >= 0 }

func (b *builder) emit(s Stmt) {
	if b.cur < 0 {
		return
	}
	bl := b.body.Blocks[b.cur]
	bl.Stmts = append(bl.Stmts, s)
}

func (b *builder) terminate(t Term) {
	if b.cur < 0 {
		return
	}
	b.body.Blocks[b.cur].Term = t
	b.cur = -1
}

func (b *builder) gotoBlock(target int) {
	b.terminate(Term{Kind: TGoto, Targets: []int{target}})
}

func (b *builder) switchTo(targets ...int) {
	b.terminate(Term{Kind: TSwitch, Targets: targets})
}

func (b *builder) newLocal(d *LocalDecl) LocalID {
	b.body.Locals = append(b.body.Locals, d)
	return LocalID(len(b.body.Locals) - 1)
}

func (b *builder) temp(t Ty, sp lexer.Span) tplace {
	id := b.newLocal(&LocalDecl{Kind: LTemp, Ty: t, Span: sp, Mut: true})
	b.temps = append(b.temps, id)
	return tplace{p: Place{Local: id}, ty: t}
}

func (b *builder) assign(dest Place, rv Rvalue, sp lexer.Span) {
	b.emit(Stmt{Kind: SAssign, Place: dest, Rv: rv, Span: sp})
}

func (b *builder) pushTemps() { b.tempMarks = append(b.tempMarks, len(b.temps)) }

// popTemps ends the temporaries created since the matching pushTemps.
func (b *builder) popTemps(sp lexer.Span) {
	m := b.tempMarks[len(b.tempMarks)-1]
	b.tempMarks = b.tempMarks[:len(b.tempMarks)-1]
	for i := len(b.temps) - 1; i >= m; i-- {
		b.endLocal(b.temps[i], sp)
	}
	b.temps = b.temps[:m]
}

// keepTemps moves the temporaries created since the matching pushTemps to
// the enclosing scope (extended temporaries of let initializers).
func (b *builder) keepTemps() {
	m := b.tempMarks[len(b.tempMarks)-1]
	b.tempMarks = b.tempMarks[:len(b.tempMarks)-1]
	if len(b.scopes) > 0 {
		sc := b.scopes[len(b.scopes)-1]
		sc.locals = append(sc.locals, b.temps[m:]...)
	}
	b.temps = b.temps[:m]
}

func (b *builder) endLocal(l LocalID, sp lexer.Span) {
	pl := Place{Local: l}
	b.emit(Stmt{Kind: SDrop, Place: pl, Span: sp})
	b.emit(Stmt{Kind: SStorageDead, Place: pl, Span: sp})
}

func (b *builder) pushScope() { b.scopes = append(b.scopes, &scope{}) }

func (b *builder) popScope(sp lexer.Span) {
	sc := b.scopes[len(b.scopes)-1]
	b.exitScope(sc, sp)
	b.scopes = b.scopes[:len(b.scopes)-1]
}

func (b *builder) exitScope(sc *scope, sp lexer.Span) {
	for i := len(sc.locals) - 1; i >= 0; i-- {
		b.endLocal(sc.locals[i], sp)
	}
}

// exitTo emits the scope exits for jumping out to scope depth d.
func (b *builder) exitTo(d int, sp lexer.Span) {
	for i := len(b.scopes) - 1; i >= d; i-- {
		b.exitScope(b.scopes[i], sp)
	}
}

// userLocal returns the MIR local of a typeck local, declaring it in the
// current scope on first use.
func (b *builder) userLocal(l *typeck.Local) LocalID {
	if id, ok := b.locals[l]; ok {
		return id
	}
	id := b.newLocal(&LocalDecl{Kind: LVar, Ty: l.Ty, Mut: l.Mut, Name: l.Name, Span: l.Span, User: l})
	b.locals[l] = id
	if len(b.scopes) > 0 {
		sc := b.scopes[len(b.scopes)-1]
		sc.locals = append(sc.locals, id)
	}
	return id
}

// ---------------------------------------------------------------------------
// Types

func (b *builder) nodeTy(e ast.Expr) Ty { return b.r.NodeTypes[e.GetID()] }

func (b *builder) adjTy(e ast.Expr) Ty { return b.r.AdjustedType(e) }

func (b *builder) isCopy(t Ty) bool {
	if t == nil {
		return true
	}
	switch t.Kind {
	case TInt, TFloat, TBool, TChar, TNever, TFnDef, TFnPtr, TPtr:
		return true
	case TRef:
		return !t.Mut
	case TInfer, TError, TParam:
		return true
	}
	if t.HasInfer() || t.HasError() {
		return true
	}
	return b.tcx.IsCopy(t)
}

func derefKindOf(t Ty) (DerefKind, Ty, bool) {
	if t == nil {
		return 0, nil, false
	}
	switch t.Kind {
	case TRef:
		if t.Mut {
			return DRefMut, t.Elem, true
		}
		return DRef, t.Elem, true
	case TPtr:
		if t.Mut {
			return DRawMut, t.Elem, true
		}
		return DRaw, t.Elem, true
	case TAdt:
		if t.Def.LangItem == "owned_box" && len(t.Args) > 0 {
			return DBox, t.Args[0], true
		}
	}
	return 0, nil, false
}

func (tp tplace) deref() tplace {
	k, elem, ok := derefKindOf(tp.ty)
	if !ok {
		return tplace{p: tp.p.project(Proj{Kind: PDeref, Deref: DRef}), ty: tp.ty}
	}
	return tplace{p: tp.p.project(Proj{Kind: PDeref, Deref: k}), ty: elem}
}

func (b *builder) operandOf(tp tplace) Operand {
	if b.isCopy(tp.ty) {
		return Operand{Kind: OCopy, Place: tp.p, Ty: tp.ty}
	}
	return Operand{Kind: OMove, Place: tp.p, Ty: tp.ty}
}

func constOp() Operand { return Operand{Kind: OConst} }

// ---------------------------------------------------------------------------
// Places

// isPlaceExpr reports whether e (unadjusted) denotes a place.
func (b *builder) isPlaceExpr(e ast.Expr) bool {
	switch x := e.(type) {
	case *ast.PathExpr:
		vr := b.r.Paths[x.ID]
		return vr != nil && (vr.Kind == typeck.VLocal || vr.Kind == typeck.VStatic)
	case *ast.FieldExpr, *ast.IndexExpr:
		return true
	case *ast.UnaryExpr:
		return x.Op == ast.UnDeref
	case *ast.ParenExpr:
		return b.isPlaceExpr(x.X)
	}
	return false
}

// placeOf evaluates e (with its adjustments) as a place; non-place
// expressions are evaluated into a temporary.
func (b *builder) placeOf(e ast.Expr) tplace {
	tp := b.unadjPlace(e)
	return b.applyAdjusts(e, tp, b.r.Adjust[e.GetID()])
}

func (b *builder) applyAdjusts(e ast.Expr, tp tplace, adjs []typeck.Adjustment) tplace {
	for _, a := range adjs {
		tp = b.applyAdj(e, tp, a)
	}
	return tp
}

func (b *builder) applyAdj(e ast.Expr, tp tplace, a typeck.Adjustment) tplace {
	sp := e.GetSpan()
	switch a.Kind {
	case typeck.AdjDeref:
		t := tp.deref()
		if a.Target != nil {
			t.ty = a.Target
		}
		return t
	case typeck.AdjDerefOverloaded:
		return b.overloadedDeref(tp, a.Mut, a.Target, a.Callee, sp)
	case typeck.AdjBorrow:
		t := b.temp(a.Target, sp)
		b.assign(t.p, Rvalue{Kind: RRef, Place: tp.p, Mut: a.Mut}, sp)
		return t
	case typeck.AdjBorrowRaw:
		t := b.temp(a.Target, sp)
		b.assign(t.p, Rvalue{Kind: RRawRef, Place: tp.p, Mut: a.Mut}, sp)
		return t
	case typeck.AdjNeverToAny:
		t := b.temp(a.Target, sp)
		return t
	default: // unsize, fn pointer reification, &mut -> &
		t := b.temp(a.Target, sp)
		if tp.ty != nil && tp.ty.Kind == TRef && tp.ty.Mut && !(len(tp.p.Proj) == 0 && b.isTempLocal(tp.p.Local)) {
			// coercing a `&mut` place reborrows it
			mut := a.Target != nil && a.Target.Kind == TRef && a.Target.Mut
			b.assign(t.p, Rvalue{Kind: RRef, Place: tp.deref().p, Mut: mut}, sp)
			return t
		}
		b.assign(t.p, Rvalue{Kind: RUse, Ops: []Operand{b.operandOf(tp)}}, sp)
		return t
	}
}

// ovlHere records the borrow statement just emitted for an overloaded
// deref/index base.
func (b *builder) ovlHere(baseTy Ty, trait string) *ovlInfo {
	if b.cur < 0 {
		return nil
	}
	return &ovlInfo{block: b.cur, stmt: len(b.body.Blocks[b.cur].Stmts) - 1, baseTy: baseTy, trait: trait}
}

// fixupMutability makes overloaded derefs/indexes that are used mutably
// use deref_mut/index_mut (rustc's typeck writeback fixup), when the base
// type implements the mutable trait.
func (b *builder) fixupMutability() {
	var fix func(p Place)
	fix = func(p Place) {
		for i := len(p.Proj) - 1; i >= 0; i-- {
			pr := &p.Proj[i]
			if pr.Kind != PDeref {
				continue
			}
			if i != 0 {
				if pr.Deref == DRefMut || pr.Deref == DRawMut || pr.Deref == DBox {
					continue
				}
				return
			}
			info := b.ovl[p.Local]
			if info == nil || pr.Deref != DRef {
				return
			}
			if tr := b.tcx.LangOpt(info.trait); tr != nil && info.baseTy != nil && !info.baseTy.HasInfer() && !b.tcx.Implements(info.baseTy, tr) {
				return
			}
			pr.Deref = DRefMut
			ld := b.body.Locals[p.Local]
			if ld.Ty != nil && ld.Ty.Kind == TRef {
				ld.Ty = Ref(ld.Ty.Elem, true)
			}
			st := &b.body.Blocks[info.block].Stmts[info.stmt]
			if st.Rv.Kind == RRef && !st.Rv.Mut {
				st.Rv.Mut = true
				fix(st.Rv.Place)
			}
			return
		}
	}
	for _, bl := range b.body.Blocks {
		for i := range bl.Stmts {
			s := &bl.Stmts[i]
			if s.Kind != SAssign {
				continue
			}
			if len(s.Place.Proj) > 0 {
				fix(s.Place)
			}
			if s.Rv.Kind == RRef && s.Rv.Mut {
				fix(s.Rv.Place)
			}
		}
	}
}

// overloadedDeref lowers `*Deref::deref(&p)` (or deref_mut): the result
// is a place behind the returned reference.
func (b *builder) overloadedDeref(tp tplace, mut bool, target Ty, c *typeck.Callee, sp lexer.Span) tplace {
	ref := b.temp(Ref(tp.ty, mut), sp)
	b.assign(ref.p, Rvalue{Kind: RRef, Place: tp.p, Mut: mut}, sp)
	info := b.ovlHere(tp.ty, "deref_mut")
	if target == nil {
		target = tp.ty
	}
	res := b.temp(Ref(target, mut), sp)
	if info != nil {
		b.ovl[res.p.Local] = info
	}
	b.call(res.p, []Operand{{Kind: OMove, Place: ref.p}}, refFlow(), "deref", sp, false)
	return res.deref()
}

func (b *builder) unadjPlace(e ast.Expr) tplace {
	if b.closure != nil && b.closure.CapExprs != nil {
		if i, ok := b.closure.CapExprs[e.GetID()]; ok {
			return b.upvarPlace(b.capUpvars[i], b.nodeTy(e))
		}
	}
	switch x := e.(type) {
	case *ast.ParenExpr:
		return b.placeOf(x.X)
	case *ast.PathExpr:
		vr := b.r.Paths[x.ID]
		if vr != nil && vr.Kind == typeck.VLocal {
			return b.localPlace(vr.Local)
		}
		if vr != nil && vr.Kind == typeck.VStatic {
			return b.staticPlace(vr.Def, b.nodeTy(e))
		}
	case *ast.FieldExpr:
		fr, ok := b.r.Fields[x.ID]
		if !ok {
			break
		}
		base := b.placeOf(x.X)
		variant := 0
		return tplace{p: base.p.project(Proj{Kind: PField, Field: fr.Index, Variant: variant, Name: x.Name.Name}), ty: b.nodeTy(e)}
	case *ast.IndexExpr:
		if b.r.IndexBuiltin[x.ID] {
			base := b.placeOf(x.X)
			idx := b.operandToLocal(x.Index)
			return tplace{p: base.p.project(Proj{Kind: PIndex, Index: idx}), ty: b.nodeTy(e)}
		}
		if c := b.r.Callees[x.ID]; c != nil {
			mut := b.r.MutPlace[x.ID]
			base := b.placeOf(x.X)
			idx := b.operandOf(b.valueTemp(x.Index))
			ref := b.temp(Ref(base.ty, mut), e.GetSpan())
			b.assign(ref.p, Rvalue{Kind: RRef, Place: base.p, Mut: mut}, e.GetSpan())
			info := b.ovlHere(base.ty, "index_mut")
			res := b.temp(Ref(b.nodeTy(e), mut), e.GetSpan())
			if info != nil {
				b.ovl[res.p.Local] = info
			}
			b.call(res.p, []Operand{{Kind: OMove, Place: ref.p}, idx}, refFlow(), "index", e.GetSpan(), false)
			return res.deref()
		}
	case *ast.UnaryExpr:
		if x.Op == ast.UnDeref {
			if c := b.r.Callees[x.ID]; c != nil {
				base := b.placeOf(x.X)
				return b.overloadedDeref(base, b.r.MutPlace[x.ID], b.nodeTy(e), c, e.GetSpan())
			}
			base := b.placeOf(x.X)
			t := base.deref()
			if nt := b.nodeTy(e); nt != nil {
				t.ty = nt
			}
			return t
		}
	}
	return b.valueTempUnadj(e)
}

// staticPlace returns the place of a static item (a local that lives for
// the whole program).
func (b *builder) staticPlace(d *Def, t Ty) tplace {
	if b.statics == nil {
		b.statics = map[*Def]LocalID{}
	}
	id, ok := b.statics[d]
	if !ok {
		mut := false
		if d.Const != nil {
			mut = d.Const.Mut
		}
		id = b.newLocal(&LocalDecl{Kind: LStatic, Ty: t, Name: d.Name, Mut: mut})
		b.statics[d] = id
	}
	return tplace{p: Place{Local: id}, ty: t}
}

// operandToLocal evaluates an index expression into a local.
func (b *builder) operandToLocal(e ast.Expr) LocalID {
	t := b.temp(b.adjTy(e), e.GetSpan())
	b.exprInto(e, t)
	return t.p.Local
}

// localPlace returns the place of a typeck local (an upvar access in
// closure bodies).
func (b *builder) localPlace(l *typeck.Local) tplace {
	if u, ok := b.upvars[l]; ok {
		return b.upvarPlace(u, l.Ty)
	}
	return tplace{p: Place{Local: b.userLocal(l)}, ty: l.Ty}
}

// upvarPlace is the place of a captured place of type ty in a closure body.
func (b *builder) upvarPlace(u *upvar, ty Ty) tplace {
	pl := Place{Local: u.local}
	if u.byRef {
		k := DRef
		if u.mutRef {
			k = DRefMut
		}
		return tplace{p: pl.project(Proj{Kind: PDeref, Deref: k}), ty: ty}
	}
	// by-value capture accessed through the closure's self
	switch b.body.ClosureKind {
	case 0:
		return tplace{p: pl.project(Proj{Kind: PDeref, Deref: DRef}), ty: ty}
	case 1:
		return tplace{p: pl.project(Proj{Kind: PDeref, Deref: DRefMut}), ty: ty}
	}
	return tplace{p: pl, ty: ty}
}

// capturedPlace is the place a closure created in this body captures.
func (b *builder) capturedPlace(c *typeck.Capture) tplace {
	var tp tplace
	rest := c.Path
	switch {
	case b.upvars[c.Local] != nil || b.closure == nil:
		tp = b.localPlace(c.Local)
	default:
		// a part of a place this closure body captured itself
		found := false
		for j, oc := range b.closure.Captures {
			if oc.Local == c.Local && typeck.PathPrefix(oc.Path, c.Path) {
				tp = b.upvarPlace(b.capUpvars[j], oc.Ty)
				rest = c.Path[len(oc.Path):]
				found = true
				break
			}
		}
		if !found {
			tp = b.localPlace(c.Local)
		}
	}
	for _, pr := range rest {
		if pr.Deref {
			tp = tp.deref()
			tp.ty = pr.Ty
			continue
		}
		tp = tplace{p: tp.p.project(Proj{Kind: PField, Field: pr.Field, Name: pr.Name}), ty: pr.Ty}
	}
	return tp
}

// valueTemp evaluates e (adjusted) into a fresh temporary.
func (b *builder) valueTemp(e ast.Expr) tplace {
	if b.isPlaceExpr(e) || len(b.r.Adjust[e.GetID()]) > 0 {
		tp := b.placeOf(e)
		if len(tp.p.Proj) == 0 && b.isTempLocal(tp.p.Local) {
			return tp
		}
		t := b.temp(tp.ty, e.GetSpan())
		b.assign(t.p, Rvalue{Kind: RUse, Ops: []Operand{b.operandOf(tp)}}, e.GetSpan())
		return t
	}
	return b.valueTempUnadj(e)
}

func (b *builder) valueTempUnadj(e ast.Expr) tplace {
	if b.promotable(e) {
		// a constant expression borrowed here is promoted to a static
		id := b.newLocal(&LocalDecl{Kind: LStatic, Ty: b.nodeTy(e), Span: e.GetSpan(), Mut: true})
		t := tplace{p: Place{Local: id}, ty: b.nodeTy(e)}
		b.assign(t.p, Rvalue{Kind: RUse, Ops: []Operand{constOp()}}, e.GetSpan())
		return t
	}
	t := b.temp(b.nodeTy(e), e.GetSpan())
	b.lowerValue(e, t)
	return t
}

// promotable reports whether e is a constant expression that rustc
// promotes to a static when borrowed (`&[1, 2, 3]`, `&None`, `&P { x: 1 }`).
func (b *builder) promotable(e ast.Expr) bool {
	if pe, ok := e.(*ast.PathExpr); ok {
		// a unit variant/struct is promotable whatever its type arguments
		if vr := b.r.Paths[pe.ID]; vr != nil && vr.Kind == typeck.VUnitCtor {
			return true
		}
	}
	if ae, ok := e.(*ast.ArrayExpr); ok && len(ae.Elems) == 0 {
		return true // `&mut []`
	}
	if !b.promotableTy(b.nodeTy(e)) {
		return false
	}
	switch x := e.(type) {
	case *ast.LitExpr:
		return true
	case *ast.ParenExpr:
		return b.promotable(x.X)
	case *ast.PathExpr:
		vr := b.r.Paths[x.ID]
		return vr != nil && (vr.Kind == typeck.VConst || vr.Kind == typeck.VUnitCtor || vr.Kind == typeck.VFn || vr.Kind == typeck.VConstParam)
	case *ast.TupleExpr:
		return b.allPromotable(x.Elems)
	case *ast.ArrayExpr:
		return b.allPromotable(x.Elems)
	case *ast.RepeatExpr:
		return b.promotable(x.Elem)
	case *ast.StructExpr:
		if x.Base != nil {
			return false
		}
		for _, f := range x.Fields {
			if !b.promotable(f.Expr) {
				return false
			}
		}
		return true
	case *ast.CallExpr:
		if c := b.r.Callees[x.ID]; c != nil && c.Kind == typeck.CallCtor {
			return b.allPromotable(x.Args)
		} else if c != nil && c.Kind == typeck.CallFn && c.Def != nil && c.Def.HasAttr("rustc_promotable") {
			return b.allPromotable(x.Args)
		}
	case *ast.UnaryExpr:
		if x.Op != ast.UnDeref && b.r.Callees[x.ID] == nil {
			return b.promotable(x.X)
		}
	case *ast.BinaryExpr:
		if b.r.Callees[x.ID] == nil && x.Op != ast.BinDiv && x.Op != ast.BinRem && x.Op != ast.BinAnd && x.Op != ast.BinOr {
			return b.promotable(x.L) && b.promotable(x.R)
		}
	case *ast.CastExpr:
		return b.promotable(x.X)
	}
	return false
}

func (b *builder) allPromotable(es []ast.Expr) bool {
	for _, e := range es {
		for _, a := range b.r.Adjust[e.GetID()] {
			if a.Kind != typeck.AdjReifyFnPtr {
				return false
			}
		}
		if !b.promotable(e) {
			return false
		}
	}
	return true
}

// promotableTy: no interior mutability, no drop glue.
func (b *builder) promotableTy(t Ty) bool {
	if t == nil {
		return false
	}
	switch t.Kind {
	case TInt, TFloat, TBool, TChar, TStr, TNever, TFnDef, TFnPtr:
		return true
	case TRef:
		return !t.Mut
	case TTuple:
		for _, e := range t.Elems {
			if !b.promotableTy(e) {
				return false
			}
		}
		return true
	case TArray, TSlice:
		return b.promotableTy(t.Elem)
	case TAdt:
		d := t.Def
		if d.LangItem == "unsafe_cell" || d.LangItem == "owned_box" {
			return false
		}
		if d.Crate != nil && d.Crate.Name == "std" {
			switch d.Name {
			case "Cell", "RefCell", "UnsafeCell", "OnceCell", "Mutex", "RwLock", "String", "Vec":
				return false
			}
		}
		if dr := b.tcx.LangOpt("drop"); dr != nil && b.tcx.Implements(t, dr) {
			return false
		}
		b.tcx.AdtFields(d)
		if d.Adt == nil {
			return false
		}
		for vi := range d.Adt.Variants {
			for _, ft := range b.fieldTypes(t, vi) {
				if !b.promotableTy(ft) {
					return false
				}
			}
		}
		return true
	}
	return false
}

func (b *builder) isTempLocal(l LocalID) bool { return b.body.Locals[l].Kind == LTemp }

// operand evaluates e (adjusted) to an operand.
func (b *builder) operand(e ast.Expr) Operand {
	if lit, ok := e.(*ast.LitExpr); ok && len(b.r.Adjust[lit.ID]) == 0 {
		return constOp()
	}
	if b.isPlaceExpr(e) || len(b.r.Adjust[e.GetID()]) > 0 {
		return b.operandOf(b.placeOf(e))
	}
	t := b.valueTempUnadj(e)
	return Operand{Kind: OMove, Place: t.p}
}

// operandNow evaluates e to an operand read now (a place operand is copied
// or moved into a temporary immediately, before later evaluation).
func (b *builder) operandNow(e ast.Expr) Operand {
	op := b.operand(e)
	if op.Kind == OConst || (len(op.Place.Proj) == 0 && b.isTempLocal(op.Place.Local)) {
		return op
	}
	t := b.temp(b.adjTy(e), e.GetSpan())
	b.assign(t.p, Rvalue{Kind: RUse, Ops: []Operand{op}}, e.GetSpan())
	return Operand{Kind: OMove, Place: t.p}
}

// argOperand evaluates a call argument: a `&mut` place passed to a call is
// implicitly reborrowed (`&mut *x`), not moved.
func (b *builder) argOperand(e ast.Expr) Operand {
	t := b.adjTy(e)
	if t != nil && t.Kind == TRef && t.Mut && (b.isPlaceExpr(e) || len(b.r.Adjust[e.GetID()]) > 0) {
		tp := b.placeOf(e)
		return b.reborrow(tp, e.GetSpan())
	}
	return b.operand(e)
}

func (b *builder) reborrow(tp tplace, sp lexer.Span) Operand {
	if tp.ty == nil || tp.ty.Kind != TRef || !tp.ty.Mut || (len(tp.p.Proj) == 0 && b.isTempLocal(tp.p.Local)) {
		return b.operandOf(tp)
	}
	t := b.temp(tp.ty, sp)
	b.assign(t.p, Rvalue{Kind: RRef, Place: tp.deref().p, Mut: true}, sp)
	return Operand{Kind: OMove, Place: t.p}
}

// exprInto evaluates e (adjusted) and stores it into dest.
func (b *builder) exprInto(e ast.Expr, dest tplace) {
	if b.isPlaceExpr(e) || len(b.r.Adjust[e.GetID()]) > 0 {
		tp := b.placeOf(e)
		if dest.ty != nil && dest.ty.Kind == TRef && !dest.ty.Mut && tp.ty != nil && tp.ty.Kind == TRef && tp.ty.Mut && !(len(tp.p.Proj) == 0 && b.isTempLocal(tp.p.Local)) {
			// `&mut T` coerced to `&T`: a shared reborrow
			b.assign(dest.p, Rvalue{Kind: RRef, Place: tp.deref().p}, e.GetSpan())
			return
		}
		b.assign(dest.p, Rvalue{Kind: RUse, Ops: []Operand{b.operandOf(tp)}}, e.GetSpan())
		return
	}
	b.lowerValue(e, dest)
}

// ---------------------------------------------------------------------------
// Calls

func (b *builder) call(dest Place, args []Operand, flow *Flow, name string, sp lexer.Span, diverges bool) {
	if b.cur < 0 {
		return
	}
	t := Term{Kind: TCall, Args: args, Dest: dest, Flow: flow, Span: sp, Name: name}
	if diverges {
		b.terminate(t)
		return
	}
	next := b.newBlock()
	t.Targets = []int{next}
	b.terminate(t)
	b.cur = next
}

func (b *builder) callTo(c *typeck.Callee, args []Operand, dest tplace, sp lexer.Span, name string) {
	flow := b.flowFor(c, len(args), dest.ty)
	diverges := dest.ty != nil && dest.ty.Kind == TNever
	b.call(dest.p, args, flow, name, sp, diverges)
}

// ---------------------------------------------------------------------------
// Values

func (b *builder) lowerValue(e ast.Expr, dest tplace) {
	sp := e.GetSpan()
	switch x := e.(type) {
	case *ast.LitExpr:
		b.assign(dest.p, Rvalue{Kind: RUse, Ops: []Operand{constOp()}}, sp)
	case *ast.PathExpr:
		vr := b.r.Paths[x.ID]
		if vr != nil && vr.Kind == typeck.VLocal {
			b.assign(dest.p, Rvalue{Kind: RUse, Ops: []Operand{b.operandOf(b.localPlace(vr.Local))}}, sp)
			return
		}
		b.assign(dest.p, Rvalue{Kind: RUse, Ops: []Operand{constOp()}}, sp)
	case *ast.ParenExpr:
		b.exprInto(x.X, dest)
	case *ast.FieldExpr, *ast.IndexExpr:
		tp := b.unadjPlace(e)
		b.assign(dest.p, Rvalue{Kind: RUse, Ops: []Operand{b.operandOf(tp)}}, sp)
	case *ast.UnaryExpr:
		if x.Op == ast.UnDeref {
			tp := b.unadjPlace(e)
			b.assign(dest.p, Rvalue{Kind: RUse, Ops: []Operand{b.operandOf(tp)}}, sp)
			return
		}
		if c := b.r.Callees[x.ID]; c != nil {
			op := b.operand(x.X)
			b.callTo(c, []Operand{op}, dest, sp, "unop")
			return
		}
		op := b.operand(x.X)
		b.assign(dest.p, Rvalue{Kind: ROther, Ops: []Operand{op}}, sp)
	case *ast.RefExpr:
		tp := b.placeOf(x.X)
		kind := RRef
		if x.Raw {
			kind = RRawRef
		}
		b.assign(dest.p, Rvalue{Kind: kind, Place: tp.p, Mut: x.Mut}, sp)
	case *ast.BinaryExpr:
		b.binary(x, dest)
	case *ast.AssignExpr:
		b.assignExpr(x)
		b.assign(dest.p, Rvalue{Kind: RUse, Ops: []Operand{constOp()}}, sp)
	case *ast.AssignOpExpr:
		b.assignOp(x)
		b.assign(dest.p, Rvalue{Kind: RUse, Ops: []Operand{constOp()}}, sp)
	case *ast.CallExpr:
		b.callExpr(x, dest)
	case *ast.MethodCallExpr:
		b.methodCall(x, dest)
	case *ast.CastExpr:
		op := b.operand(x.X)
		kind := ROther
		if t := b.nodeTy(e); t != nil && (t.Kind == TRef || t.Kind == TAdt) {
			kind = RUse
		}
		b.assign(dest.p, Rvalue{Kind: kind, Ops: []Operand{op}}, sp)
	case *ast.BlockExpr:
		b.blockExpr(x, dest)
	case *ast.IfExpr:
		b.ifExpr(x, dest)
	case *ast.MatchExpr:
		b.matchExpr(x, dest)
	case *ast.WhileExpr:
		b.whileExpr(x)
		b.assign(dest.p, Rvalue{Kind: RUse, Ops: []Operand{constOp()}}, sp)
	case *ast.LoopExpr:
		b.loopExpr(x, dest)
	case *ast.ForExpr:
		b.forExpr(x)
		b.assign(dest.p, Rvalue{Kind: RUse, Ops: []Operand{constOp()}}, sp)
	case *ast.BreakExpr:
		b.breakExpr(x)
	case *ast.ContinueExpr:
		b.continueExpr(x)
	case *ast.ReturnExpr:
		b.returnExpr(x.Value, sp)
	case *ast.TupleExpr:
		b.aggregate(x.Elems, dest, sp)
	case *ast.ArrayExpr:
		b.aggregate(x.Elems, dest, sp)
	case *ast.RepeatExpr:
		op := b.operand(x.Elem)
		b.assign(dest.p, Rvalue{Kind: RAggregate, Ops: []Operand{op}}, sp)
	case *ast.RangeExpr:
		var es []ast.Expr
		if x.Start != nil {
			es = append(es, x.Start)
		}
		if x.End != nil {
			es = append(es, x.End)
		}
		b.aggregate(es, dest, sp)
	case *ast.StructExpr:
		b.structExpr(x, dest)
	case *ast.ClosureExpr:
		b.closureExpr(x, dest)
	case *ast.TryExpr:
		b.tryExpr(x, dest)
	case *ast.FormatArgsExpr:
		var ops []Operand
		for _, a := range x.Args {
			tp := b.placeOf(a)
			t := b.temp(Ref(tp.ty, false), a.GetSpan())
			b.assign(t.p, Rvalue{Kind: RRef, Place: tp.p}, a.GetSpan())
			ops = append(ops, Operand{Kind: OMove, Place: t.p})
		}
		b.assign(dest.p, Rvalue{Kind: RAggregate, Ops: ops}, sp)
	case *ast.AwaitExpr:
		op := b.operand(x.X)
		b.assign(dest.p, Rvalue{Kind: ROther, Ops: []Operand{op}}, sp)
	case *ast.AsmExpr:
		b.asmExpr(x)
		b.assign(dest.p, Rvalue{Kind: RUse, Ops: []Operand{constOp()}}, sp)
	case *ast.LetExpr:
		// `let` outside a condition: evaluate the scrutinee
		b.operand(x.Expr)
		b.assign(dest.p, Rvalue{Kind: RUse, Ops: []Operand{constOp()}}, sp)
	default:
		b.assign(dest.p, Rvalue{Kind: RUse, Ops: []Operand{constOp()}}, sp)
	}
}

func (b *builder) aggregate(es []ast.Expr, dest tplace, sp lexer.Span) {
	var ops []Operand
	for _, e := range es {
		ops = append(ops, b.argOperand(e))
	}
	b.assign(dest.p, Rvalue{Kind: RAggregate, Ops: ops}, sp)
}

func (b *builder) binary(x *ast.BinaryExpr, dest tplace) {
	sp := x.Span
	if c := b.r.Callees[x.ID]; c != nil {
		if x.Op >= ast.BinEq {
			// comparisons take both operands by reference
			l := b.placeOf(x.L)
			r := b.placeOf(x.R)
			lr := b.temp(Ref(l.ty, false), sp)
			b.assign(lr.p, Rvalue{Kind: RRef, Place: l.p}, sp)
			rr := b.temp(Ref(r.ty, false), sp)
			b.assign(rr.p, Rvalue{Kind: RRef, Place: r.p}, sp)
			b.call(dest.p, []Operand{{Kind: OMove, Place: lr.p}, {Kind: OMove, Place: rr.p}}, noFlow(), "cmp", sp, false)
			return
		}
		l := b.operand(x.L)
		r := b.operand(x.R)
		b.callTo(c, []Operand{l, r}, dest, sp, "binop")
		return
	}
	if x.Op == ast.BinAnd || x.Op == ast.BinOr {
		// short circuit: the right operand runs conditionally
		t := b.temp(TyBool, sp)
		l := b.operand(x.L)
		b.assign(t.p, Rvalue{Kind: ROther, Ops: []Operand{l}}, sp)
		rhs, end := b.newBlock(), b.newBlock()
		b.switchTo(rhs, end)
		b.cur = rhs
		r := b.operand(x.R)
		b.assign(t.p, Rvalue{Kind: ROther, Ops: []Operand{r}}, sp)
		b.gotoBlock(end)
		b.cur = end
		b.assign(dest.p, Rvalue{Kind: ROther, Ops: []Operand{{Kind: OCopy, Place: t.p}}}, sp)
		return
	}
	l := b.operand(x.L)
	r := b.operand(x.R)
	b.assign(dest.p, Rvalue{Kind: ROther, Ops: []Operand{l, r}}, sp)
}

func (b *builder) assignExpr(x *ast.AssignExpr) {
	sp := x.Span
	switch l := x.L.(type) {
	case *ast.TupleExpr, *ast.ArrayExpr, *ast.StructExpr, *ast.CallExpr, *ast.UnderscoreExpr:
		// destructuring assignment
		v := b.valueTemp(x.R)
		b.destructure(l, v)
		return
	}
	op := b.operandNow(x.R)
	tp := b.placeOf(x.L)
	b.emit(Stmt{Kind: SDrop, Place: tp.p, Span: sp})
	b.assign(tp.p, Rvalue{Kind: RUse, Ops: []Operand{op}}, sp)
}

func (b *builder) destructure(l ast.Expr, v tplace) {
	switch x := l.(type) {
	case *ast.UnderscoreExpr:
		return
	case *ast.TupleExpr:
		for i, e := range x.Elems {
			if _, ok := e.(*ast.RangeExpr); ok {
				continue
			}
			ft := b.nodeTy(e)
			b.destructure(e, tplace{p: v.p.project(Proj{Kind: PField, Field: i}), ty: ft})
		}
		return
	case *ast.ParenExpr:
		b.destructure(x.X, v)
		return
	}
	if !b.isPlaceExpr(l) {
		b.operand(l)
		return
	}
	tp := b.placeOf(l)
	b.assign(tp.p, Rvalue{Kind: RUse, Ops: []Operand{b.operandOf(v)}}, l.GetSpan())
}

func (b *builder) assignOp(x *ast.AssignOpExpr) {
	sp := x.Span
	if c := b.r.Callees[x.ID]; c != nil {
		r := b.operand(x.R)
		l := b.placeOf(x.L)
		ref := b.temp(Ref(l.ty, true), sp)
		b.assign(ref.p, Rvalue{Kind: RRef, Place: l.p, Mut: true}, sp)
		t := b.temp(TyUnit, sp)
		b.call(t.p, []Operand{{Kind: OMove, Place: ref.p}, r}, noFlow(), "assign_op", sp, false)
		return
	}
	r := b.operandNow(x.R)
	l := b.placeOf(x.L)
	b.assign(l.p, Rvalue{Kind: ROther, Ops: []Operand{{Kind: OCopy, Place: l.p}, r}}, sp)
}

func (b *builder) callExpr(x *ast.CallExpr, dest tplace) {
	sp := x.Span
	c := b.r.Callees[x.ID]
	if c == nil {
		var ops []Operand
		for _, a := range x.Args {
			ops = append(ops, b.operand(a))
		}
		b.call(dest.p, ops, noFlow(), "?", sp, false)
		return
	}
	switch c.Kind {
	case typeck.CallCtor:
		b.aggregate(x.Args, dest, sp)
		return
	case typeck.CallClosure, typeck.CallDynFn, typeck.CallFnPtr:
		fp := b.placeOf(x.Fn)
		var callee Operand
		kind := 2
		switch c.Kind {
		case typeck.CallClosure:
			kind = 0
			if ci := b.tcx.Closures[c.Ty]; ci != nil && ci.Results == b.r {
				// a closure of this body: its own kind (a closure from
				// elsewhere is called through a generic bound we don't see)
				kind = ci.Kind
			}
		case typeck.CallDynFn:
			kind = dynFnKind(c.Ty)
		case typeck.CallFnPtr:
			kind = 0
		}
		switch kind {
		case 0, 1:
			ref := b.temp(Ref(fp.ty, kind == 1), sp)
			b.assign(ref.p, Rvalue{Kind: RRef, Place: fp.p, Mut: kind == 1}, sp)
			callee = Operand{Kind: OMove, Place: ref.p}
		default:
			callee = b.operandOf(fp)
		}
		ops := []Operand{callee}
		for _, a := range x.Args {
			ops = append(ops, b.argOperand(a))
		}
		diverges := dest.ty != nil && dest.ty.Kind == TNever
		// the output of a Fn* call cannot borrow from the closure itself
		fl := noFlow()
		if carriesBorrows(dest.ty) {
			for i := 1; i < len(ops); i++ {
				fl.Edges = append(fl.Edges, FlowEdge{Arg: i, From: 0, To: 0, Deep: true})
			}
		}
		b.call(dest.p, ops, fl, "closure", sp, diverges)
		return
	}
	var ops []Operand
	if c.Kind == typeck.CallFn || c.Kind == typeck.CallVirtual || c.Kind == typeck.CallBuiltin {
		// the callee path itself is a constant
		if _, isPath := unparen(x.Fn).(*ast.PathExpr); !isPath {
			b.operand(x.Fn)
		}
	}
	for _, a := range x.Args {
		ops = append(ops, b.argOperand(a))
	}
	b.callTo(c, ops, dest, sp, defName(c))
}

func defName(c *typeck.Callee) string {
	if c.Def != nil {
		return c.Def.Name
	}
	return "?"
}

func dynFnKind(t Ty) int {
	if t != nil && t.Kind == TDyn && t.Def != nil {
		switch t.Def.Name {
		case "Fn":
			return 0
		case "FnMut":
			return 1
		}
	}
	return 2
}

func (b *builder) methodCall(x *ast.MethodCallExpr, dest tplace) {
	sp := x.Span
	c := b.r.Callees[x.ID]
	adjs := b.r.Adjust[x.Receiver.GetID()]
	// two-phase borrows: an autoref of the receiver is taken after the
	// arguments are evaluated
	var last *typeck.Adjustment
	if n := len(adjs); n > 0 && adjs[n-1].Kind == typeck.AdjBorrow {
		last = &adjs[n-1]
		adjs = adjs[:n-1]
	}
	recv := b.applyAdjusts(x.Receiver, b.unadjPlace(x.Receiver), adjs)
	isTemp := len(recv.p.Proj) == 0 && b.isTempLocal(recv.p.Local)
	mutRecv := recv.ty != nil && recv.ty.Kind == TRef && recv.ty.Mut && !isTemp
	if last == nil && !isTemp && !mutRecv {
		// by-value receiver: read it now
		t := b.temp(recv.ty, sp)
		b.assign(t.p, Rvalue{Kind: RUse, Ops: []Operand{b.operandOf(recv)}}, sp)
		recv = t
	}
	var ops []Operand
	for _, a := range x.Args {
		ops = append(ops, b.argOperand(a))
	}
	var r Operand
	if last == nil && mutRecv {
		// a `&mut` receiver is reborrowed (two-phase: after the arguments)
		r = b.reborrow(recv, sp)
	} else if last != nil {
		ref := b.temp(last.Target, sp)
		b.assign(ref.p, Rvalue{Kind: RRef, Place: recv.p, Mut: last.Mut}, sp)
		r = Operand{Kind: OMove, Place: ref.p}
	} else {
		r = b.operandOf(recv)
	}
	ops = append([]Operand{r}, ops...)
	if c == nil {
		b.call(dest.p, ops, noFlow(), x.Method.Name, sp, false)
		return
	}
	b.callTo(c, ops, dest, sp, x.Method.Name)
}

func (b *builder) structExpr(x *ast.StructExpr, dest tplace) {
	sp := x.Span
	var ops []Operand
	for _, f := range x.Fields {
		ops = append(ops, b.argOperand(f.Expr))
	}
	if x.Base != nil {
		v := b.r.Variants[x.ID]
		given := map[int]bool{}
		if v != nil {
			for _, f := range x.Fields {
				given[v.FieldIndex(f.Name.Name)] = true
			}
		}
		base := b.placeOf(x.Base)
		bt := base.ty
		if v != nil && bt != nil && bt.Kind == TAdt {
			fts := b.fieldTypes(bt, 0)
			for i, ft := range fts {
				if given[i] {
					continue
				}
				ops = append(ops, b.operandOf(tplace{p: base.p.project(Proj{Kind: PField, Field: i}), ty: ft}))
			}
		} else {
			ops = append(ops, b.operandOf(base))
		}
	}
	b.assign(dest.p, Rvalue{Kind: RAggregate, Ops: ops}, sp)
}

func (b *builder) fieldTypes(t Ty, vi int) []Ty {
	d := t.Def
	b.tcx.AdtFields(d)
	if d.Adt == nil || vi >= len(d.Adt.Variants) {
		return nil
	}
	v := d.Adt.Variants[vi]
	out := make([]Ty, len(v.Fields))
	for i, f := range v.Fields {
		out[i] = b.tcx.Normalize(Subst(f.Ty, t.Args))
	}
	return out
}

func (b *builder) asmExpr(x *ast.AsmExpr) {
	for _, o := range x.Operands {
		if o.In != nil && o.Kind != "sym" && o.Kind != "const" {
			b.operand(o.In)
		}
		if o.Out != nil {
			if b.isPlaceExpr(o.Out) {
				tp := b.placeOf(o.Out)
				b.assign(tp.p, Rvalue{Kind: RUse, Ops: []Operand{constOp()}}, x.Span)
			}
		}
	}
}

// ---------------------------------------------------------------------------
// Closures

func (b *builder) closureExpr(x *ast.ClosureExpr, dest tplace) {
	sp := x.Span
	ci := b.r.Closures[x.ID]
	if ci == nil {
		b.assign(dest.p, Rvalue{Kind: RUse, Ops: []Operand{constOp()}}, sp)
		return
	}
	// build the closure body now: the capture modes follow its uses
	cb := buildClosure(b.tcx, b.r, ci, b.name+"::{closure}")
	b.closures = append(b.closures, cb.body)
	b.closures = append(b.closures, cb.closures...)
	needMut := cb.upvarMutUses()
	var ops []Operand
	for _, c := range ci.Captures {
		cp := b.capturedPlace(c)
		if c.ByRef {
			// a mutably captured `&mut` place is captured as `*x` (a
			// unique reborrow: `x` itself need not be mutable)
			if cp.ty != nil && cp.ty.Kind == TRef {
				cp = cp.deref()
			}
			mut := c.Mut && needMut[c.Local] // only read: a shared borrow suffices
			t := b.temp(Ref(cp.ty, mut), sp)
			b.assign(t.p, Rvalue{Kind: RRef, Place: cp.p, Mut: mut}, sp)
			ops = append(ops, Operand{Kind: OMove, Place: t.p})
			continue
		}
		ops = append(ops, b.operandOf(cp))
	}
	b.assign(dest.p, Rvalue{Kind: RAggregate, Ops: ops}, sp)
}

// upvarMutUses reports, per captured variable of a closure body, whether
// the body uses it mutably (assignment, `&mut` borrow, or moving a captured
// `&mut` out).
func (b *builder) upvarMutUses() map[*typeck.Local]bool {
	out := map[*typeck.Local]bool{}
	byLocal := map[LocalID]*typeck.Local{}
	for l, u := range b.upvars {
		byLocal[u.local] = l
	}
	if b.closure != nil {
		for i, u := range b.capUpvars {
			byLocal[u.local] = b.closure.Captures[i].Local
		}
	}
	mark := func(p Place) {
		if l, ok := byLocal[p.Local]; ok {
			out[l] = true
		}
	}
	for _, bl := range b.body.Blocks {
		for _, s := range bl.Stmts {
			if s.Kind != SAssign {
				continue
			}
			if len(s.Place.Proj) > 0 {
				mark(s.Place)
			}
			if s.Rv.Kind == RRef && s.Rv.Mut {
				mark(s.Rv.Place)
			}
			for _, o := range s.Rv.Ops {
				if o.Kind == OMove {
					mark(o.Place)
				}
			}
		}
		if bl.Term.Kind == TCall {
			for _, o := range bl.Term.Args {
				if o.Kind == OMove {
					mark(o.Place)
				}
			}
		}
	}
	return out
}

// ---------------------------------------------------------------------------
// Control flow

func (b *builder) blockExpr(x *ast.BlockExpr, dest tplace) {
	if x.Const || x.Async {
		b.assign(dest.p, Rvalue{Kind: RUse, Ops: []Operand{constOp()}}, x.Span)
		return
	}
	if x.Label != "" {
		end := b.newBlock()
		lc := &loopCtx{label: x.Label, brk: end, cont: -1, depth: len(b.scopes), dest: &dest, isBlock: true}
		b.loops = append(b.loops, lc)
		b.block(x.Block, dest)
		b.loops = b.loops[:len(b.loops)-1]
		b.gotoBlock(end)
		b.cur = end
		return
	}
	b.block(x.Block, dest)
}

func blockTail(bl *ast.Block) ast.Expr {
	if len(bl.Stmts) == 0 {
		return nil
	}
	if es, ok := bl.Stmts[len(bl.Stmts)-1].(*ast.ExprStmt); ok && !es.Semi {
		return es.Expr
	}
	return nil
}

func (b *builder) block(bl *ast.Block, dest tplace) {
	if bl == nil {
		return
	}
	b.pushScope()
	tail := blockTail(bl)
	n := len(bl.Stmts)
	if tail != nil {
		n--
	}
	for _, s := range bl.Stmts[:n] {
		b.stmt(s)
	}
	if tail != nil {
		b.exprInto(tail, dest)
	} else {
		b.assign(dest.p, Rvalue{Kind: RUse, Ops: []Operand{constOp()}}, bl.Span)
	}
	b.popScope(bl.Span)
}

func (b *builder) stmt(s ast.Stmt) {
	switch x := s.(type) {
	case *ast.LetStmt:
		b.letStmt(x)
	case *ast.ExprStmt:
		b.pushTemps()
		t := b.temp(b.adjTy(x.Expr), x.Expr.GetSpan())
		b.exprInto(x.Expr, t)
		b.popTemps(x.Expr.GetSpan())
	}
}

func (b *builder) letStmt(x *ast.LetStmt) {
	if x.Init == nil {
		b.declarePat(x.Pat)
		return
	}
	b.pushTemps()
	sp := x.Span
	if ip, ok := x.Pat.(*ast.IdentPat); ok && ip.Sub == nil && x.Else == nil && len(b.r.PatDerefs[ip.ID]) == 0 {
		if l := b.r.Locals[ip.ID]; l != nil && !l.ByRef {
			id := b.userLocal(l)
			if x.Ty != nil && l.Ty != nil && l.Ty.Kind == TRef && l.Ty.Mut {
				// `let r: &mut T = v;` reborrows
				b.assign(Place{Local: id}, Rvalue{Kind: RUse, Ops: []Operand{b.argOperand(x.Init)}}, sp)
				b.endLetTemps(x.Init, sp)
				return
			}
			b.exprInto(x.Init, tplace{p: Place{Local: id}, ty: l.Ty})
			b.endLetTemps(x.Init, sp)
			return
		}
	}
	var src tplace
	if b.isPlaceExpr(x.Init) || len(b.r.Adjust[x.Init.GetID()]) > 0 {
		src = b.placeOf(x.Init)
	} else {
		src = b.valueTempUnadj(x.Init)
	}
	if x.Else != nil {
		bind, els := b.newBlock(), b.newBlock()
		b.emit(Stmt{Kind: SFakeRead, Place: src.p, Span: sp})
		b.switchTo(bind, els)
		b.cur = els
		t := b.temp(TyUnit, sp)
		b.block(x.Else, t)
		b.terminate(Term{Kind: TUnreachable})
		b.cur = bind
	}
	b.declarePat(x.Pat)
	if _, wild := x.Pat.(*ast.WildPat); wild {
		b.emit(Stmt{Kind: SFakeRead, Place: src.p, Span: sp})
	}
	b.bindPat(x.Pat, src, false)
	b.endLetTemps(x.Init, sp)
}

// endLetTemps ends the temporaries of a let initializer, except those
// whose lifetime is extended to the enclosing block (`let r = &temp();`).
func (b *builder) endLetTemps(init ast.Expr, sp lexer.Span) {
	if extendsTemps(init) {
		b.keepTemps()
		return
	}
	b.popTemps(sp)
}

func extendsTemps(e ast.Expr) bool {
	switch x := e.(type) {
	case *ast.RefExpr:
		return true
	case *ast.ParenExpr:
		return extendsTemps(x.X)
	case *ast.CastExpr:
		return extendsTemps(x.X)
	case *ast.TupleExpr:
		for _, el := range x.Elems {
			if extendsTemps(el) {
				return true
			}
		}
	case *ast.ArrayExpr:
		for _, el := range x.Elems {
			if extendsTemps(el) {
				return true
			}
		}
	case *ast.StructExpr:
		for _, f := range x.Fields {
			if f.Expr != nil && extendsTemps(f.Expr) {
				return true
			}
		}
	case *ast.CallExpr:
		for _, a := range x.Args {
			if extendsTemps(a) {
				return true
			}
		}
	case *ast.BlockExpr:
		if x.Block != nil {
			if t := blockTail(x.Block); t != nil {
				return extendsTemps(t)
			}
		}
	case *ast.IfExpr:
		if x.Then != nil {
			if t := blockTail(x.Then); t != nil && extendsTemps(t) {
				return true
			}
		}
		return x.Else != nil && extendsTemps(x.Else)
	case *ast.MatchExpr:
		for _, a := range x.Arms {
			if extendsTemps(a.Body) {
				return true
			}
		}
	case *ast.FormatArgsExpr:
		return true
	}
	return false
}

func (b *builder) ifExpr(x *ast.IfExpr, dest tplace) {
	b.pushScope()
	then, els := b.newBlock(), b.newBlock()
	b.cond(x.Cond, then, els)
	end := b.newBlock()
	b.cur = then
	b.block(x.Then, dest)
	b.popScope(x.Span)
	b.gotoBlock(end)
	// the else branch: the condition's scope was exited on the failure edges
	b.cur = els
	if x.Else != nil {
		b.exprInto(x.Else, dest)
	} else {
		b.assign(dest.p, Rvalue{Kind: RUse, Ops: []Operand{constOp()}}, x.Span)
	}
	b.gotoBlock(end)
	b.cur = end
}

// cond lowers a condition (with let-chains), branching to t or f. Pattern
// bindings are declared in the current scope; the failure edges exit it.
func (b *builder) cond(c ast.Expr, t, f int) {
	switch x := c.(type) {
	case *ast.ParenExpr:
		if containsLet(x.X) {
			b.cond(x.X, t, f)
			return
		}
	case *ast.BinaryExpr:
		if x.Op == ast.BinAnd && containsLet(c) {
			mid := b.newBlock()
			b.cond(x.L, mid, f)
			b.cur = mid
			b.cond(x.R, t, f)
			return
		}
	case *ast.LetExpr:
		var src tplace
		if b.isPlaceExpr(x.Expr) || len(b.r.Adjust[x.Expr.GetID()]) > 0 {
			src = b.placeOf(x.Expr)
		} else {
			src = b.valueTempUnadj(x.Expr)
		}
		b.emit(Stmt{Kind: SFakeRead, Place: src.p, Span: x.Span})
		bind := b.newBlock()
		fail := b.newBlock()
		b.switchTo(bind, fail)
		b.cur = fail
		b.gotoBlock(f)
		b.cur = bind
		b.declarePat(x.Pat)
		b.bindPat(x.Pat, src, false)
		b.gotoBlock(t)
		return
	}
	op := b.operand(c)
	_ = op
	b.switchTo(t, f)
}

func containsLet(e ast.Expr) bool {
	switch x := e.(type) {
	case *ast.LetExpr:
		return true
	case *ast.ParenExpr:
		return containsLet(x.X)
	case *ast.BinaryExpr:
		return x.Op == ast.BinAnd && (containsLet(x.L) || containsLet(x.R))
	}
	return false
}

func (b *builder) matchExpr(x *ast.MatchExpr, dest tplace) {
	sp := x.Span
	var src tplace
	if b.isPlaceExpr(x.Scrutinee) || len(b.r.Adjust[x.Scrutinee.GetID()]) > 0 {
		src = b.placeOf(x.Scrutinee)
	} else {
		src = b.valueTempUnadj(x.Scrutinee)
	}
	b.emit(Stmt{Kind: SFakeRead, Place: src.p, Span: sp})
	end := b.newBlock()
	for _, arm := range x.Arms {
		armB, next := b.newBlock(), b.newBlock()
		b.switchTo(armB, next)
		b.cur = armB
		b.pushScope()
		b.declarePat(arm.Pat)
		if arm.Guard != nil {
			// bind by copy for the guard; the moves happen once it holds
			b.bindPat(arm.Pat, src, true)
			body := b.newBlock()
			fail := b.newBlock()
			b.pushTemps()
			op := b.operand(arm.Guard)
			_ = op
			b.popTemps(sp)
			b.switchTo(body, fail)
			b.cur = fail
			b.exitScopeNoPop()
			b.gotoBlock(next)
			b.cur = body
			b.bindPatInit(arm.Pat, src)
		} else {
			b.bindPat(arm.Pat, src, false)
		}
		b.exprInto(arm.Body, dest)
		b.popScope(sp)
		b.gotoBlock(end)
		b.cur = next
	}
	b.terminate(Term{Kind: TUnreachable})
	b.cur = end
}

func (b *builder) exitScopeNoPop() {
	if len(b.scopes) > 0 {
		b.exitScope(b.scopes[len(b.scopes)-1], lexer.Span{})
	}
}

func (b *builder) whileExpr(x *ast.WhileExpr) {
	head := b.newBlock()
	end := b.newBlock()
	b.gotoBlock(head)
	b.cur = head
	lc := &loopCtx{label: x.Label, brk: end, cont: head, depth: len(b.scopes)}
	b.loops = append(b.loops, lc)
	b.pushScope()
	body, exit := b.newBlock(), b.newBlock()
	b.cond(x.Cond, body, exit)
	b.cur = body
	t := b.temp(TyUnit, x.Span)
	b.block(x.Body, t)
	b.popScope(x.Span)
	b.gotoBlock(head)
	b.cur = exit
	b.gotoBlock(end)
	b.loops = b.loops[:len(b.loops)-1]
	b.cur = end
}

func (b *builder) loopExpr(x *ast.LoopExpr, dest tplace) {
	head := b.newBlock()
	end := b.newBlock()
	b.gotoBlock(head)
	b.cur = head
	lc := &loopCtx{label: x.Label, brk: end, cont: head, depth: len(b.scopes), dest: &dest}
	b.loops = append(b.loops, lc)
	t := b.temp(TyUnit, x.Span)
	b.block(x.Body, t)
	b.gotoBlock(head)
	b.loops = b.loops[:len(b.loops)-1]
	b.cur = end
}

func (b *builder) forExpr(x *ast.ForExpr) {
	sp := x.Span
	info := b.r.Fors[x.ID]
	if info == nil {
		b.operand(x.Iter)
		return
	}
	b.pushScope()
	src := b.operand(x.Iter)
	iter := b.temp(info.IterTy, sp)
	b.scopes[len(b.scopes)-1].locals = append(b.scopes[len(b.scopes)-1].locals, iter.p.Local)
	b.removeTemp(iter.p.Local)
	b.call(iter.p, []Operand{src}, b.flowFor(info.IntoIter, 1, info.IterTy), "into_iter", sp, false)
	head := b.newBlock()
	end := b.newBlock()
	b.gotoBlock(head)
	b.cur = head
	lc := &loopCtx{label: x.Label, brk: end, cont: head, depth: len(b.scopes)}
	b.loops = append(b.loops, lc)
	b.pushScope()
	ref := b.temp(Ref(info.IterTy, true), sp)
	b.assign(ref.p, Rvalue{Kind: RRef, Place: iter.p, Mut: true}, sp)
	var optTy Ty
	if opt := b.tcx.Lang("Option"); opt != nil {
		optTy = Adt(opt, []Ty{info.ItemTy})
	}
	next := b.temp(optTy, sp)
	b.removeTemp(next.p.Local)
	b.scopes[len(b.scopes)-1].locals = append(b.scopes[len(b.scopes)-1].locals, next.p.Local)
	b.call(next.p, []Operand{{Kind: OMove, Place: ref.p}}, b.flowFor(info.Next, 1, optTy), "next", sp, false)
	body, exit := b.newBlock(), b.newBlock()
	b.switchTo(body, exit)
	b.cur = exit
	b.exitScopeNoPop()
	b.gotoBlock(end)
	b.cur = body
	b.declarePat(x.Pat)
	item := tplace{p: next.p.project(Proj{Kind: PField, Field: 0, Variant: 2}), ty: info.ItemTy}
	b.bindPat(x.Pat, item, false)
	t := b.temp(TyUnit, sp)
	b.block(x.Body, t)
	b.popScope(sp)
	b.gotoBlock(head)
	b.loops = b.loops[:len(b.loops)-1]
	b.cur = end
	b.popScope(sp)
}

func (b *builder) removeTemp(l LocalID) {
	for i := len(b.temps) - 1; i >= 0; i-- {
		if b.temps[i] == l {
			b.temps = append(b.temps[:i], b.temps[i+1:]...)
			return
		}
	}
}

func (b *builder) findLoop(label string, isBreak bool) *loopCtx {
	for i := len(b.loops) - 1; i >= 0; i-- {
		lc := b.loops[i]
		if label == "" {
			if lc.isBlock {
				continue
			}
			return lc
		}
		if lc.label == label {
			return lc
		}
	}
	return nil
}

func (b *builder) breakExpr(x *ast.BreakExpr) {
	lc := b.findLoop(x.Label, true)
	if x.Value != nil {
		if lc != nil && lc.dest != nil {
			b.exprInto(x.Value, *lc.dest)
		} else {
			b.operand(x.Value)
		}
	}
	if lc == nil {
		b.terminate(Term{Kind: TUnreachable})
		return
	}
	b.exitTo(lc.depth, x.Span)
	b.gotoBlock(lc.brk)
}

func (b *builder) continueExpr(x *ast.ContinueExpr) {
	lc := b.findLoop(x.Label, false)
	if lc == nil || lc.cont < 0 {
		b.terminate(Term{Kind: TUnreachable})
		return
	}
	b.exitTo(lc.depth, x.Span)
	b.gotoBlock(lc.cont)
}

func (b *builder) returnExpr(v ast.Expr, sp lexer.Span) {
	ret := tplace{p: Place{Local: 0}, ty: b.body.Locals[0].Ty}
	if v != nil {
		b.exprInto(v, ret)
	} else {
		b.assign(ret.p, Rvalue{Kind: RUse, Ops: []Operand{constOp()}}, sp)
	}
	b.exitTo(0, sp)
	b.gotoBlock(b.retBlock)
}

func (b *builder) tryExpr(x *ast.TryExpr, dest tplace) {
	sp := x.Span
	info := b.r.Tries[x.ID]
	src := b.valueTemp(x.X)
	ok, fail := b.newBlock(), b.newBlock()
	b.switchTo(ok, fail)
	// error path: convert and return
	b.cur = fail
	if info != nil && !info.IsOption {
		e := b.operandOf(tplace{p: src.p.project(Proj{Kind: PField, Field: 0, Variant: 2}), ty: info.ErrTy})
		ret := Place{Local: 0}
		b.assign(ret, Rvalue{Kind: RAggregate, Ops: []Operand{e}}, sp)
	} else {
		b.assign(Place{Local: 0}, Rvalue{Kind: RUse, Ops: []Operand{constOp()}}, sp)
	}
	b.exitTo(0, sp)
	b.gotoBlock(b.retBlock)
	b.cur = ok
	vt := dest.ty
	if info != nil {
		vt = info.OkTy
	}
	b.assign(dest.p, Rvalue{Kind: RUse, Ops: []Operand{b.operandOf(tplace{p: src.p.project(Proj{Kind: PField, Field: 0, Variant: 1}), ty: vt})}}, sp)
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
