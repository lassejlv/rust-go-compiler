package borrowck

import (
	"fmt"
	"os"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
	"rustgo/compiler/typeck"
)

// ---------------------------------------------------------------------------
// Signature flows

// IntoEdge: a call writes arg[Src]'s loans (from depth SrcDepth) into the
// referent of the `&mut` argument Dst (at depth DstDepth of the referent).
type IntoEdge struct {
	Dst, DstDepth, Src, SrcDepth int
}

type occ struct {
	name  string
	depth int
}

type sigWalker struct {
	params  map[string]bool // type params (incl. Self)
	selfTy  ast.Type
	elided  int
	inOut   bool
	occs    []occ
	inSelf  int
	implAll bool // output has `impl Trait`
	named   bool // a named lifetime ('a, 'static) was seen
}

func (w *sigWalker) add(name string, depth int) {
	w.occs = append(w.occs, occ{name, depth})
}

func (w *sigWalker) lifetime(lt string, depth int) {
	if lt == "" || lt == "'_" {
		if w.inOut {
			w.add("'?", depth)
			return
		}
		w.elided++
		w.add(fmt.Sprintf("'#%d", w.elided), depth)
		return
	}
	w.named = true
	if lt == "'static" {
		return
	}
	w.add(lt, depth)
}

func (w *sigWalker) walk(t ast.Type, depth int) {
	if t == nil || depth > 6 {
		return
	}
	switch x := t.(type) {
	case *ast.RefType:
		w.lifetime(x.Lifetime, depth)
		w.walk(x.Elem, depth+1)
	case *ast.SliceType:
		w.walk(x.Elem, depth)
	case *ast.ArrayType:
		w.walk(x.Elem, depth)
	case *ast.ParenType:
		w.walk(x.Elem, depth)
	case *ast.TupleType:
		for _, e := range x.Elems {
			w.walk(e, depth)
		}
	case *ast.PathType:
		if x.QSelf != nil {
			return // <T as Trait>::Assoc: opaque
		}
		w.path(x.Path, depth)
	case *ast.ImplTraitType:
		if w.inOut {
			w.implAll = true
		}
		w.bounds(x.Bounds, depth)
	case *ast.DynTraitType:
		w.bounds(x.Bounds, depth)
	}
}

func (w *sigWalker) bounds(bs []*ast.Bound, depth int) {
	for _, b := range bs {
		if b.Lifetime != "" {
			w.lifetime(b.Lifetime, depth)
		}
		if b.Path != nil {
			w.genericArgsOf(b.Path, depth)
		}
	}
}

func (w *sigWalker) path(p *ast.Path, depth int) {
	if p == nil || len(p.Segments) == 0 {
		return
	}
	first := p.Segments[0].Ident.Name
	if first == "Self" {
		if w.selfTy != nil && w.inSelf < 2 && len(p.Segments) == 1 {
			w.inSelf++
			w.walk(w.selfTy, depth)
			w.inSelf--
			return
		}
		if len(p.Segments) == 1 {
			w.add("Self", depth)
		}
		// Self::Assoc projections are opaque: no flow
		return
	}
	if w.params[first] {
		// T (T::Assoc projections are opaque: no flow)
		if len(p.Segments) == 1 {
			w.add(first, depth)
		}
		return
	}
	w.genericArgsOf(p, depth)
}

func (w *sigWalker) genericArgsOf(p *ast.Path, depth int) {
	for _, seg := range p.Segments {
		ga := seg.Args
		if ga == nil {
			continue
		}
		for _, a := range ga.Args {
			if a.Lifetime != "" {
				w.lifetime(a.Lifetime, depth)
			}
			if a.Type != nil {
				w.walk(a.Type, depth)
			}
		}
		for _, bd := range ga.Bindings {
			if bd.Ty != nil {
				w.walk(bd.Ty, depth)
			}
			w.bounds(bd.Bounds, depth)
		}
		for _, in := range ga.Inputs {
			w.walk(in, depth)
		}
		if ga.Output != nil {
			w.walk(ga.Output, depth)
		}
	}
}

func genericNames(g *ast.Generics, params map[string]bool) {
	if g == nil {
		return
	}
	for _, p := range g.Params {
		if p.Kind == ast.GPType {
			params[p.Name.Name] = true
		}
	}
}

// sigFlow derives the loan flow of a call from the callee's declared
// signature: lifetimes and type parameters shared by an input and the
// output (with lifetime elision), and inputs written into `&mut` inputs.
func (b *builder) sigFlow(c *typeck.Callee, nargs int, ret Ty) *Flow {
	d := c.Def
	if d == nil || d.Fn == nil || d.Fn.Ast == nil || d.Fn.Ast.Decl == nil {
		return &Flow{All: true}
	}
	decl := d.Fn.Ast.Decl
	params := map[string]bool{"Self": true}
	genericNames(d.Fn.Ast.Generics, params)
	var selfTy ast.Type
	if p := d.Parent; p != nil && p.Item != nil {
		switch k := p.Item.Kind.(type) {
		case *ast.ImplItem:
			genericNames(k.Generics, params)
			selfTy = k.SelfTy
		case *ast.TraitItem:
			genericNames(k.Generics, params)
		}
	}
	w := &sigWalker{params: params, selfTy: selfTy}
	var ins [][]occ
	var mutIn []bool
	if sp := decl.SelfParam; sp != nil {
		w.occs = nil
		switch sp.Kind {
		case ast.SelfValue:
			w.path(&ast.Path{Segments: []*ast.PathSegment{{Ident: ast.Ident{Name: "Self"}}}}, 0)
			mutIn = append(mutIn, false)
		case ast.SelfRef, ast.SelfRefMut:
			lt := sp.Lifetime
			if lt == "" {
				lt = "'self"
			}
			w.add(lt, 0)
			w.path(&ast.Path{Segments: []*ast.PathSegment{{Ident: ast.Ident{Name: "Self"}}}}, 1)
			mutIn = append(mutIn, sp.Kind == ast.SelfRefMut)
		case ast.SelfTyped:
			w.walk(sp.Ty, 0)
			rt, ok := sp.Ty.(*ast.RefType)
			mutIn = append(mutIn, ok && rt.Mut)
		}
		ins = append(ins, w.occs)
	}
	for _, p := range decl.Params {
		w.occs = nil
		w.walk(p.Ty, 0)
		ins = append(ins, w.occs)
		rt, ok := p.Ty.(*ast.RefType)
		mutIn = append(mutIn, ok && rt.Mut)
	}
	if len(ins) != nargs {
		return &Flow{All: true}
	}
	// lifetime elision for the output
	elided := ""
	if sp := decl.SelfParam; sp != nil && (sp.Kind == ast.SelfRef || sp.Kind == ast.SelfRefMut) {
		elided = sp.Lifetime
		if elided == "" {
			elided = "'self"
		}
	} else {
		names := map[string]bool{}
		for _, os := range ins {
			for _, o := range os {
				if o.name[0] == '\'' {
					names[o.name] = true
				}
			}
		}
		if len(names) == 1 {
			for n := range names {
				elided = n
			}
		}
	}
	w.occs = nil
	w.inOut = true
	w.walk(decl.Output, 0)
	outs := w.occs
	if w.implAll {
		// `impl Trait` captures the type parameters (edition 2024: also
		// every lifetime)
		all := d.Crate != nil && d.Crate.Edition >= "2024"
		for _, os := range ins {
			for _, o := range os {
				if all || o.name[0] != '\'' {
					outs = append(outs, occ{o.name, 0})
				}
			}
		}
	}
	if len(outs) == 0 && !w.named && !w.implAll && carriesBorrows(ret) && hasLifetimeAdt(ret) {
		// a hidden elided lifetime in a path (`-> Iter<T>`)
		outs = append(outs, occ{"'?", 0})
	}
	fl := &Flow{}
	seen := map[FlowEdge]bool{}
	for _, o := range outs {
		name := o.name
		if name == "'?" {
			if elided == "" {
				continue
			}
			name = elided
		}
		for i, os := range ins {
			for _, io := range os {
				if io.name == name {
					e := FlowEdge{Arg: i, From: io.depth, To: o.depth, Deep: true}
					if !seen[e] {
						seen[e] = true
						fl.Edges = append(fl.Edges, e)
					}
				}
			}
		}
	}
	// inputs stored into `&mut` inputs (v.push(&x))
	for i, os := range ins {
		if !mutIn[i] {
			continue
		}
		for _, io := range os {
			if io.depth < 1 {
				continue
			}
			for j, js := range ins {
				if j == i {
					continue
				}
				for _, jo := range js {
					if jo.name == io.name {
						fl.Into = append(fl.Into, IntoEdge{Dst: i, DstDepth: io.depth - 1, Src: j, SrcDepth: jo.depth})
					}
				}
			}
		}
	}
	return fl
}

func hasLifetimeAdt(t Ty) bool {
	if t == nil {
		return false
	}
	switch t.Kind {
	case TAdt:
		if adtHasLifetimes(t.Def) {
			return true
		}
		for _, a := range t.Args {
			if hasLifetimeAdt(a) {
				return true
			}
		}
	case TTuple:
		for _, e := range t.Elems {
			if hasLifetimeAdt(e) {
				return true
			}
		}
	case TArray, TSlice:
		return hasLifetimeAdt(t.Elem)
	}
	return false
}

// ---------------------------------------------------------------------------
// Loans

const maxDepth = 4

type loan struct {
	place Place
	mut   bool
	block int
	stmt  int
	span  lexer.Span
}

type loanChecker struct {
	*checker
	loans    []loan
	loanAt   map[[2]int]int
	nLocals  int
	nLoans   int
	dropUse  []bool
	liveIn   []bitset // per block
	reported map[string]bool
	curState *lstate
	// loans by the local their place is based on (see indexLoans)
	localLoans map[LocalID][]int
	derefLoans map[LocalID][]int
}

var debugLoans = os.Getenv("RUSTGO_BORROWCK_DEBUG") != ""

// lstate: loans held by each local at each depth (sparse).
type lstate struct {
	holds map[int]bitset // [local*maxDepth + depth]
}

func (lc *loanChecker) newState() *lstate {
	return &lstate{holds: map[int]bitset{}}
}

func (st *lstate) clone() *lstate {
	c := &lstate{holds: make(map[int]bitset, len(st.holds))}
	for i, h := range st.holds {
		c.holds[i] = h.clone()
	}
	return c
}

func (st *lstate) join(o *lstate) bool {
	ch := false
	for i, h := range o.holds {
		if cur, ok := st.holds[i]; ok {
			if cur.union(h) {
				ch = true
			}
			continue
		}
		st.holds[i] = h.clone()
		ch = true
	}
	return ch
}

// prune drops the entries of dead locals.
func (st *lstate) prune(live bitset) {
	for i := range st.holds {
		if !live.has(i / maxDepth) {
			delete(st.holds, i)
		}
	}
}

// or adds v into holds entry i.
func (lc *loanChecker) or(st *lstate, i int, v bitset) {
	if v == nil || v.empty() {
		return
	}
	if cur, ok := st.holds[i]; ok {
		cur.union(v)
		return
	}
	st.holds[i] = v.clone()
}

func (lc *loanChecker) set(st *lstate, i int, v bitset) {
	if v == nil || v.empty() {
		delete(st.holds, i)
		return
	}
	st.holds[i] = v.clone()
}

type lval []bitset // loans by depth

func (lc *loanChecker) emptyVal() lval {
	v := make(lval, maxDepth)
	for i := range v {
		v[i] = newBitset(lc.nLoans)
	}
	return v
}

func (lc *loanChecker) localVal(st *lstate, l LocalID) lval {
	v := lc.emptyVal()
	for d := 0; d < maxDepth; d++ {
		if h := st.holds[int(l)*maxDepth+d]; h != nil {
			v[d].copyFrom(h)
		}
	}
	return v
}

// placeVal returns the loans held by the value at place p.
func (lc *loanChecker) placeVal(st *lstate, p Place) lval {
	v := lc.localVal(st, p.Local)
	for _, pr := range p.Proj {
		if pr.Kind != PDeref {
			continue
		}
		switch pr.Deref {
		case DRef, DRefMut:
			nv := lc.emptyVal()
			for d := 1; d < maxDepth; d++ {
				nv[d-1].copyFrom(v[d])
			}
			v = nv
		case DRaw, DRawMut:
			return lc.emptyVal()
		}
	}
	return v
}

func (lc *loanChecker) opVal(st *lstate, o Operand) lval {
	if o.Kind == OConst || (o.Ty != nil && !carriesBorrows(o.Ty)) {
		return lc.emptyVal()
	}
	return lc.placeVal(st, o.Place)
}

func (lc *loanChecker) unionVal(a, b lval) {
	for d := range a {
		a[d].union(b[d])
	}
}

// refVal is the value of `&place` created by loan li.
func (lc *loanChecker) refVal(st *lstate, p Place, li int) lval {
	v := lc.emptyVal()
	if li >= 0 {
		v[0].set(li)
	}
	// a reborrow through a reference keeps that reference's loans alive
	for i, pr := range p.Proj {
		if pr.Kind == PDeref && (pr.Deref == DRef || pr.Deref == DRefMut) {
			pv := lc.placeVal(st, Place{Local: p.Local, Proj: p.Proj[:i]})
			v[0].union(pv[0])
		}
	}
	inner := lc.placeVal(st, p)
	for d := 0; d+1 < maxDepth; d++ {
		v[d+1].union(inner[d])
	}
	return v
}

func (lc *loanChecker) store(st *lstate, p Place, v lval) {
	base := int(p.Local) * maxDepth
	if !carriesBorrows(lc.body.Locals[p.Local].Ty) {
		// values that cannot hold references hold no loans
		for d := 0; d < maxDepth; d++ {
			delete(st.holds, base+d)
		}
		return
	}
	if len(p.Proj) == 0 {
		for d := 0; d < maxDepth; d++ {
			lc.set(st, base+d, v[d])
		}
		return
	}
	// writes through derefs land in the referent: weak update at the
	// corresponding depth of the root
	n := 0
	for _, pr := range p.Proj {
		if pr.Kind == PDeref && (pr.Deref == DRef || pr.Deref == DRefMut) {
			n++
		}
		if pr.Kind == PDeref && (pr.Deref == DRaw || pr.Deref == DRawMut) {
			return
		}
	}
	for d := 0; d+n < maxDepth; d++ {
		lc.or(st, base+d+n, v[d])
	}
}

// killDerefLoans removes loans of places reached through a deref of local l
// (assigning l changes what they refer to).
func (lc *loanChecker) killDerefLoans(st *lstate, l LocalID, v lval) {
	lis := lc.derefLoans[l]
	if len(lis) == 0 {
		return
	}
	for _, h := range st.holds {
		if h != nil {
			for _, li := range lis {
				h.clear(li)
			}
		}
	}
	// also from the value being assigned (`cur = &mut (*cur).next`)
	for _, h := range v {
		for _, li := range lis {
			h.clear(li)
		}
	}
}

func (lc *loanChecker) killLocalLoans(st *lstate, l LocalID) {
	lis := lc.localLoans[l]
	if len(lis) == 0 {
		return
	}
	for _, h := range st.holds {
		if h != nil {
			for _, li := range lis {
				h.clear(li)
			}
		}
	}
}

// indexLoans records, per local, the loans of places based on it (and those
// reached through a non-Box deref of it).
func (lc *loanChecker) indexLoans() {
	lc.localLoans = map[LocalID][]int{}
	lc.derefLoans = map[LocalID][]int{}
	for li := range lc.loans {
		p := &lc.loans[li].place
		lc.localLoans[p.Local] = append(lc.localLoans[p.Local], li)
		for _, pr := range p.Proj {
			if pr.Kind == PDeref && pr.Deref != DBox {
				lc.derefLoans[p.Local] = append(lc.derefLoans[p.Local], li)
				break
			}
		}
	}
}

// checkLoans runs the loan analysis (NLL) on the body.
func (c *checker) checkLoans() {
	body := c.body
	lc := &loanChecker{checker: c, loanAt: map[[2]int]int{}, nLocals: len(body.Locals), reported: map[string]bool{}}
	for bi, bl := range body.Blocks {
		for si, s := range bl.Stmts {
			if s.Kind == SAssign && s.Rv.Kind == RRef {
				lc.loanAt[[2]int{bi, si}] = len(lc.loans)
				lc.loans = append(lc.loans, loan{place: s.Rv.Place, mut: s.Rv.Mut, block: bi, stmt: si, span: s.Span})
			}
		}
	}
	lc.nLoans = len(lc.loans)
	if lc.nLoans == 0 {
		return
	}
	lc.indexLoans()
	lc.dropUse = make([]bool, len(body.Locals))
	for i, ld := range body.Locals {
		lc.dropUse[i] = c.ctx.dropSignificant(ld.Ty)
	}
	lc.computeLiveness()
	in := make([]*lstate, len(body.Blocks))
	in[0] = lc.newState()
	work := []int{0}
	queued := map[int]bool{0: true}
	for len(work) > 0 {
		bi := work[0]
		work = work[1:]
		queued[bi] = false
		out := lc.runBlock(bi, in[bi], false)
		for _, sc := range body.Blocks[bi].Term.Targets {
			o := out.clone()
			o.prune(lc.liveIn[sc])
			if in[sc] == nil {
				in[sc] = o
			} else if !in[sc].join(o) {
				continue
			}
			if !queued[sc] {
				queued[sc] = true
				work = append(work, sc)
			}
		}
	}
	for bi := range body.Blocks {
		if in[bi] != nil {
			lc.runBlock(bi, in[bi], true)
		}
	}
}

// ---------------------------------------------------------------------------
// Liveness of locals

type useDef struct {
	uses []LocalID
	def  LocalID // -1: none
}

func placeUses(p Place, out *[]LocalID) {
	*out = append(*out, p.Local)
	for _, pr := range p.Proj {
		if pr.Kind == PIndex && pr.Index >= 0 {
			*out = append(*out, pr.Index)
		}
	}
}

func (lc *loanChecker) stmtUseDef(s Stmt) useDef {
	ud := useDef{def: -1}
	switch s.Kind {
	case SAssign:
		switch s.Rv.Kind {
		case RRef, RRawRef:
			placeUses(s.Rv.Place, &ud.uses)
		default:
			for _, o := range s.Rv.Ops {
				if o.Kind != OConst {
					placeUses(o.Place, &ud.uses)
				}
			}
		}
		lc.writeUseDef(s.Place, &ud)
	case SFakeRead:
		placeUses(s.Place, &ud.uses)
	case SDrop:
		if len(s.Place.Proj) == 0 && lc.dropUse[s.Place.Local] {
			ud.uses = append(ud.uses, s.Place.Local)
		}
	case SStorageDead:
		ud.def = s.Place.Local
	}
	return ud
}

func (lc *loanChecker) writeUseDef(p Place, ud *useDef) {
	if len(p.Proj) == 0 {
		ud.def = p.Local
		return
	}
	for _, pr := range p.Proj {
		if pr.Kind == PDeref && pr.Deref != DBox {
			placeUses(p, &ud.uses)
			return
		}
		if pr.Kind == PIndex && pr.Index >= 0 {
			ud.uses = append(ud.uses, pr.Index)
		}
	}
}

func (lc *loanChecker) termUseDef(t Term) useDef {
	ud := useDef{def: -1}
	switch t.Kind {
	case TCall:
		for _, o := range t.Args {
			if o.Kind != OConst {
				placeUses(o.Place, &ud.uses)
			}
		}
		lc.writeUseDef(t.Dest, &ud)
	case TReturn:
		ud.uses = append(ud.uses, 0)
	}
	return ud
}

func (lc *loanChecker) applyUseDef(live bitset, ud useDef) {
	if ud.def >= 0 {
		live.clear(int(ud.def))
	}
	for _, u := range ud.uses {
		live.set(int(u))
	}
}

func (lc *loanChecker) computeLiveness() {
	body := lc.body
	n := len(body.Blocks)
	lc.liveIn = make([]bitset, n)
	for i := range lc.liveIn {
		lc.liveIn[i] = newBitset(lc.nLocals)
	}
	preds := make([][]int, n)
	for bi, bl := range body.Blocks {
		for _, s := range bl.Term.Targets {
			preds[s] = append(preds[s], bi)
		}
	}
	work := make([]int, 0, n)
	queued := make([]bool, n)
	for i := n - 1; i >= 0; i-- {
		work = append(work, i)
		queued[i] = true
	}
	for len(work) > 0 {
		bi := work[0]
		work = work[1:]
		queued[bi] = false
		live := lc.liveOut(bi)
		bl := body.Blocks[bi]
		lc.applyUseDef(live, lc.termUseDef(bl.Term))
		for i := len(bl.Stmts) - 1; i >= 0; i-- {
			lc.applyUseDef(live, lc.stmtUseDef(bl.Stmts[i]))
		}
		if lc.liveIn[bi].union(live) {
			for _, p := range preds[bi] {
				if !queued[p] {
					queued[p] = true
					work = append(work, p)
				}
			}
		}
	}
}

func (lc *loanChecker) liveOut(bi int) bitset {
	live := newBitset(lc.nLocals)
	for _, s := range lc.body.Blocks[bi].Term.Targets {
		live.union(lc.liveIn[s])
	}
	return live
}

// liveAfter returns, for each statement of block bi, the locals live
// after it (index len(stmts): after the terminator).
func (lc *loanChecker) liveAfter(bi int) []bitset {
	bl := lc.body.Blocks[bi]
	out := make([]bitset, len(bl.Stmts)+1)
	live := lc.liveOut(bi)
	out[len(bl.Stmts)] = live.clone()
	lc.applyUseDef(live, lc.termUseDef(bl.Term))
	for i := len(bl.Stmts) - 1; i >= 0; i-- {
		out[i] = live.clone()
		lc.applyUseDef(live, lc.stmtUseDef(bl.Stmts[i]))
	}
	return out
}

// ---------------------------------------------------------------------------
// Transfer and conflicts

type accessKind int

const (
	aRead accessKind = iota
	aShallowRead
	aWrite
	aMove
	aBorrowShared
	aBorrowMut
	aStorageDead
)

func (lc *loanChecker) runBlock(bi int, in *lstate, report bool) *lstate {
	st := in.clone()
	bl := lc.body.Blocks[bi]
	var after []bitset
	if report {
		after = lc.liveAfter(bi)
	}
	liveLoans := func(i int, uses []LocalID) bitset {
		ll := newBitset(lc.nLoans)
		live := after[i]
		for i, h := range st.holds {
			if h != nil && live.has(i/maxDepth) {
				ll.union(h)
			}
		}
		// loans held by values used by this statement are live here too
		for _, u := range uses {
			for d := 0; d < maxDepth; d++ {
				if h := st.holds[int(u)*maxDepth+d]; h != nil {
					ll.union(h)
				}
			}
		}
		return ll
	}
	lc.curState = st
	for si, s := range bl.Stmts {
		if report {
			ud := lc.stmtUseDef(s)
			ll := liveLoans(si, ud.uses)
			own := -1
			if li, ok := lc.loanAt[[2]int{bi, si}]; ok {
				own = li
			}
			lc.checkStmt(st, s, ll, own)
		}
		lc.transferStmt(st, bi, si, s)
	}
	t := bl.Term
	if report {
		ud := lc.termUseDef(t)
		ll := liveLoans(len(bl.Stmts), ud.uses)
		lc.checkTerm(st, t, ll)
	}
	lc.transferTerm(st, t)
	return st
}

func (lc *loanChecker) transferStmt(st *lstate, bi, si int, s Stmt) {
	switch s.Kind {
	case SAssign:
		var v lval
		switch s.Rv.Kind {
		case RUse, RAggregate:
			v = lc.emptyVal()
			for _, o := range s.Rv.Ops {
				lc.unionVal(v, lc.opVal(st, o))
			}
		case RRef:
			li := -1
			if x, ok := lc.loanAt[[2]int{bi, si}]; ok {
				li = x
			}
			v = lc.refVal(st, s.Rv.Place, li)
		default:
			v = lc.emptyVal()
		}
		if len(s.Place.Proj) == 0 {
			lc.killDerefLoans(st, s.Place.Local, v)
		}
		lc.store(st, s.Place, v)
	case SStorageDead:
		base := int(s.Place.Local) * maxDepth
		for d := 0; d < maxDepth; d++ {
			delete(st.holds, base+d)
		}
		lc.killLocalLoans(st, s.Place.Local)
	}
}

func (lc *loanChecker) transferTerm(st *lstate, t Term) {
	if t.Kind != TCall {
		return
	}
	args := make([]lval, len(t.Args))
	for i, o := range t.Args {
		args[i] = lc.opVal(st, o)
	}
	v := lc.emptyVal()
	if t.Flow != nil {
		if t.Flow.All {
			for _, a := range args {
				lc.unionVal(v, a)
			}
		}
		for _, e := range t.Flow.Edges {
			if e.Arg >= len(args) {
				continue
			}
			a := args[e.Arg]
			for k := 0; e.From+k < maxDepth && e.To+k < maxDepth; k++ {
				v[e.To+k].union(a[e.From+k])
				if !e.Deep {
					break
				}
			}
		}
		for _, e := range t.Flow.Into {
			if e.Dst >= len(args) || e.Src >= len(args) {
				continue
			}
			// the places the `&mut` argument borrows receive the loans
			for li := 0; li < lc.nLoans; li++ {
				if !args[e.Dst][0].has(li) {
					continue
				}
				p := lc.loans[li].place
				through := false
				for _, pr := range p.Proj {
					if pr.Kind == PDeref && pr.Deref != DBox {
						through = true
					}
				}
				if through {
					continue
				}
				base := int(p.Local) * maxDepth
				for k := 0; e.DstDepth+k < maxDepth && e.SrcDepth+k < maxDepth; k++ {
					lc.or(st, base+e.DstDepth+k, args[e.Src][e.SrcDepth+k])
				}
			}
		}
	}
	if len(t.Dest.Proj) == 0 {
		lc.killDerefLoans(st, t.Dest.Local, v)
	}
	lc.store(st, t.Dest, v)
}

func (lc *loanChecker) checkStmt(st *lstate, s Stmt, live bitset, own int) {
	switch s.Kind {
	case SAssign:
		switch s.Rv.Kind {
		case RRef:
			k := aBorrowShared
			if s.Rv.Mut {
				k = aBorrowMut
			}
			lc.access(s.Rv.Place, k, live, own, s.Span)
		case RRawRef:
		default:
			for _, o := range s.Rv.Ops {
				lc.operandAccess(o, live, s.Span)
			}
		}
		lc.access(s.Place, aWrite, live, -1, s.Span)
	case SFakeRead:
		lc.access(s.Place, aShallowRead, live, -1, s.Span)
	case SStorageDead:
		lc.access(s.Place, aStorageDead, live, -1, s.Span)
	}
}

func (lc *loanChecker) operandAccess(o Operand, live bitset, sp lexer.Span) {
	switch o.Kind {
	case OCopy:
		lc.access(o.Place, aRead, live, -1, sp)
	case OMove:
		lc.access(o.Place, aMove, live, -1, sp)
	}
}

func (lc *loanChecker) checkTerm(st *lstate, t Term, live bitset) {
	switch t.Kind {
	case TCall:
		for _, o := range t.Args {
			lc.operandAccess(o, live, t.Span)
		}
		lc.access(t.Dest, aWrite, live, -1, t.Span)
	case TReturn:
		lc.checkReturn(st, t.Span)
	}
}

// overlap reports whether an access to place a may touch the loan's place
// l. shallow: the access does not go through references in l beyond a.
func overlap(a, l Place, shallow bool) bool {
	if a.Local != l.Local {
		return false
	}
	n := len(a.Proj)
	if len(l.Proj) < n {
		n = len(l.Proj)
	}
	for i := 0; i < n; i++ {
		pa, pl := a.Proj[i], l.Proj[i]
		if pa.Kind != pl.Kind {
			return true
		}
		switch pa.Kind {
		case PField:
			if pa.Variant != pl.Variant || pa.Field != pl.Field {
				return false
			}
		case PIndex:
			if pa.Index == -2 && pl.Index == -2 && pa.Field != pl.Field {
				return false
			}
		}
	}
	if len(l.Proj) > len(a.Proj) && shallow {
		// the loan is of a part of a: through a reference, writing or
		// killing a does not affect what the reference points to
		for _, pr := range l.Proj[len(a.Proj):] {
			if pr.Kind == PDeref && pr.Deref != DBox {
				return false
			}
		}
	}
	return true
}

func (lc *loanChecker) access(p Place, k accessKind, live bitset, own int, sp lexer.Span) {
	if len(p.Proj) == 0 && lc.body.Locals[p.Local].Kind == LTemp && k == aWrite {
		return
	}
	for li := 0; li < lc.nLoans; li++ {
		if li == own || !live.has(li) {
			continue
		}
		ln := lc.loans[li]
		shallow := k == aWrite || k == aStorageDead || k == aShallowRead
		if !overlap(p, ln.place, shallow) {
			continue
		}
		switch k {
		case aRead, aShallowRead, aBorrowShared:
			if !ln.mut {
				continue
			}
		}
		if debugLoans {
			var holders []string
			for i, h := range lc.curState.holds {
				if h.has(li) {
					holders = append(holders, fmt.Sprintf("_%d@%d", i/maxDepth, i%maxDepth))
				}
			}
			fmt.Fprintf(os.Stderr, "borrowck: %s access %d to %s conflicts with loan %d of %s (%v) held by %v\n", sp.Lo, k, lc.body.placeString(p), li, lc.body.placeString(ln.place), ln.span.Lo, holders)
		}
		lc.conflict(p, k, ln, sp)
		return
	}
}

func (lc *loanChecker) conflict(p Place, k accessKind, ln loan, sp lexer.Span) {
	what := lc.describe(p)
	lp := lc.describe(ln.place)
	key := fmt.Sprintf("%v/%d/%s", sp.Lo, k, what)
	if lc.reported[key] {
		return
	}
	lc.reported[key] = true
	switch k {
	case aBorrowMut:
		if ln.mut {
			lc.errorf(sp, "E0499", "cannot borrow `%s` as mutable more than once at a time", what)
		} else {
			lc.errorf(sp, "E0502", "cannot borrow `%s` as mutable because it is also borrowed as immutable", what)
		}
	case aBorrowShared:
		lc.errorf(sp, "E0502", "cannot borrow `%s` as immutable because it is also borrowed as mutable", what)
	case aRead, aShallowRead:
		lc.errorf(sp, "E0503", "cannot use `%s` because it was mutably borrowed", what)
	case aMove:
		lc.errorf(sp, "E0505", "cannot move out of `%s` because it is borrowed", what)
	case aWrite:
		lc.errorf(sp, "E0506", "cannot assign to `%s` because it is borrowed", what)
	case aStorageDead:
		if lc.body.Locals[p.Local].Kind == LTemp {
			lc.errorf(ln.span, "E0716", "temporary value dropped while borrowed")
		} else {
			lc.errorf(ln.span, "E0597", "`%s` does not live long enough", lp)
		}
	}
}

// checkReturn reports returned references to locals (E0515).
func (lc *loanChecker) checkReturn(st *lstate, sp lexer.Span) {
	for d := 0; d < maxDepth; d++ {
		h, ok := st.holds[d] // local 0
		if !ok {
			continue
		}
		for li := 0; li < lc.nLoans; li++ {
			if !h.has(li) {
				continue
			}
			ln := lc.loans[li]
			through := false
			for _, pr := range ln.place.Proj {
				if pr.Kind == PDeref && pr.Deref != DBox {
					through = true
				}
			}
			ld := lc.body.Locals[ln.place.Local]
			if through || ld.Kind == LUpvar || ld.Kind == LStatic {
				continue
			}
			if ld.Kind == LTemp {
				lc.errorf(ln.span, "E0515", "cannot return value referencing temporary value")
			} else if d == 0 && len(ln.place.Proj) == 0 {
				lc.errorf(ln.span, "E0515", "cannot return reference to local variable `%s`", ld.Name)
			} else {
				lc.errorf(ln.span, "E0515", "cannot return value referencing local variable `%s`", ld.Name)
			}
			return
		}
	}
}

// dropSignificant: values whose drop runs user code that may use the loans
// they hold (types with a Drop impl outside std).
func (ctx *Ctx) dropSignificant(t Ty) bool {
	if t == nil || t.Kind != TAdt || ctx.tcx == nil {
		return false
	}
	if t.Def.Crate != nil && t.Def.Crate.Name == "std" {
		return false
	}
	if !carriesBorrows(t) {
		return false
	}
	dr := ctx.tcx.LangOpt("drop")
	return dr != nil && ctx.tcx.Implements(t, dr)
}
