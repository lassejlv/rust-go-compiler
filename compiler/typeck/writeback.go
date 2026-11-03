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

// capUse is a use of a place (a variable or a place inside it) by a
// closure body.
type capUse struct {
	path  []CapProj
	moved bool // used by value, not Copy
	value bool // used by value
	mut   bool
	order int
	// nodes lists, per occurrence, the expressions denoting the prefixes of
	// path: nodes[0] is the variable's PathExpr, nodes[i] the expression
	// whose place is path[:i] (0 when that is a dereference applied by an
	// adjustment)
	nodes [][]ast.NodeID
}

type capWalker struct {
	fcx   *FnCtxt
	stack []*ast.ClosureExpr
	uses  map[ast.NodeID]map[*Local][]*capUse
	first map[ast.NodeID]map[*Local]int // first use of each variable
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
	// a closure calling another closure uses it according to the callee's
	// kind, which is only known after the analysis: iterate until the kinds
	// are stable
	for i := 0; i < 3; i++ {
		before := map[*ClosureInfo]int{}
		for _, ci := range fcx.r.Closures {
			before[ci] = ci.Kind
		}
		fcx.analyzeCapturesOnce()
		changed := false
		for _, ci := range fcx.r.Closures {
			if ci.Kind != before[ci] {
				changed = true
			}
		}
		if !changed {
			return
		}
	}
}

// preciseCaptures reports whether closures of the body capture disjoint
// places (Rust 2021) rather than whole variables.
func (fcx *FnCtxt) preciseCaptures() bool {
	if os.Getenv("RUSTGO_WHOLE_CAPTURES") != "" {
		return false
	}
	d := fcx.r.Inst.Def
	if d == nil || d.Crate == nil {
		return true
	}
	if d.Crate.Name == "std" {
		return false
	}
	return d.Crate.Edition == "" || d.Crate.Edition >= "2021"
}

func (fcx *FnCtxt) analyzeCapturesOnce() {
	w := &capWalker{fcx: fcx, uses: map[ast.NodeID]map[*Local][]*capUse{}, first: map[ast.NodeID]map[*Local]int{}}
	if fcx.r.Body != nil {
		w.block(fcx.r.Body, useValue)
	}
	if fcx.r.ConstExpr != nil {
		w.expr(fcx.r.ConstExpr, useValue)
	}
	precise := fcx.preciseCaptures()
	for _, ce := range fcx.closureExprs {
		ci := fcx.r.Closures[ce.ID]
		us := w.uses[ce.ID]
		first := w.first[ce.ID]
		var locals []*Local
		for l := range us {
			locals = append(locals, l)
		}
		sort.Slice(locals, func(i, j int) bool { return first[locals[i]] < first[locals[j]] })
		kind := 0
		ci.Captures = nil
		ci.CapExprs = nil
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
			isMove := ce.Move && (ownParams == nil || ownParams[l])
			type capPlace struct {
				path []CapProj
				rank int // 0 read, 1 mut, 2 by value
			}
			var caps []*capPlace
			for _, u := range us[l] {
				if u.moved {
					kind = 2
				} else if u.mut && kind < 1 {
					kind = 1
				}
				c := &capPlace{}
				switch {
				case isMove || u.moved:
					c.rank = 2
				case u.mut:
					c.rank = 1
				}
				if precise {
					c.path = w.truncateCapture(l, u.path, c.rank == 2)
				}
				// minimal capture set: an ancestor place absorbs its
				// descendants (rustc's compute_min_captures)
				merged := false
				for _, o := range caps {
					if PathPrefix(o.path, c.path) {
						if c.rank > o.rank {
							o.rank = c.rank
						}
						merged = true
						break
					}
				}
				if merged {
					continue
				}
				keep := caps[:0]
				for _, o := range caps {
					if PathPrefix(c.path, o.path) {
						if o.rank > c.rank {
							c.rank = o.rank
						}
						continue
					}
					keep = append(keep, o)
				}
				caps = append(keep, c)
			}
			// captures of one variable are ordered by field (drop order)
			sort.SliceStable(caps, func(i, j int) bool { return capPathLess(caps[i].path, caps[j].path) })
			for _, cp := range caps {
				c := &Capture{Local: l, Path: cp.path, Ty: l.Ty}
				if len(cp.path) > 0 {
					c.Ty = cp.path[len(cp.path)-1].Ty
				}
				switch {
				case cp.rank == 2:
					c.ByRef = false
				case cp.rank == 1:
					c.ByRef = true
					c.Mut = true
				case c.Ty != nil && c.Ty.Kind == TRef && !c.Ty.Mut:
					// A shared reference that is only read is captured by
					// copy (rustc captures the reborrowed `*r` and truncates
					// it to `r` by value), so the closure does not borrow the
					// enclosing frame's local and may outlive it.
					c.ByRef = false
				default:
					c.ByRef = true
				}
				idx := len(ci.Captures)
				ci.Captures = append(ci.Captures, c)
				if len(cp.path) == 0 {
					continue
				}
				// the expressions denoting the captured place
				for _, u := range us[l] {
					if !PathPrefix(cp.path, u.path) {
						continue
					}
					for _, ns := range u.nodes {
						if n := ns[len(cp.path)]; n != 0 {
							if ci.CapExprs == nil {
								ci.CapExprs = map[ast.NodeID]int{}
							}
							ci.CapExprs[n] = idx
						}
					}
				}
			}
		}
		ci.Kind = kind
	}
}

func capPathLess(a, b []CapProj) bool {
	for i := 0; i < len(a) && i < len(b); i++ {
		if a[i].Deref != b[i].Deref {
			return a[i].Deref
		}
		if a[i].Field != b[i].Field {
			return a[i].Field < b[i].Field
		}
	}
	return len(a) < len(b)
}

// truncateCapture applies rustc's capture precision rules to a used place:
// by-value captures stop at the first dereference and do not move out of a
// type with a destructor; reads through a shared reference capture the
// reference; and a captured place never ends in a dereference (the
// reference or Box itself is captured instead).
func (w *capWalker) truncateCapture(l *Local, path []CapProj, byValue bool) []CapProj {
	tyAt := func(i int) Ty { // type of the place path[:i]
		if i == 0 {
			return l.Ty
		}
		return path[i-1].Ty
	}
	if byValue {
		for i, pr := range path {
			if pr.Deref {
				path = path[:i]
				break
			}
		}
		if len(path) > 0 && !w.isCopy(path[len(path)-1].Ty) {
			if dt := w.fcx.t.LangOpt("drop"); dt != nil {
				for i, pr := range path {
					if t := tyAt(i); !pr.Deref && t != nil && t.Kind == TAdt && w.fcx.t.Implements(t, dt) {
						path = path[:i]
						break
					}
				}
			}
		}
	} else {
		for i := len(path) - 1; i >= 0; i-- {
			if path[i].Deref {
				if t := tyAt(i); t != nil && t.Kind == TRef && !t.Mut {
					path = path[:i+1]
				}
				break
			}
		}
	}
	for len(path) > 0 && path[len(path)-1].Deref {
		path = path[:len(path)-1]
	}
	return path
}

func (w *capWalker) record(l *Local, path []CapProj, nodes []ast.NodeID, ctx useCtx) {
	pt := l.Ty
	if len(path) > 0 {
		pt = path[len(path)-1].Ty
	}
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
			m = map[*Local][]*capUse{}
			w.uses[c.ID] = m
			w.first[c.ID] = map[*Local]int{}
		}
		var u *capUse
		for _, o := range m[l] {
			if len(o.path) == len(path) && PathPrefix(o.path, path) {
				u = o
				break
			}
		}
		if u == nil {
			w.n++
			u = &capUse{path: path, order: w.n}
			m[l] = append(m[l], u)
			if _, ok := w.first[c.ID][l]; !ok {
				w.first[c.ID][l] = w.n
			}
		}
		u.nodes = append(u.nodes, nodes)
		switch eff {
		case useValue:
			u.value = true
			if !w.isCopy(pt) {
				u.moved = true
			}
		case usePlaceMut:
			u.mut = true
		}
		if c.Move && c.MoveParamsOf == nil {
			moveSeen = true
		}
	}
}

// capPlace returns the place an (unadjusted) expression denotes when it is
// a variable followed by field projections and dereferences of references
// and Boxes, with the expressions denoting its prefixes (see capUse.nodes).
func (w *capWalker) capPlace(e ast.Expr) (*Local, []CapProj, []ast.NodeID, bool) {
	r := w.fcx.r
	switch x := e.(type) {
	case *ast.ParenExpr:
		return w.capPlaceAdj(x.X)
	case *ast.PathExpr:
		if vr := r.Paths[x.ID]; vr != nil && vr.Kind == VLocal {
			return vr.Local, nil, []ast.NodeID{x.ID}, true
		}
	case *ast.FieldExpr:
		fr, ok := r.Fields[x.ID]
		t := r.NodeTypes[x.ID]
		if !ok || fr.Union || t == nil || t.IsUnsized() {
			break
		}
		l, path, nodes, ok := w.capPlaceAdj(x.X)
		if !ok {
			break
		}
		bt := l.Ty
		if len(path) > 0 {
			bt = path[len(path)-1].Ty
		}
		if bt == nil || (bt.Kind == TAdt && bt.Def.ReprPacked() > 0) {
			break
		}
		return l, append(path, CapProj{Field: fr.Index, Name: x.Name.Name, Ty: t}), append(nodes, x.ID), true
	case *ast.UnaryExpr:
		if x.Op != ast.UnDeref || r.Callees[x.ID] != nil {
			break
		}
		t := r.NodeTypes[x.ID]
		bt := r.AdjustedType(x.X)
		if t == nil || t.IsUnsized() || bt == nil || !(bt.Kind == TRef || (bt.Kind == TAdt && bt.Def.LangItem == "owned_box")) {
			break
		}
		l, path, nodes, ok := w.capPlaceAdj(x.X)
		if !ok {
			break
		}
		return l, append(path, CapProj{Deref: true, Ty: t}), append(nodes, x.ID), true
	}
	return nil, nil, nil, false
}

// capPlaceAdj is capPlace for an expression with its adjustments (only
// builtin dereferences continue a place).
func (w *capWalker) capPlaceAdj(e ast.Expr) (*Local, []CapProj, []ast.NodeID, bool) {
	l, path, nodes, ok := w.capPlace(e)
	if !ok {
		return nil, nil, nil, false
	}
	cur := w.fcx.r.NodeTypes[e.GetID()]
	for _, a := range w.fcx.r.Adjust[e.GetID()] {
		if a.Kind != AdjDeref || cur == nil || !(cur.Kind == TRef || (cur.Kind == TAdt && cur.Def.LangItem == "owned_box")) || a.Target == nil || a.Target.IsUnsized() {
			return nil, nil, nil, false
		}
		path = append(path, CapProj{Deref: true, Ty: a.Target})
		nodes = append(nodes, 0)
		cur = a.Target
	}
	return l, path, nodes, true
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
			w.record(vr.Local, nil, []ast.NodeID{x.ID}, ctx)
		}
	case *ast.ParenExpr:
		w.expr(x.X, ctx)
	case *ast.FieldExpr:
		if l, path, nodes, ok := w.capPlace(x); ok {
			w.record(l, path, nodes, ctx)
			return
		}
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
			if l, path, nodes, ok := w.capPlace(x); ok {
				w.record(l, path, nodes, ctx)
				return
			}
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
