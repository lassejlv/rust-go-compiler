package typeck

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
)

// FnCtxt checks one body instance.
type FnCtxt struct {
	t    *Tcx
	ic   *InferCtxt
	r    *Results
	inst Instance
	def  *Def
	ac   *astConv

	scopes   []map[string]*Local
	modStack []*Module

	retTy    Ty
	loops    []*loopCtx
	closures []*closureCtx
	// closureParent maps closure node -> enclosing closure node (0 = fn).
	closureParent map[ast.NodeID]ast.NodeID
	localCount    int

	// deferred trait-method callees to resolve after inference.
	deferred []*Callee
	// deferred cast checks
	casts []*ast.CastExpr
	// closure exprs in order of appearance
	closureExprs []*ast.ClosureExpr
}

type loopCtx struct {
	label   string
	breakTy Ty // nil for while/for (break value must be ())
	isLoop  bool
	broke   bool
	block   bool // labeled block
}

type closureCtx struct {
	expr  *ast.ClosureExpr
	retTy Ty
	loops []*loopCtx
}

// CheckInstance type checks a fn/const/static body with concrete args.
func (t *Tcx) CheckInstance(def *Def, args []Ty) *Results {
	key := InstKey(def, args)
	if r := t.results[key]; r != nil {
		return r
	}
	if t.inProgress[key] {
		t.Errorf(def.Span, "cycle detected when type checking `%s` (recursive `impl Trait`?)", def.Path())
		return nil
	}
	t.inProgress[key] = true
	defer delete(t.inProgress, key)
	t.BuildImpls()
	fcx := t.newFnCtxt(def, args)
	switch def.Kind {
	case DefFn:
		fcx.checkFn()
	case DefConst, DefStatic:
		fcx.checkConstBody()
	}
	fcx.writeback()
	t.results[key] = fcx.r
	return fcx.r
}

// ResultsFor returns cached results for an instance (nil if not checked).
func (t *Tcx) ResultsFor(def *Def, args []Ty) *Results { return t.results[InstKey(def, args)] }

func (t *Tcx) newFnCtxt(def *Def, args []Ty) *FnCtxt {
	fcx := &FnCtxt{
		t:             t,
		ic:            newInferCtxt(t),
		inst:          Instance{Def: def, Args: args},
		def:           def,
		closureParent: map[ast.NodeID]ast.NodeID{},
	}
	fcx.r = newResults(fcx.inst)
	t.Generics(def)
	fcx.ac = t.itemConv(def)
	fcx.ac.subst = args
	fcx.ac.infer = func() Ty { return fcx.ic.newVar() }
	fcx.modStack = []*Module{def.Scope}
	fcx.scopes = []map[string]*Local{{}}
	return fcx
}

func (fcx *FnCtxt) errorf(sp lexer.Span, format string, args ...any) {
	fcx.t.Errorf(sp, format, args...)
}

// ---------------------------------------------------------------------------
// Scopes and locals

func (fcx *FnCtxt) pushScope() { fcx.scopes = append(fcx.scopes, map[string]*Local{}) }
func (fcx *FnCtxt) popScope()  { fcx.scopes = fcx.scopes[:len(fcx.scopes)-1] }

func (fcx *FnCtxt) curClosure() ast.NodeID {
	if len(fcx.closures) == 0 {
		return 0
	}
	return fcx.closures[len(fcx.closures)-1].expr.ID
}

func (fcx *FnCtxt) newLocal(name string, ty Ty, mut bool, sp lexer.Span, node ast.NodeID) *Local {
	fcx.localCount++
	l := &Local{ID: fcx.localCount, Name: name, Ty: ty, Mut: mut, Span: sp, Node: node, Owner: fcx.curClosure()}
	fcx.scopes[len(fcx.scopes)-1][name] = l
	return l
}

func (fcx *FnCtxt) lookupLocal(name string) *Local {
	for i := len(fcx.scopes) - 1; i >= 0; i-- {
		if l, ok := fcx.scopes[i][name]; ok {
			return l
		}
	}
	return nil
}

func (fcx *FnCtxt) curMod() *Module { return fcx.modStack[len(fcx.modStack)-1] }

// ---------------------------------------------------------------------------
// Type helpers

func (fcx *FnCtxt) record(id ast.NodeID, t Ty) Ty {
	fcx.r.NodeTypes[id] = t
	return t
}

func (fcx *FnCtxt) shallow(t Ty) Ty { return fcx.ic.shallow(t) }
func (fcx *FnCtxt) resolve(t Ty) Ty { return fcx.ic.resolve(t) }

// structurally resolves a type: follows vars, normalizes projections and
// processes obligations if the type is still unknown.
func (fcx *FnCtxt) structurally(t Ty) Ty {
	t = fcx.shallow(t)
	if t.Kind == TProjection {
		t = fcx.ic.tryNormalize(fcx.resolve(t))
	}
	if t.Kind == TInfer && t.Sub == 0 {
		fcx.selectWherePossible()
		t = fcx.shallow(t)
	}
	return t
}

func (fcx *FnCtxt) demandEq(sp lexer.Span, expected, actual Ty) {
	if !fcx.ic.unify(expected, actual) {
		fcx.errorf(sp, "mismatched types: expected `%s`, found `%s`", fcx.resolve(expected), fcx.resolve(actual))
	}
}

func (fcx *FnCtxt) lowerTy(t ast.Type) Ty {
	fcx.ac.scope = fcx.curMod()
	return fcx.normalizeInBody(fcx.ac.lowerTy(t), t.GetSpan())
}

// normalizeInBody replaces projections that cannot be normalized yet with
// inference variables (registering obligations) and reveals opaque types.
func (fcx *FnCtxt) normalizeInBody(t Ty, sp lexer.Span) Ty {
	if t == nil {
		return t
	}
	if t.HasOpaque() {
		t = Fold(t, func(x Ty) Ty {
			if !x.HasOpaque() {
				return x
			}
			if x.Kind == TOpaque {
				return fcx.revealOpaque(x, sp)
			}
			return nil
		})
	}
	if !t.HasProj() {
		return t
	}
	return Fold(t, func(x Ty) Ty {
		if !x.HasProj() {
			return x
		}
		if x.Kind == TProjection {
			args := make([]Ty, len(x.Args))
			for i, a := range x.Args {
				args[i] = fcx.normalizeInBody(a, sp)
			}
			if n := fcx.ic.normalizeProjection(x.Def, fcx.ic.resolveList(args), x.Name, 0); n != nil {
				return fcx.normalizeInBody(n, sp)
			}
			v := fcx.ic.newVar()
			fcx.ic.obligations = append(fcx.ic.obligations, &Obligation{Proj: Projection(x.Def, args, x.Name), Var: v, Span: sp})
			return v
		}
		return nil
	})
}

// revealOpaque returns the hidden type of an opaque type.
func (fcx *FnCtxt) revealOpaque(x Ty, sp lexer.Span) Ty {
	args := fcx.ic.resolveList(x.Args)
	hasInfer := false
	for _, a := range args {
		if a.HasInfer() {
			hasInfer = true
		}
	}
	if hasInfer {
		v := fcx.ic.newVar()
		fcx.ic.obligations = append(fcx.ic.obligations, &Obligation{Proj: Opaque(x.Def, args, x.Index), Var: v, Span: sp})
		return v
	}
	o := Opaque(x.Def, args, x.Index)
	if h, ok := fcx.t.OpaqueHidden[o]; ok {
		return h
	}
	if x.Def == fcx.def && sameArgs(args, fcx.inst.Args) {
		fcx.errorf(sp, "recursive use of opaque type")
		return TyErr
	}
	fcx.t.CheckInstance(x.Def, args)
	if h, ok := fcx.t.OpaqueHidden[o]; ok {
		return h
	}
	return TyErr
}

func sameArgs(a, b []Ty) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}

// RevealOpaque replaces opaque types in a concrete type with hidden types.
func (t *Tcx) RevealOpaque(ty Ty) Ty {
	if !ty.HasOpaque() {
		return ty
	}
	return Fold(ty, func(x Ty) Ty {
		if !x.HasOpaque() {
			return x
		}
		if x.Kind == TOpaque {
			if h, ok := t.OpaqueHidden[x]; ok {
				return t.RevealOpaque(h)
			}
			t.CheckInstance(x.Def, x.Args)
			if h, ok := t.OpaqueHidden[x]; ok {
				return t.RevealOpaque(h)
			}
			return TyErr
		}
		return nil
	})
}

// InstantiateFnSig returns the concrete signature of a fn def / ctor with
// args (no inference).
func (t *Tcx) InstantiateFnSig(d *Def, args []Ty) ([]Ty, Ty) {
	switch d.Kind {
	case DefStruct, DefVariant:
		t.AdtFields(d)
		v := variantOf(d)
		ins := make([]Ty, len(v.Fields))
		for i, f := range v.Fields {
			ins[i] = t.Normalize(Subst(f.Ty, args))
		}
		adt := d
		if d.Kind == DefVariant {
			adt = d.Parent
		}
		return ins, Adt(adt, args)
	}
	sig := t.FnSig(d)
	ins := make([]Ty, len(sig.Inputs))
	for i, in := range sig.Inputs {
		ins[i] = t.Normalize(Subst(in, args))
	}
	return ins, t.Normalize(Subst(sig.Output, args))
}

// variantOf returns the VariantDef for a struct or variant def.
func variantOf(d *Def) *VariantDef {
	if d.Kind == DefVariant {
		return d.Variant
	}
	return d.Adt.Variants[0]
}

// instantiateSig substitutes args into a callee signature, normalizing.
func (fcx *FnCtxt) instantiateSig(d *Def, args []Ty, sp lexer.Span) ([]Ty, Ty) {
	t := fcx.t
	switch d.Kind {
	case DefStruct, DefVariant:
		t.AdtFields(d)
		v := variantOf(d)
		ins := make([]Ty, len(v.Fields))
		for i, f := range v.Fields {
			ins[i] = fcx.normalizeInBody(Subst(f.Ty, args), sp)
		}
		adt := d
		if d.Kind == DefVariant {
			adt = d.Parent
		}
		return ins, Adt(adt, args)
	}
	sig := t.FnSig(d)
	ins := make([]Ty, len(sig.Inputs))
	for i, in := range sig.Inputs {
		ins[i] = fcx.normalizeInBody(Subst(in, args), sp)
	}
	return ins, fcx.normalizeInBody(Subst(sig.Output, args), sp)
}

// registerPredicates adds the callee's where-clauses as obligations.
func (fcx *FnCtxt) registerPredicates(d *Def, args []Ty, sp lexer.Span) {
	g := fcx.t.Generics(d)
	for _, p := range g.Predicates {
		if p.Maybe || p.Trait.Trait.LangItem == "sized" {
			continue
		}
		sp2 := p.Subst(args)
		for i, a := range sp2.Trait.Args {
			sp2.Trait.Args[i] = fcx.normalizeInBody(a, sp)
		}
		for i, b := range sp2.Bindings {
			sp2.Bindings[i].Ty = fcx.normalizeInBody(b.Ty, sp)
		}
		fcx.ic.obligations = append(fcx.ic.obligations, &Obligation{Pred: sp2, Span: sp})
	}
}

// selectWherePossible processes pending obligations until no progress.
func (fcx *FnCtxt) selectWherePossible() {
	ic := fcx.ic
	for iter := 0; iter < 50; iter++ {
		progress := false
		obls := ic.obligations
		for i := 0; i < len(obls); i++ {
			o := obls[i]
			if o.Done {
				continue
			}
			if o.Proj != nil {
				if o.Proj.Kind == TOpaque {
					args := ic.resolveList(o.Proj.Args)
					ok := true
					for _, a := range args {
						if a.HasInfer() {
							ok = false
						}
					}
					if ok {
						h := fcx.revealOpaque(Opaque(o.Proj.Def, args, o.Proj.Index), o.Span)
						o.Done = true
						progress = true
						fcx.demandEq(o.Span, o.Var, h)
					}
					continue
				}
				args := ic.resolveList(o.Proj.Args)
				if n := ic.normalizeProjection(o.Proj.Def, args, o.Proj.Name, 0); n != nil {
					o.Done = true
					progress = true
					n = fcx.normalizeInBody(n, o.Span)
					if !ic.unify(o.Var, n) {
						fcx.errorf(o.Span, "type mismatch resolving `%s == %s`", fcx.resolve(o.Proj), fcx.resolve(n))
					}
				}
				continue
			}
			tr := &TraitRef{Trait: o.Pred.Trait.Trait, Args: ic.resolveList(o.Pred.Trait.Args)}
			sel := ic.selectTrait(tr, o.Depth)
			switch sel.Kind {
			case SelAmbig:
				continue
			case SelNone:
				o.Done = true
				progress = true
				// Unsatisfied bounds are reported leniently (only when fully
				// resolved, to avoid cascades).
				rt := &TraitRef{Trait: tr.Trait, Args: ic.resolveList(tr.Args)}
				hasInfer := false
				for _, a := range rt.Args {
					if a.HasInfer() || a.HasError() {
						hasInfer = true
					}
				}
				if !hasInfer {
					fcx.errorf(o.Span, "the trait bound `%s: %s` is not satisfied", rt.Args[0], traitRefDisplay(rt))
				}
				continue
			}
			o.Done = true
			progress = true
			if sel.Kind == SelImpl {
				// nested obligations from the impl's where clauses
				g := fcx.t.Generics(sel.Impl)
				for _, p := range g.Predicates {
					if p.Maybe || p.Trait.Trait.LangItem == "sized" {
						continue
					}
					np := p.Subst(sel.Args)
					ic.obligations = append(ic.obligations, &Obligation{Pred: np, Span: o.Span, Depth: o.Depth + 1})
				}
			}
			for _, b := range o.Pred.Bindings {
				n := ic.normalizeBinding(tr, b.Name, 0)
				if n == nil {
					v := ic.newVar()
					for _, st := range fcx.t.AllSupertraits(tr) {
						if it, ok := st.Trait.Trait.Items[b.Name]; ok && it.Kind == DefAssocTy {
							ic.obligations = append(ic.obligations, &Obligation{Proj: Projection(st.Trait, st.Args, b.Name), Var: v, Span: o.Span})
						}
					}
					n = v
				}
				n = fcx.normalizeInBody(n, o.Span)
				if !ic.unify(n, b.Ty) {
					fcx.errorf(o.Span, "type mismatch: expected `%s` to be `%s`", fcx.resolve(b.Ty), fcx.resolve(n))
				}
			}
		}
		if len(ic.obligations) > 2000 {
			// compact
			var keep []*Obligation
			for _, o := range ic.obligations {
				if !o.Done {
					keep = append(keep, o)
				}
			}
			ic.obligations = keep
		}
		if !progress {
			return
		}
	}
}

func traitRefDisplay(tr *TraitRef) string {
	s := tr.Trait.Name
	if len(tr.Args) > 1 {
		s += "<"
		for i, a := range tr.Args[1:] {
			if i > 0 {
				s += ", "
			}
			s += a.String()
		}
		s += ">"
	}
	return s
}

// ---------------------------------------------------------------------------
// Bodies

func (fcx *FnCtxt) checkFn() {
	d := fcx.def
	fn := d.Fn.Ast
	if fn.Body == nil {
		return // intrinsic / extern / required trait method
	}
	// Declared opaque types of this very function become inference vars.
	sig := fcx.t.FnSig(d)
	ins := make([]Ty, len(sig.Inputs))
	for i, in := range sig.Inputs {
		ins[i] = fcx.normalizeInBody(Subst(in, fcx.inst.Args), d.Span)
	}
	declOut := Subst(sig.Output, fcx.inst.Args)
	var opaques []Ty
	if declOut.HasOpaque() {
		declOut = Fold(declOut, func(x Ty) Ty {
			if x.Kind == TOpaque && x.Def == d {
				v := fcx.ic.newVar()
				opaques = append(opaques, x, v)
				return v
			}
			return nil
		})
	}
	out := fcx.normalizeInBody(declOut, d.Span)
	fcx.retTy = out
	fcx.r.RetTy = out
	fcx.r.ParamTys = ins
	fcx.r.Body = fn.Body
	i := 0
	if sp := fn.Decl.SelfParam; sp != nil {
		l := fcx.newLocal("self", ins[0], sp.Mut, sp.Span, sp.ID)
		fcx.r.SelfLocal = l
		fcx.r.Params = append(fcx.r.Params, l)
		i = 1
	}
	for _, p := range fn.Decl.Params {
		pty := ins[i]
		i++
		if ip, ok := p.Pat.(*ast.IdentPat); ok && ip.Sub == nil && !ip.ByRef {
			l := fcx.newLocal(ip.Name.Name, pty, ip.Mut, ip.Name.Span, ip.ID)
			fcx.r.Locals[ip.ID] = l
			fcx.record(ip.ID, pty)
			fcx.r.Params = append(fcx.r.Params, l)
			continue
		}
		// pattern param: bind a synthetic local, then destructure
		l := &Local{ID: -1, Name: "__arg", Ty: pty, Span: p.Span}
		fcx.localCount++
		l.ID = fcx.localCount
		fcx.r.Params = append(fcx.r.Params, l)
		fcx.checkPat(p.Pat, pty, bmByValue, true)
	}
	bodyTy := fcx.checkBlock(fn.Body, out)
	fcx.coerceBlockResult(fn.Body, bodyTy, out)
	fcx.selectWherePossible()
	fcx.finishInference()
	for k := 0; k < len(opaques); k += 2 {
		hidden := fcx.resolve(opaques[k+1])
		fcx.t.OpaqueHidden[Opaque(d, fcx.inst.Args, opaques[k].Index)] = hidden
	}
}

func (fcx *FnCtxt) checkConstBody() {
	d := fcx.def
	ty := fcx.normalizeInBody(Subst(fcx.t.ConstTy(d), fcx.inst.Args), d.Span)
	fcx.retTy = ty
	fcx.r.RetTy = ty
	if d.Const.Expr == nil {
		return
	}
	fcx.r.ConstExpr = d.Const.Expr
	fcx.checkExprCoerce(d.Const.Expr, ty)
	fcx.selectWherePossible()
	fcx.finishInference()
}

// finishInference applies integer/float defaults and re-processes obligations.
func (fcx *FnCtxt) finishInference() {
	for round := 0; round < 4; round++ {
		fcx.selectWherePossible()
		changed := false
		for id, ty := range fcx.r.NodeTypes {
			_ = id
			changed = fcx.defaultVars(ty) || changed
		}
		for _, o := range fcx.ic.obligations {
			if !o.Done && o.Pred != nil {
				for _, a := range o.Pred.Trait.Args {
					changed = fcx.defaultVars(a) || changed
				}
			}
		}
		if !changed {
			break
		}
	}
	fcx.selectWherePossible()
}

func (fcx *FnCtxt) defaultVars(ty Ty) bool {
	ty = fcx.resolve(ty)
	if !ty.HasInfer() {
		return false
	}
	changed := false
	Walk(ty, func(x Ty) bool {
		if x.Kind == TInfer {
			switch x.Sub {
			case 1:
				fcx.ic.unify(x, TyI32)
				changed = true
			case 2:
				fcx.ic.unify(x, TyF64)
				changed = true
			}
		}
		return x.HasInfer()
	})
	return changed
}

// coerceBlockResult coerces the value of a fn body block to the return type.
func (fcx *FnCtxt) coerceBlockResult(b *ast.Block, bodyTy, out Ty) {
	if tail := blockTail(b); tail != nil {
		return // coerced in checkBlock via expectation
	}
	bt := fcx.shallow(bodyTy)
	if bt.Kind == TNever {
		return
	}
	if !fcx.ic.unify(out, TyUnit) {
		fcx.errorf(b.Span, "mismatched types: expected `%s`, found `()`", fcx.resolve(out))
	}
}

// blockTail returns the tail expression of a block (nil if none).
func blockTail(b *ast.Block) ast.Expr {
	if len(b.Stmts) == 0 {
		return nil
	}
	if es, ok := b.Stmts[len(b.Stmts)-1].(*ast.ExprStmt); ok && !es.Semi {
		return es.Expr
	}
	return nil
}

// checkBlock checks a block; expected (may be nil) is the coercion target
// for the tail expression.
func (fcx *FnCtxt) checkBlock(b *ast.Block, expected Ty) Ty {
	fcx.pushScope()
	defer fcx.popScope()
	if bm := fcx.t.R.BlockMods[b.ID]; bm != nil {
		fcx.modStack = append(fcx.modStack, bm)
		defer func() { fcx.modStack = fcx.modStack[:len(fcx.modStack)-1] }()
	}
	diverges := false
	tail := blockTail(b)
	n := len(b.Stmts)
	if tail != nil {
		n--
	}
	for _, s := range b.Stmts[:n] {
		if fcx.checkStmt(s) {
			diverges = true
		}
	}
	var ty Ty
	if tail != nil {
		if expected != nil {
			ty = fcx.checkExprCoerce(tail, expected)
		} else {
			ty = fcx.checkExpr(tail, nil)
		}
		if diverges && fcx.shallow(ty).Kind != TNever {
			// e.g. `{ return; 5 }` still has the tail's type
		}
	} else if diverges {
		ty = TyNever
	} else {
		ty = TyUnit
	}
	fcx.record(b.ID, ty)
	return ty
}

// checkStmt checks a statement; returns true if it diverges.
func (fcx *FnCtxt) checkStmt(s ast.Stmt) bool {
	switch x := s.(type) {
	case *ast.LetStmt:
		return fcx.checkLet(x)
	case *ast.ExprStmt:
		var ty Ty
		if isBlockLike(x.Expr) && !x.Semi {
			ty = fcx.checkExprCoerce(x.Expr, TyUnit)
		} else {
			ty = fcx.checkExpr(x.Expr, nil)
		}
		return fcx.shallow(ty).Kind == TNever
	case *ast.ItemStmt, *ast.EmptyStmt:
		return false
	case *ast.MacStmt:
		fcx.errorf(x.Mac.Span, "macro `%s!` was not expanded", ast.PathString(x.Mac.Path))
		return false
	}
	return false
}

func isBlockLike(e ast.Expr) bool {
	switch e.(type) {
	case *ast.BlockExpr, *ast.IfExpr, *ast.MatchExpr, *ast.LoopExpr, *ast.WhileExpr, *ast.ForExpr:
		return true
	}
	return false
}

func (fcx *FnCtxt) checkLet(x *ast.LetStmt) bool {
	var declTy Ty
	if x.Ty != nil {
		declTy = fcx.lowerTy(x.Ty)
	}
	var initTy Ty
	diverges := false
	if x.Init != nil {
		if declTy != nil {
			initTy = fcx.checkExprCoerce(x.Init, declTy)
		} else {
			initTy = fcx.checkExpr(x.Init, nil)
		}
		diverges = fcx.shallow(initTy).Kind == TNever
	}
	ty := declTy
	if ty == nil {
		if initTy != nil {
			ty = initTy
		} else {
			ty = fcx.ic.newVar()
		}
	}
	if x.Else != nil {
		elseTy := fcx.checkBlock(x.Else, nil)
		if fcx.shallow(elseTy).Kind != TNever {
			fcx.errorf(x.Else.Span, "`else` clause of `let...else` does not diverge")
		}
	}
	fcx.checkPat(x.Pat, ty, bmByValue, true)
	return diverges
}

// ---------------------------------------------------------------------------
// Loops / labels

func (fcx *FnCtxt) loopStack() *[]*loopCtx {
	if len(fcx.closures) > 0 {
		return &fcx.closures[len(fcx.closures)-1].loops
	}
	return &fcx.loops
}

func (fcx *FnCtxt) findLoop(label string, sp lexer.Span, isContinue bool) *loopCtx {
	ls := *fcx.loopStack()
	for i := len(ls) - 1; i >= 0; i-- {
		l := ls[i]
		if label == "" {
			if l.block {
				continue
			}
			return l
		}
		if l.label == label {
			return l
		}
	}
	if label != "" {
		fcx.errorf(sp, "use of undeclared label `%s`", label)
	} else if isContinue {
		fcx.errorf(sp, "`continue` outside of a loop")
	} else {
		fcx.errorf(sp, "`break` outside of a loop")
	}
	return nil
}
