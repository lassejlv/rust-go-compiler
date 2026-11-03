package typeck

import (
	"os"
	"sort"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

// fin resolves a type completely for codegen.
func (fcx *FnCtxt) fin(t Ty) Ty {
	if t == nil {
		return nil
	}
	t = fcx.ic.resolve(t)
	if t.HasProj() {
		t = fcx.ic.tryNormalize(t)
		t = fcx.ic.resolve(t)
	}
	if t.HasInfer() {
		t = Fold(t, func(x Ty) Ty {
			if !x.HasInfer() {
				return x
			}
			if x.Kind == TInfer {
				switch x.Sub {
				case 1:
					return TyI32
				case 2:
					return TyF64
				}
				if os.Getenv("RUSTGO_DEBUG_INFER") != "" {
					println("warning: unresolved inference variable defaulted to ()")
				}
				return TyUnit
			}
			return nil
		})
	}
	if t.HasProj() || t.HasOpaque() {
		t = fcx.t.Normalize(t)
	}
	return t
}

func (fcx *FnCtxt) finList(ts []Ty) []Ty {
	out := make([]Ty, len(ts))
	for i, t := range ts {
		out[i] = fcx.fin(t)
	}
	return out
}

func (fcx *FnCtxt) finCallee(c *Callee) {
	if c == nil {
		return
	}
	c.Args = fcx.finList(c.Args)
	c.Ty = fcx.fin(c.Ty)
	if c.Kind == CallClosure && c.Ty != nil {
		// a call through a then-unrevealed opaque: use the hidden type
		for i := 0; i < 8 && c.Ty.Kind == TOpaque; i++ {
			c.Ty = fcx.t.RevealOpaque(c.Ty)
		}
		switch c.Ty.Kind {
		case TFnPtr:
			c.Kind = CallFnPtr
		case TFnDef:
			c.Kind, c.Def, c.Args = CallFn, c.Ty.Def, c.Ty.Args
		}
	}
	if c.Kind == CallFn && c.Def != nil && c.Def.InTrait() && len(c.Args) > 0 && c.Args[0].Kind == TDyn && fcx.t.IsObjectMethod(c.Def, c.Args[0]) {
		c.Kind = CallVirtual
	}
	if c.Kind == CallVirtual && c.Def != nil && len(c.Args) > 0 && !fcx.t.IsObjectMethod(c.Def, c.Args[0]) {
		c.Kind = CallFn
	}
}

func (fcx *FnCtxt) writeback() {
	r := fcx.r
	if fcx.t.quiet == 0 && !fcx.t.isStd(fcx.def) {
		fcx.checkUninferredLocals()
	}
	for id, t := range r.NodeTypes {
		r.NodeTypes[id] = fcx.fin(t)
	}
	for _, adj := range r.Adjust {
		for i := range adj {
			adj[i].Target = fcx.fin(adj[i].Target)
			if adj[i].Callee != nil {
				fcx.finCallee(adj[i].Callee)
				// overloaded deref callee: switch to deref_mut when Mut
				if adj[i].Kind == AdjDerefOverloaded && adj[i].Mut {
					if dm := fcx.t.LangOpt("deref_mut"); dm != nil {
						adj[i].Callee.Def = dm.Trait.Items["deref_mut"]
					}
				}
			}
		}
	}
	for id, c := range r.Callees {
		fcx.finCallee(c)
		if r.MutPlace[id] {
			switch c.Def.Name {
			case "deref":
				if dm := fcx.t.LangOpt("deref_mut"); dm != nil {
					c.Def = dm.Trait.Items["deref_mut"]
				}
			case "index":
				if im := fcx.t.LangOpt("index_mut"); im != nil {
					c.Def = im.Trait.Items["index_mut"]
				}
			}
		}
	}
	for _, vr := range r.Paths {
		vr.Args = fcx.finList(vr.Args)
	}
	seenLocals := map[*Local]bool{}
	finLocal := func(l *Local) {
		if l != nil && !seenLocals[l] {
			seenLocals[l] = true
			l.Ty = fcx.fin(l.Ty)
		}
	}
	for _, l := range r.Locals {
		finLocal(l)
	}
	for _, l := range r.Params {
		finLocal(l)
	}
	for _, vr := range r.Paths {
		if vr.Local != nil {
			finLocal(vr.Local)
		}
	}
	for id, ds := range r.PatDerefs {
		r.PatDerefs[id] = fcx.finList(ds)
	}
	for _, f := range r.Fors {
		fcx.finCallee(f.IntoIter)
		fcx.finCallee(f.Next)
		f.IterTy = fcx.fin(f.IterTy)
		f.ItemTy = fcx.fin(f.ItemTy)
	}
	for _, ti := range r.Tries {
		fcx.finCallee(ti.From)
		ti.ErrTy = fcx.fin(ti.ErrTy)
		ti.OutErrTy = fcx.fin(ti.OutErrTy)
		ti.OkTy = fcx.fin(ti.OkTy)
		ti.OutTy = fcx.fin(ti.OutTy)
	}
	r.RetTy = fcx.fin(r.RetTy)
	r.ParamTys = fcx.finList(r.ParamTys)
	for _, ci := range r.Closures {
		ci.Inputs = fcx.finList(ci.Inputs)
		ci.Output = fcx.fin(ci.Output)
		for _, p := range ci.Params {
			finLocal(p)
		}
	}
	r.LocalCount = fcx.localCount
	fcx.analyzeCaptures()
}

// ---------------------------------------------------------------------------
// Capture analysis

type useCtx int

const (
	useValue useCtx = iota
	usePlace
	usePlaceMut
)

type capUse struct {
	value bool // moved (by-value use of a non-Copy local)
	mut   bool
	order int
}

type capWalker struct {
	fcx   *FnCtxt
	stack []*ast.ClosureExpr
	uses  map[ast.NodeID]map[*Local]*capUse
	n     int
}

func (fcx *FnCtxt) insideClosure(l *Local, c ast.NodeID) bool {
	o := l.Owner
	for o != 0 {
		if o == c {
			return true
		}
		o = fcx.closureParent[o]
	}
	return false
}

func (fcx *FnCtxt) analyzeCaptures() {
	if len(fcx.r.Closures) == 0 {
		return
	}
	w := &capWalker{fcx: fcx, uses: map[ast.NodeID]map[*Local]*capUse{}}
	if fcx.r.Body != nil {
		w.block(fcx.r.Body, useValue)
	}
	if fcx.r.ConstExpr != nil {
		w.expr(fcx.r.ConstExpr, useValue)
	}
	for _, ce := range fcx.closureExprs {
		ci := fcx.r.Closures[ce.ID]
		us := w.uses[ce.ID]
		var locals []*Local
		for l := range us {
			locals = append(locals, l)
		}
		sort.Slice(locals, func(i, j int) bool { return us[locals[i]].order < us[locals[j]].order })
		kind := 0
		ci.Captures = nil
		// a non-move async closure's future moves only the closure's params
		var ownParams map[*Local]bool
		if ce.MoveParamsOf != nil {
			ownParams = map[*Local]bool{}
			for _, p := range ce.MoveParamsOf.Params {
				ast.Walk(p.Pat, func(n any) bool {
					if ip, ok := n.(*ast.IdentPat); ok {
						if pl := fcx.r.Locals[ip.ID]; pl != nil {
							ownParams[pl] = true
						}
					}
					return true
				})
			}
		}
		for _, l := range locals {
			u := us[l]
			copyTy := fcx.t.IsCopy(l.Ty)
			c := &Capture{Local: l, Ty: l.Ty}
			moved := u.value && !copyTy
			isMove := ce.Move && (ownParams == nil || ownParams[l])
			switch {
			case isMove:
				c.ByRef = false
			case moved:
				c.ByRef = false
			case u.mut:
				c.ByRef = true
				c.Mut = true
			case l.Ty != nil && l.Ty.Kind == TRef && !l.Ty.Mut:
				// A shared reference that is only read is captured by
				// copy (rustc captures the reborrowed `*r` and truncates
				// it to `r` by value), so the closure does not borrow the
				// enclosing frame's local and may outlive it.
				c.ByRef = false
			default:
				c.ByRef = true
			}
			if moved {
				kind = 2
			} else if u.mut && kind < 1 {
				kind = 1
			}
			ci.Captures = append(ci.Captures, c)
		}
		ci.Kind = kind
	}
}

func (w *capWalker) record(l *Local, ctx useCtx) {
	moveSeen := false
	for i := len(w.stack) - 1; i >= 0; i-- {
		c := w.stack[i]
		if w.fcx.insideClosure(l, c.ID) {
			break
		}
		eff := ctx
		if moveSeen {
			eff = useValue
		}
		m := w.uses[c.ID]
		if m == nil {
			m = map[*Local]*capUse{}
			w.uses[c.ID] = m
		}
		u := m[l]
		if u == nil {
			w.n++
			u = &capUse{order: w.n}
			m[l] = u
		}
		switch eff {
		case useValue:
			u.value = true
		case usePlaceMut:
			u.mut = true
		}
		if c.Move && c.MoveParamsOf == nil {
			moveSeen = true
		}
	}
}

func (w *capWalker) isCopy(t Ty) bool {
	if t == nil {
		return true
	}
	return w.fcx.t.IsCopy(t)
}

// adjustCtx computes the context in which an expression itself is used,
// given its adjustments.
// declInputs returns the declared (generic) input types of a called fn.
func (w *capWalker) declInputs(c *Callee) []Ty {
	if c == nil || c.Def == nil || c.Def.Kind != DefFn {
		return nil
	}
	return w.fcx.t.FnSig(c.Def).Inputs
}

// argCtx is the use context of a call argument: a `&mut` value passed
// where the callee declares a reference parameter is implicitly
// reborrowed (a mutable use of the place) rather than moved.
func (w *capWalker) argCtx(a ast.Expr, decl []Ty, i int) useCtx {
	if i >= len(decl) || decl[i] == nil || decl[i].Kind != TRef {
		return useValue
	}
	if t := w.fcx.r.NodeTypes[a.GetID()]; t != nil && t.Kind == TRef && t.Mut {
		return usePlaceMut
	}
	return useValue
}

func (w *capWalker) adjustCtx(e ast.Expr, ctx useCtx) useCtx {
	adj := w.fcx.r.Adjust[e.GetID()]
	for _, a := range adj {
		switch a.Kind {
		case AdjBorrow:
			if a.Mut {
				return usePlaceMut
			}
			return usePlace
		case AdjDerefOverloaded:
			if a.Mut {
				return usePlaceMut
			}
			return usePlace
		case AdjDeref:
			if ctx == useValue {
				t := w.fcx.r.NodeTypes[e.GetID()]
				if t != nil && t.Kind == TRef {
					return usePlace
				}
			}
			return ctx
		}
	}
	return ctx
}

func (w *capWalker) block(b *ast.Block, ctx useCtx) {
	for i, s := range b.Stmts {
		last := i == len(b.Stmts)-1
		switch x := s.(type) {
		case *ast.LetStmt:
			if x.Init != nil {
				w.expr(x.Init, w.patCtx(x.Pat))
			}
			if x.Else != nil {
				w.block(x.Else, useValue)
			}
		case *ast.ExprStmt:
			if last && !x.Semi {
				w.expr(x.Expr, ctx)
			} else {
				w.expr(x.Expr, useValue)
			}
		}
	}
}

// patCtx: does matching pat move out of the scrutinee?
func (w *capWalker) patCtx(p ast.Pat) useCtx {
	ctx := usePlace
	ast.Walk(p, func(n any) bool {
		if ip, ok := n.(*ast.IdentPat); ok {
			if l := w.fcx.r.Locals[ip.ID]; l != nil && !l.ByRef && !w.isCopy(l.Ty) {
				ctx = useValue
			}
		}
		return true
	})
	return ctx
}

func (w *capWalker) expr(e ast.Expr, ctx useCtx) {
	r := w.fcx.r
	ctx = w.adjustCtx(e, ctx)
	switch x := e.(type) {
	case *ast.PathExpr:
		if vr := r.Paths[x.ID]; vr != nil && vr.Kind == VLocal {
			c := ctx
			if c == useValue && w.isCopy(vr.Local.Ty) {
				c = usePlace
			}
			w.record(vr.Local, c)
		}
	case *ast.ParenExpr:
		w.expr(x.X, ctx)
	case *ast.FieldExpr:
		bc := ctx
		if ctx == useValue && w.isCopy(r.NodeTypes[x.ID]) {
			bc = usePlace
		}
		w.expr(x.X, bc)
	case *ast.IndexExpr:
		bc := usePlace
		if ctx == usePlaceMut {
			bc = usePlaceMut
		}
		w.expr(x.X, bc)
		w.expr(x.Index, useValue)
	case *ast.UnaryExpr:
		if x.Op == ast.UnDeref {
			bc := usePlace
			if ctx == usePlaceMut {
				bc = usePlaceMut
			}
			if t := r.NodeTypes[x.X.GetID()]; t != nil && t.Kind == TAdt && t.Def.LangItem == "owned_box" && ctx == useValue {
				bc = useValue
			}
			w.expr(x.X, bc)
		} else {
			w.expr(x.X, useValue)
		}
	case *ast.BinaryExpr:
		c := useValue
		if _, ok := r.Callees[x.ID]; ok && isCompare(x.Op) {
			c = usePlace
		}
		w.expr(x.L, c)
		w.expr(x.R, c)
	case *ast.AssignExpr:
		w.assignLhs(x.L)
		w.expr(x.R, useValue)
	case *ast.AssignOpExpr:
		w.expr(x.L, usePlaceMut)
		w.expr(x.R, useValue)
	case *ast.CallExpr:
		fc := useValue
		if c := r.Callees[x.ID]; c != nil {
			switch c.Kind {
			case CallClosure:
				if ci := w.fcx.t.Closures[c.Ty]; ci != nil {
					fc = []useCtx{usePlace, usePlaceMut, useValue}[ci.Kind]
				}
			case CallDynFn:
				switch c.Ty.Def.LangItem {
				case "fn":
					fc = usePlace
				case "fn_mut":
					fc = usePlaceMut
				}
			}
		}
		w.expr(x.Fn, fc)
		decl := w.declInputs(r.Callees[x.ID])
		for i, a := range x.Args {
			w.expr(a, w.argCtx(a, decl, i))
		}
	case *ast.MethodCallExpr:
		decl := w.declInputs(r.Callees[x.ID])
		w.expr(x.Receiver, w.argCtx(x.Receiver, decl, 0))
		for i, a := range x.Args {
			w.expr(a, w.argCtx(a, decl, i+1))
		}
	case *ast.RefExpr:
		if x.Mut {
			w.expr(x.X, usePlaceMut)
		} else {
			w.expr(x.X, usePlace)
		}
	case *ast.AsmExpr:
		for _, op := range x.Operands {
			if op.Out != nil {
				w.expr(op.Out, usePlaceMut)
			}
			if op.In != nil && op.In != op.Out {
				w.expr(op.In, useValue)
			}
		}
	case *ast.CastExpr:
		w.expr(x.X, useValue)
	case *ast.BlockExpr:
		w.block(x.Block, ctx)
	case *ast.IfExpr:
		w.expr(x.Cond, useValue)
		w.block(x.Then, ctx)
		if x.Else != nil {
			w.expr(x.Else, ctx)
		}
	case *ast.LetExpr:
		w.expr(x.Expr, w.patCtx(x.Pat))
	case *ast.MatchExpr:
		sc := usePlace
		for _, a := range x.Arms {
			if w.patCtx(a.Pat) == useValue {
				sc = useValue
			}
		}
		w.expr(x.Scrutinee, sc)
		for _, a := range x.Arms {
			if a.Guard != nil {
				w.expr(a.Guard, useValue)
			}
			w.expr(a.Body, ctx)
		}
	case *ast.WhileExpr:
		w.expr(x.Cond, useValue)
		w.block(x.Body, useValue)
	case *ast.LoopExpr:
		w.block(x.Body, useValue)
	case *ast.ForExpr:
		w.expr(x.Iter, useValue)
		w.block(x.Body, useValue)
	case *ast.BreakExpr:
		if x.Value != nil {
			w.expr(x.Value, useValue)
		}
	case *ast.ReturnExpr:
		if x.Value != nil {
			w.expr(x.Value, useValue)
		}
	case *ast.TupleExpr:
		for _, el := range x.Elems {
			w.expr(el, useValue)
		}
	case *ast.ArrayExpr:
		for _, el := range x.Elems {
			w.expr(el, useValue)
		}
	case *ast.RepeatExpr:
		w.expr(x.Elem, useValue)
	case *ast.StructExpr:
		for _, f := range x.Fields {
			w.expr(f.Expr, useValue)
		}
		if x.Base != nil {
			w.expr(x.Base, useValue)
		}
	case *ast.RangeExpr:
		if x.Start != nil {
			w.expr(x.Start, useValue)
		}
		if x.End != nil {
			w.expr(x.End, useValue)
		}
	case *ast.TryExpr:
		w.expr(x.X, useValue)
	case *ast.ClosureExpr:
		w.stack = append(w.stack, x)
		w.expr(x.Body, useValue)
		w.stack = w.stack[:len(w.stack)-1]
		// the closure value itself captures: uses were recorded for outer
		// closures during the walk above.
	}
}

func (w *capWalker) assignLhs(l ast.Expr) {
	switch x := l.(type) {
	case *ast.TupleExpr:
		for _, el := range x.Elems {
			w.assignLhs(el)
		}
	case *ast.ParenExpr:
		w.assignLhs(x.X)
	case *ast.UnderscoreExpr:
	default:
		w.expr(l, usePlaceMut)
	}
}

// checkUninferredLocals reports locals whose type inference left
// unconstrained (rustc E0282).
func (fcx *FnCtxt) checkUninferredLocals() {
	var ls []*Local
	for id, l := range fcx.r.Locals {
		if l != nil && l.Node == id && l.Ty != nil {
			ls = append(ls, l)
		}
	}
	sort.Slice(ls, func(i, j int) bool { return ls[i].ID < ls[j].ID })
	for _, l := range ls {
		t := fcx.ic.resolve(l.Ty)
		if t.HasProj() {
			t = fcx.ic.resolve(fcx.ic.tryNormalize(t))
		}
		unresolved := false
		Walk(t, func(x Ty) bool {
			if x.Kind == TInfer && x.Sub == 0 {
				unresolved = true
			}
			return !unresolved
		})
		if unresolved {
			fcx.t.ErrorCode(l.Span, "E0282", "type annotations needed")
			return
		}
	}
}
