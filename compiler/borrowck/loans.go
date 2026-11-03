package borrowck

import (
	"fmt"
	"os"
	"strings"

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
	// selfElided numbers elided lifetimes inside the impl self type
	selfElided int
}

func (w *sigWalker) add(name string, depth int) {
	w.occs = append(w.occs, occ{name, depth})
}

func (w *sigWalker) lifetime(lt string, depth int) {
	if (lt == "" || lt == "'_") && w.inSelf > 0 {
		// an elided lifetime of the impl's self type (`impl Clone for
		// &T`): a fixed impl lifetime, named by position
		w.selfElided++
		w.add(fmt.Sprintf("'Self#%d", w.selfElided), depth)
		return
	}
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
		if !w.inOut {
			w.add("'static", depth)
		}
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
			if w.inSelf == 0 {
				w.selfElided = 0
			}
			w.inSelf++
			w.walk(w.selfTy, depth)
			w.inSelf--
			return
		}
		// Self or Self::Assoc (an associated type of Self is made of what
		// Self holds: an iterator's items borrow what it borrows)
		if len(p.Segments) > 1 {
			w.add("Self::", depth)
		} else {
			w.add("Self", depth)
		}
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
		// a type with lifetime arguments holds its type arguments behind
		// those references (Iter<'a, T> holds &'a T)
		td := depth
		for _, a := range ga.Args {
			if a.Lifetime != "" {
				td = depth + 1
			}
		}
		for _, a := range ga.Args {
			if a.Lifetime != "" {
				w.lifetime(a.Lifetime, depth)
			}
			if a.Type != nil {
				w.walk(a.Type, td)
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

// staticBounded collects the type parameters bounded by 'static.
func staticBounded(g *ast.Generics, out map[string]bool) {
	if g == nil {
		return
	}
	for _, p := range g.Params {
		if p.Kind != ast.GPType {
			continue
		}
		for _, bd := range p.Bounds {
			if bd.Lifetime == "'static" {
				out[p.Name.Name] = true
			}
		}
	}
	for _, wp := range g.Where {
		pt, ok := wp.Ty.(*ast.PathType)
		if !ok || pt.QSelf != nil || pt.Path == nil || len(pt.Path.Segments) != 1 {
			continue
		}
		for _, bd := range wp.Bounds {
			if bd.Lifetime == "'static" {
				out[pt.Path.Segments[0].Ident.Name] = true
			}
		}
	}
}

// sigFlow derives the loan flow of a call from the callee's declared
// signature: lifetimes and type parameters shared by an input and the
// output (with lifetime elision), and inputs written into `&mut` inputs.
func (b *builder) sigFlow(c *typeck.Callee, nargs int, ret Ty, argTys []Ty) *Flow {
	d := c.Def
	var selfConcrete Ty
	if d != nil && d.InTrait() && len(c.Args) > 0 {
		selfConcrete = c.Args[0]
	}
	if d != nil && d.InTrait() && len(c.Args) > 0 && !c.Args[0].HasInfer() && !c.Args[0].HasParam() {
		// the implementation's signature is more precise than the trait's
		if im, _, ok := b.tcx.ResolveAssocFn(d, c.Args); ok && im != nil {
			d = im
		}
	}
	if d == nil || d.Fn == nil || d.Fn.Ast == nil || d.Fn.Ast.Decl == nil {
		return &Flow{All: true}
	}
	decl := d.Fn.Ast.Decl
	params := map[string]bool{"Self": true}
	genericNames(d.Fn.Ast.Generics, params)
	var selfTy ast.Type
	gens := []*ast.Generics{d.Fn.Ast.Generics}
	if p := d.Parent; p != nil && p.Item != nil {
		switch k := p.Item.Kind.(type) {
		case *ast.ImplItem:
			genericNames(k.Generics, params)
			selfTy = k.SelfTy
			gens = append(gens, k.Generics)
		case *ast.TraitItem:
			genericNames(k.Generics, params)
			gens = append(gens, k.Generics)
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
	statics := map[string]bool{}
	staticBounded(d.Fn.Ast.Generics, statics)
	var static []StaticReq
	for i, os := range ins {
		for _, o := range os {
			if o.name == "'static" {
				static = append(static, StaticReq{Arg: i, Depth: o.depth})
			} else if statics[o.name] {
				static = append(static, StaticReq{Arg: i, Depth: o.depth, Deep: true})
			}
		}
	}
	// 'static occurrences relate to nothing else
	for i, os := range ins {
		var keep []occ
		for _, o := range os {
			if o.name != "'static" {
				keep = append(keep, o)
			}
		}
		ins[i] = keep
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
		if sp := decl.SelfParam; sp != nil && sp.Kind == ast.SelfTyped {
			if lt := selfRefLifetime(sp.Ty); lt != "" {
				elided = lt
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
	// output type parameters no input mentions (`collect::<B>()`): built
	// from what the arguments hold
	inNames := map[string]bool{}
	for _, os := range ins {
		for _, o := range os {
			inNames[o.name] = true
		}
	}
	fl := &Flow{}
	seen := map[FlowEdge]bool{}
	addEdge := func(e FlowEdge) {
		if !seen[e] {
			seen[e] = true
			fl.Edges = append(fl.Edges, e)
		}
	}
	// associated types / unconstrained outputs carry what the input holds,
	// aligned at the innermost reference level (an iterator's items hold
	// its borrows; `copied()` drops one level)
	outLv := refLevels(ret, 0)
	for _, o := range outs {
		if o.name == "Self::" {
			for i, os := range ins {
				for _, io := range os {
					if o.name == "Self::" && io.name != "Self" {
						continue
					}
					if o.name != "Self::" && io.depth != 0 {
						continue
					}
					var src Ty
					if o.name == "Self::" {
						src = selfConcrete
					} else if i < len(argTys) {
						src = argTys[i]
					}
					shift := 0
					if src != nil {
						if sl := refLevels(src, 0); sl > outLv+o.depth {
							shift = sl - outLv - o.depth
						}
					}
					addEdge(FlowEdge{Arg: i, From: io.depth + shift, To: o.depth, Deep: true})
				}
			}
			continue
		}
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
					// a lifetime covers the references at its level; what
					// lies below flows through type parameters (and
					// lifetimes) that the signature relates explicitly
					addEdge(FlowEdge{Arg: i, From: io.depth, To: o.depth, Deep: name[0] != '\''})
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
	fl.Static = static
	slotParams := map[string]bool{}
	for k := range params {
		if k != "Self" || selfTy == nil {
			slotParams[k] = true
		}
	}
	func() {
		defer func() {
			if e := recover(); e != nil {
				if debugLoans {
					fmt.Fprintf(os.Stderr, "borrowck: slotFlow %s: %v\n", d.Name, e)
				}
				fl.Slots, fl.Covered, fl.ArgFull, fl.RetLay, fl.ArgLays, fl.IntoSlots = nil, nil, nil, nil, nil, nil
			}
		}()
		slotFlow(fl, d, selfTy, slotParams, gens, argTys, ret, elided, mutIn)
	}()
	return fl
}

// StaticReq: argument Arg must not hold loans of locals at Depth (and
// deeper when Deep): its parameter requires 'static.
type StaticReq struct {
	Arg, Depth int
	Deep       bool
}

// refLevels counts the reference levels of a type: references and
// structs with lifetime parameters (which hold references) add a level.
func refLevels(t Ty, depth int) int {
	if t == nil || depth > 8 {
		return 0
	}
	switch t.Kind {
	case TRef:
		return 1 + refLevels(t.Elem, depth+1)
	case TArray, TSlice:
		return refLevels(t.Elem, depth+1)
	case TTuple:
		m := 0
		for _, e := range t.Elems {
			m = max(m, refLevels(e, depth+1))
		}
		return m
	case TAdt:
		m := 0
		for _, a := range t.Args {
			m = max(m, refLevels(a, depth+1))
		}
		if adtHasLifetimes(t.Def) {
			return 1 + m
		}
		return m
	}
	return 0
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
	place  Place
	mut    bool
	block  int
	stmt   int
	span   lexer.Span
	origin string // universal region pseudo-loan (place.Local = -1)
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
	regions  *sigRegions
	params   []LocalID // argument locals in signature order
	// loans by the local their place is based on (see indexLoans)
	localLoans map[LocalID][]int
	derefLoans map[LocalID][]int
	localLays  []*layout
	placeInfos map[string]*placeInfo
	// sigSlots: the parameters' origins sit in the slots their declared
	// types give them; retLabels labels the return value's slots
	sigSlots  bool
	retLabels []string
}

var debugLoans = os.Getenv("RUSTGO_BORROWCK_DEBUG") != ""

// lstate: loans held by each local in each region slot of its type
// (sparse; see slots.go).
type lstate struct {
	holds map[int]bitset // [local<<slotBits | slot]
}

const slotBits = 7

func skey(l LocalID, s int) int { return int(l)<<slotBits | s }

func keyLocal(k int) int { return k >> slotBits }

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
		if !live.has(keyLocal(i)) {
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

// lval: the loans of a value, per slot of its layout.
type lval struct {
	lay *layout
	s   []bitset
}

func (lc *loanChecker) emptyVal(l *layout) lval {
	v := lval{lay: l, s: make([]bitset, l.n())}
	for i := range v.s {
		v.s[i] = newBitset(lc.nLoans)
	}
	return v
}

// depthView merges the slots of v by depth (the coarse model).
func (lc *loanChecker) depthView(v lval) []bitset {
	dv := make([]bitset, maxDepth)
	for d := range dv {
		dv[d] = newBitset(lc.nLoans)
	}
	for i, d := range v.lay.depth {
		if d < maxDepth {
			dv[d].union(v.s[i])
		}
	}
	return dv
}

// fromDepths builds a value of layout l from loans by depth.
func (lc *loanChecker) fromDepths(dv []bitset, l *layout) lval {
	v := lc.emptyVal(l)
	for i, d := range l.depth {
		if d < maxDepth {
			v.s[i].union(dv[d])
		}
	}
	return v
}

// conv converts v to layout to: slot by slot when the layouts agree, by
// depth otherwise.
func (lc *loanChecker) conv(v lval, to *layout) lval {
	if v.lay == to || (v.lay.key == to.key && v.lay.n() == to.n()) {
		return v
	}
	out := lc.emptyVal(to)
	for j, m := range collapseMap(v.lay, to, 0) {
		for _, i := range m {
			out.s[j].union(v.s[i])
		}
	}
	return out
}

func (lc *loanChecker) localLayout(l LocalID) *layout {
	if lc.localLays[l] == nil {
		lc.localLays[l] = layoutOf(lc.body.Locals[l].Ty)
	}
	return lc.localLays[l]
}

func (lc *loanChecker) localVal(st *lstate, l LocalID) lval {
	v := lc.emptyVal(lc.localLayout(l))
	for i := range v.s {
		if h := st.holds[skey(l, i)]; h != nil {
			v.s[i].copyFrom(h)
		}
	}
	return v
}

// placeInfo: the layout of the value at a place, and for each of its
// slots the slots of the root local it is stored in.
type placeInfo struct {
	lay *layout
	m   [][]int
	raw bool // behind a raw pointer: not tracked
}

// projTy returns the type of a projection of a value of type t (nil:
// unknown).
func (lc *loanChecker) projTy(t Ty, pr Proj) Ty {
	if t == nil {
		return nil
	}
	switch pr.Kind {
	case PDeref:
		switch t.Kind {
		case TRef, TPtr:
			return t.Elem
		case TAdt:
			if len(t.Args) > 0 && t.Def != nil && t.Def.Name == "Box" {
				return t.Args[0]
			}
		}
	case PField:
		switch t.Kind {
		case TTuple:
			if pr.Field < len(t.Elems) {
				return t.Elems[pr.Field]
			}
		case TAdt:
			v := 0
			if pr.Variant > 0 {
				v = pr.Variant - 1
			}
			if fi := adtField(lc.ctx.tcx, t, v, pr.Field); fi != nil {
				return fi.ty
			}
		}
	case PIndex:
		if t.Kind == TArray || t.Kind == TSlice {
			return t.Elem
		}
	}
	return nil
}

func (lc *loanChecker) placeInfo(p Place) *placeInfo {
	if len(p.Proj) == 0 {
		l := lc.localLayout(p.Local)
		return &placeInfo{lay: l, m: rangeMap(0, l.n())}
	}
	k := lc.body.placeString(p)
	if pi, ok := lc.placeInfos[k]; ok {
		return pi
	}
	ty := lc.body.Locals[p.Local].Ty
	cur := lc.localLayout(p.Local)
	m := rangeMap(0, cur.n())
	for _, pr := range p.Proj {
		if pr.Kind == PDeref && (pr.Deref == DRaw || pr.Deref == DRawMut) {
			pi := &placeInfo{raw: true, lay: flatLayout}
			lc.placeInfos[k] = pi
			return pi
		}
		cty := lc.projTy(ty, pr)
		cl := layoutOf(cty)
		var cm [][]int
		if cty != nil && !cur.flat && !cl.flat && cur.root.ty == ty {
			root := cur.root
			switch {
			case pr.Kind == PDeref && ty.Kind == TRef && len(root.kids) == 1:
				cm = rangeMap(root.kids[0].start, cl.n())
			case pr.Kind == PDeref && pr.Deref == DBox && ty.Kind == TAdt && len(root.kids) > 0:
				cm = rangeMap(root.kids[0].start, cl.n())
			case pr.Kind == PField && ty.Kind == TTuple && pr.Field < len(root.kids):
				cm = rangeMap(root.kids[pr.Field].start, cl.n())
			case pr.Kind == PField && ty.Kind == TAdt:
				v := 0
				if pr.Variant > 0 {
					v = pr.Variant - 1
				}
				if fi := adtField(lc.ctx.tcx, ty, v, pr.Field); fi != nil && len(fi.m) == cl.n() {
					cm = fi.m
				}
			case pr.Kind == PIndex && len(root.kids) == 1:
				cm = rangeMap(root.kids[0].start, cl.n())
			}
			if cm != nil {
				// the kid's subtree must have the child's layout
				for _, x := range cm {
					for _, i := range x {
						if i >= cur.n() {
							cm = nil
							break
						}
					}
					if cm == nil {
						break
					}
				}
			}
		}
		if cm == nil {
			shift := 0
			if pr.Kind == PDeref && (pr.Deref == DRef || pr.Deref == DRefMut) {
				shift = 1
			}
			cm = collapseMap(cur, cl, shift)
		}
		nm := make([][]int, len(cm))
		for j, xs := range cm {
			seen := map[int]bool{}
			for _, x := range xs {
				for _, r := range m[x] {
					if !seen[r] {
						seen[r] = true
						nm[j] = append(nm[j], r)
					}
				}
			}
		}
		m, cur, ty = nm, cl, cty
	}
	pi := &placeInfo{lay: cur, m: m}
	lc.placeInfos[k] = pi
	return pi
}

// placeVal returns the loans held by the value at place p.
func (lc *loanChecker) placeVal(st *lstate, p Place) lval {
	pi := lc.placeInfo(p)
	v := lc.emptyVal(pi.lay)
	if pi.raw {
		return v
	}
	for j, rs := range pi.m {
		for _, r := range rs {
			if h := st.holds[skey(p.Local, r)]; h != nil {
				v.s[j].union(h)
			}
		}
	}
	return v
}

func (lc *loanChecker) opVal(st *lstate, o Operand) lval {
	if o.Kind == OConst || (o.Ty != nil && !carriesBorrows(o.Ty)) {
		return lc.emptyVal(layoutOf(o.Ty))
	}
	v := lc.placeVal(st, o.Place)
	if o.Ty != nil {
		v = lc.conv(v, layoutOf(o.Ty))
	}
	return v
}

func (lc *loanChecker) unionVal(a, b lval) {
	b = lc.conv(b, a.lay)
	for i := range a.s {
		a.s[i].union(b.s[i])
	}
}

// refVal is the value of `&place` created by loan li.
func (lc *loanChecker) refVal(st *lstate, p Place, li int, mut bool) lval {
	inner := lc.placeVal(st, p)
	var rl *layout
	if t := inner.lay.root.ty; t != nil && !inner.lay.flat {
		rl = layoutOf(Ref(t, mut))
	}
	var v lval
	if rl != nil && !rl.flat && rl.n() == inner.lay.n()+1 {
		v = lc.emptyVal(rl)
		for i := range inner.s {
			v.s[i+1].union(inner.s[i])
		}
	} else {
		// by depth
		dv := lc.depthView(inner)
		nd := make([]bitset, maxDepth)
		nd[0] = newBitset(lc.nLoans)
		for d := 0; d+1 < maxDepth; d++ {
			nd[d+1] = dv[d]
		}
		v = lc.fromDepths(nd, flatLayout)
	}
	if li >= 0 {
		v.s[0].set(li)
	}
	// a reborrow through references keeps their loans alive (rustc's
	// reborrow constraints: from the last deref outwards, stopping after
	// the first shared reference)
	for i := len(p.Proj) - 1; i >= 0; i-- {
		pr := p.Proj[i]
		if pr.Kind != PDeref || (pr.Deref != DRef && pr.Deref != DRefMut) {
			continue
		}
		pv := lc.placeVal(st, Place{Local: p.Local, Proj: p.Proj[:i]})
		v.s[0].union(lc.depthView(pv)[0])
		if pr.Deref == DRef {
			break
		}
	}
	return v
}

func (lc *loanChecker) store(st *lstate, p Place, v lval) {
	if !carriesBorrows(lc.body.Locals[p.Local].Ty) {
		// values that cannot hold references hold no loans
		n := lc.localLayout(p.Local).n()
		for s := 0; s < n; s++ {
			delete(st.holds, skey(p.Local, s))
		}
		return
	}
	pi := lc.placeInfo(p)
	if pi.raw {
		return
	}
	v = lc.conv(v, pi.lay)
	if len(p.Proj) == 0 {
		for s := range v.s {
			lc.set(st, skey(p.Local, s), v.s[s])
		}
		return
	}
	// writes into a part (or through derefs, into the referent): weak
	// update of the root's slots
	for j, rs := range pi.m {
		for _, r := range rs {
			lc.or(st, skey(p.Local, r), v.s[j])
		}
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
	for _, h := range v.s {
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
			if s.Kind == SAssign && s.Rv.Kind == RRef && !ignoreBorrow(s.Rv.Place) {
				lc.loanAt[[2]int{bi, si}] = len(lc.loans)
				lc.loans = append(lc.loans, loan{place: s.Rv.Place, mut: s.Rv.Mut, block: bi, stmt: si, span: s.Span})
			}
		}
	}
	type originAt struct {
		local LocalID
		slot  int
		loan  int
	}
	var origins []originAt
	if c.ctx.fn != nil && body.ClosureKind < 0 {
		lc.regions = fnRegions(c.ctx.fn)
	}
	for i, ld := range body.Locals {
		if ld.Kind == LArg {
			lc.params = append(lc.params, LocalID(i))
		}
	}
	lc.localLays = make([]*layout, len(body.Locals))
	lc.placeInfos = map[string]*placeInfo{}
	if lc.regions != nil && len(lc.regions.params) == len(lc.params) {
		r := lc.regions
		byName := map[string]int{}
		loanOf := func(name string) int {
			li, ok := byName[name]
			if !ok {
				li = len(lc.loans)
				byName[name] = li
				lc.loans = append(lc.loans, loan{place: Place{Local: -1}, origin: name, span: body.Span})
			}
			return li
		}
		// the parameters' regions by slot, when the declared types
		// describe all of them
		if len(r.inAst) == len(lc.params) {
			w := newLabeler(r.tparams, r.selfAst)
			var placed []originAt
			ok := true
			var names []string
			for pi, l := range lc.params {
				w.label(r.inAst[pi], lc.localLayout(l))
				if !w.full {
					ok = false
					break
				}
				for k, lb := range w.labels {
					if lb[0] == '\'' && lb != "'static" && lb != "'?" && !strings.HasPrefix(lb, "'#h") {
						placed = append(placed, originAt{l, k, -1})
						names = append(names, lb)
					}
				}
			}
			if ok && w.elided == r.nElided {
				lc.sigSlots = true
				for i, o := range placed {
					o.loan = loanOf(names[i])
					origins = append(origins, o)
				}
				w.out = true
				w.label(r.outAst, lc.localLayout(0))
				lc.retLabels = w.labels
			}
		}
		if !lc.sigSlots {
			for pi, os := range r.params {
				lay := lc.localLayout(lc.params[pi])
				for _, o := range os {
					if o.name[0] != '\'' || o.name == "'static" || o.name == "'?" {
						continue
					}
					li := loanOf(o.name)
					for k, d := range lay.depth {
						if d == o.depth {
							origins = append(origins, originAt{lc.params[pi], k, li})
						}
					}
				}
			}
		}
	} else {
		lc.regions = nil
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
	for _, o := range origins {
		v := newBitset(lc.nLoans)
		v.set(o.loan)
		lc.or(in[0], skey(o.local, o.slot), v)
	}
	lc.solve(in)
	for bi := range body.Blocks {
		if in[bi] != nil {
			lc.runBlock(bi, in[bi], true)
		}
	}
}

// solve computes the loans held at the entry of each block (in[0] given).
func (lc *loanChecker) solve(in []*lstate) {
	work := []int{0}
	queued := map[int]bool{0: true}
	for len(work) > 0 {
		bi := work[0]
		work = work[1:]
		queued[bi] = false
		out := lc.runBlock(bi, in[bi], false)
		for _, sc := range lc.body.Blocks[bi].Term.Targets {
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
}

// closSummary: what a closure's result holds of its parameters' regions
// (per result slot: the parameter slots, or captured data).
type closSummary struct {
	ret     *layout
	fromArg [][][2]int // result slot -> (parameter, slot)
	caps    []bool     // result slot holds loans of captured variables
}

// fromArgs reports whether any result slot holds parameter regions.
func (s *closSummary) fromArgs() bool {
	for _, x := range s.fromArg {
		if len(x) > 0 {
			return true
		}
	}
	return false
}

func (s *closSummary) fromCaps() bool {
	for _, x := range s.caps {
		if x {
			return true
		}
	}
	return false
}

var closSummaries = map[Ty]*closSummary{}

// summarizeClosure runs the loan analysis on a closure body with an origin
// per slot of each parameter (and one for the captures).
func summarizeClosure(tcx *typeck.Tcx, t Ty, body *Body) {
	if t == nil || body == nil {
		return
	}
	defer func() {
		if e := recover(); e != nil {
			delete(closSummaries, t)
			if debugLoans {
				fmt.Fprintf(os.Stderr, "borrowck: closure summary: %v\n", e)
			}
		}
	}()
	c := &checker{body: body, seen: map[string]bool{}, ctx: &Ctx{tcx: tcx}}
	lc := &loanChecker{checker: c, loanAt: map[[2]int]int{}, nLocals: len(body.Locals), reported: map[string]bool{}}
	for bi, bl := range body.Blocks {
		for si, s := range bl.Stmts {
			if s.Kind == SAssign && s.Rv.Kind == RRef && !ignoreBorrow(s.Rv.Place) {
				lc.loanAt[[2]int{bi, si}] = len(lc.loans)
				lc.loans = append(lc.loans, loan{place: s.Rv.Place, mut: s.Rv.Mut, block: bi, stmt: si, span: s.Span})
			}
		}
	}
	lc.localLays = make([]*layout, len(body.Locals))
	lc.placeInfos = map[string]*placeInfo{}
	type seed struct {
		local LocalID
		slot  int
		loan  int
	}
	var seeds []seed
	argOf := map[int][2]int{}
	capLoan := len(lc.loans)
	lc.loans = append(lc.loans, loan{place: Place{Local: -1}, origin: "'cap"})
	ai := 0
	for i, ld := range body.Locals {
		switch ld.Kind {
		case LArg:
			for k := 0; k < lc.localLayout(LocalID(i)).n(); k++ {
				argOf[len(lc.loans)] = [2]int{ai, k}
				seeds = append(seeds, seed{LocalID(i), k, len(lc.loans)})
				lc.loans = append(lc.loans, loan{place: Place{Local: -1}, origin: "'arg"})
			}
			ai++
		case LUpvar:
			for k := 0; k < lc.localLayout(LocalID(i)).n(); k++ {
				seeds = append(seeds, seed{LocalID(i), k, capLoan})
			}
		}
	}
	lc.nLoans = len(lc.loans)
	lc.indexLoans()
	lc.dropUse = make([]bool, len(body.Locals))
	lc.computeLiveness()
	in := make([]*lstate, len(body.Blocks))
	in[0] = lc.newState()
	for _, sd := range seeds {
		v := newBitset(lc.nLoans)
		v.set(sd.loan)
		lc.or(in[0], skey(sd.local, sd.slot), v)
	}
	lc.solve(in)
	rl := lc.localLayout(0)
	sm := &closSummary{ret: rl, fromArg: make([][][2]int, rl.n()), caps: make([]bool, rl.n())}
	for bi, bl := range body.Blocks {
		if bl.Term.Kind != TReturn || in[bi] == nil {
			continue
		}
		out := lc.runBlock(bi, in[bi], false)
		for j := 0; j < rl.n(); j++ {
			h := out.holds[skey(0, j)]
			if h == nil {
				continue
			}
			for li := 0; li < lc.nLoans; li++ {
				if !h.has(li) {
					continue
				}
				if li == capLoan {
					sm.caps[j] = true
				} else if a, ok := argOf[li]; ok {
					sm.fromArg[j] = append(sm.fromArg[j], a)
				} else if lc.loans[li].origin == "" && lc.body.Locals[lc.loans[li].place.Local].Kind == LUpvar {
					sm.caps[j] = true
				}
			}
		}
	}
	closSummaries[t] = sm
	if debugLoans {
		fmt.Fprintf(os.Stderr, "borrowck: closure %s summary fromArg=%v caps=%v\n", t, sm.fromArg, sm.caps)
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
	liveLoans := func(i int, ud useDef) bitset {
		ll := newBitset(lc.nLoans)
		live := after[i]
		for i, h := range st.holds {
			// a local overwritten here no longer holds its old loans
			if h != nil && live.has(keyLocal(i)) && LocalID(keyLocal(i)) != ud.def {
				ll.union(h)
			}
		}
		uses := ud.uses
		// loans held by values used by this statement are live here too
		for _, u := range uses {
			for k := 0; k < lc.localLayout(u).n(); k++ {
				if h := st.holds[skey(u, k)]; h != nil {
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
			// the loan this statement creates may still be live from a
			// previous iteration: it is checked like any other
			ll := liveLoans(si, ud)
			lc.checkStmt(st, s, ll, -1)
		}
		lc.transferStmt(st, bi, si, s)
	}
	t := bl.Term
	if report {
		ud := lc.termUseDef(t)
		ll := liveLoans(len(bl.Stmts), ud)
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
		case RUse:
			if len(s.Rv.Ops) > 0 {
				v = lc.opVal(st, s.Rv.Ops[0])
			} else {
				v = lc.emptyVal(flatLayout)
			}
		case RAggregate:
			v = lc.aggVal(st, s)
		case RRef:
			li := -1
			if x, ok := lc.loanAt[[2]int{bi, si}]; ok {
				li = x
			}
			v = lc.refVal(st, s.Rv.Place, li, s.Rv.Mut)
		default:
			v = lc.emptyVal(flatLayout)
		}
		if len(s.Place.Proj) == 0 {
			lc.killDerefLoans(st, s.Place.Local, v)
		}
		lc.store(st, s.Place, v)
	case SStorageDead:
		n := lc.localLayout(s.Place.Local).n()
		for k := 0; k < n; k++ {
			delete(st.holds, skey(s.Place.Local, k))
		}
		lc.killLocalLoans(st, s.Place.Local)
	}
}

// aggVal is the value of an aggregate: each operand lands in the slots of
// its component.
func (lc *loanChecker) aggVal(st *lstate, s Stmt) lval {
	pi := lc.placeInfo(s.Place)
	dl := pi.lay
	out := lc.emptyVal(dl)
	t := dl.root.ty
	ops := s.Rv.Ops
	put := func(o Operand, start int, cl *layout) {
		ov := lc.conv(lc.opVal(st, o), cl)
		for j := range ov.s {
			out.s[start+j].union(ov.s[j])
		}
	}
	if !dl.flat && t != nil {
		switch {
		case t.Kind == TTuple && len(ops) == len(dl.root.kids):
			for k, o := range ops {
				put(o, dl.root.kids[k].start, layoutOf(t.Elems[k]))
			}
			return out
		case t.Kind == TArray && len(dl.root.kids) == 1:
			for _, o := range ops {
				put(o, dl.root.kids[0].start, layoutOf(t.Elem))
			}
			return out
		case t.Kind == TAdt && t.Def.Adt != nil:
			vi := s.Rv.Variant - 1
			if vi < 0 && !t.Def.Adt.IsEnum() {
				vi = 0
			}
			if vi < 0 {
				break
			}
			for k, o := range ops {
				f := k
				if s.Rv.Fields != nil {
					if k >= len(s.Rv.Fields) {
						f = -1
					} else {
						f = s.Rv.Fields[k]
					}
				}
				var fi *fieldInfo
				if f >= 0 {
					fi = adtField(lc.ctx.tcx, t, vi, f)
				}
				if fi == nil {
					lc.unionVal(out, lc.opVal(st, o))
					continue
				}
				ov := lc.conv(lc.opVal(st, o), layoutOf(fi.ty))
				for j, rs := range fi.m {
					if j >= len(ov.s) {
						break
					}
					for _, r := range rs {
						out.s[r].union(ov.s[j])
					}
				}
			}
			return out
		}
	}
	for _, o := range ops {
		lc.unionVal(out, lc.opVal(st, o))
	}
	return out
}

func (lc *loanChecker) transferTerm(st *lstate, t Term) {
	if t.Kind != TCall {
		return
	}
	args := make([]lval, len(t.Args))
	for i, o := range t.Args {
		args[i] = lc.opVal(st, o)
	}
	dpi := lc.placeInfo(t.Dest)
	var v lval
	fl := t.Flow
	if fl == nil {
		v = lc.emptyVal(dpi.lay)
	} else {
		// what each argument contributes by depth (the coarse model)
		dres := make([][]bitset, len(args))
		for i, a := range args {
			dv := lc.depthView(a)
			r := make([]bitset, maxDepth)
			for d := range r {
				r[d] = newBitset(lc.nLoans)
				if fl.All {
					r[d].union(dv[d])
				}
			}
			for _, e := range fl.Edges {
				if e.Arg != i {
					continue
				}
				for k := 0; e.From+k < maxDepth && e.To+k < maxDepth; k++ {
					r[e.To+k].union(dv[e.From+k])
					if !e.Deep {
						break
					}
				}
			}
			dres[i] = r
		}
		precise := fl.RetLay != nil && len(fl.ArgLays) == len(args)
		for i := 0; precise && i < len(args); i++ {
			if t.Args[i].Kind == OConst {
				args[i] = lc.emptyVal(fl.ArgLays[i])
			}
			if args[i].lay.key != fl.ArgLays[i].key || args[i].lay.n() != fl.ArgLays[i].n() {
				precise = false
			}
		}
		if precise {
			v = lc.emptyVal(fl.RetLay)
			for j, d := range fl.RetLay.depth {
				if d >= maxDepth {
					continue
				}
				for i := range args {
					if fl.Covered[j] && fl.ArgFull[i] {
						continue
					}
					v.s[j].union(dres[i][d])
				}
			}
			for _, e := range fl.Slots {
				v.s[e.To].union(args[e.Arg].s[e.From])
			}
		} else {
			merged := make([]bitset, maxDepth)
			for d := range merged {
				merged[d] = newBitset(lc.nLoans)
				for i := range args {
					merged[d].union(dres[i][d])
				}
			}
			v = lc.fromDepths(merged, dpi.lay)
		}
		for _, e := range fl.Into {
			if e.Dst >= len(args) || e.Src >= len(args) {
				continue
			}
			src := lc.depthView(args[e.Src])
			// the places the `&mut` argument borrows receive the loans
			for _, p := range lc.borrowedPlaces(args[e.Dst]) {
				pl := lc.localLayout(p.Local)
				for k, d := range pl.depth {
					if d >= e.DstDepth && d-e.DstDepth+e.SrcDepth < maxDepth {
						lc.or(st, skey(p.Local, k), src[d-e.DstDepth+e.SrcDepth])
					}
				}
			}
		}
		if len(fl.IntoSlots) > 0 && precise {
			for _, e := range fl.IntoSlots {
				dl := fl.ArgLays[e.Dst]
				if dl.flat || dl.root.ty == nil || dl.root.ty.Kind != TRef || e.DstSlot < 1 {
					continue
				}
				rl := layoutOf(dl.root.ty.Elem)
				for _, p := range lc.borrowedPlaces(args[e.Dst]) {
					pi := lc.placeInfo(p)
					if pi.raw || pi.lay.key != rl.key || pi.lay.n() != rl.n() || e.DstSlot-1 >= len(pi.m) {
						continue
					}
					for _, r := range pi.m[e.DstSlot-1] {
						lc.or(st, skey(p.Local, r), args[e.Src].s[e.SrcSlot])
					}
				}
			}
		}
	}
	if len(t.Dest.Proj) == 0 {
		lc.killDerefLoans(st, t.Dest.Local, v)
	}
	lc.store(st, t.Dest, v)
}

// borrowedPlaces returns the places (not behind references) a reference
// value's loans borrow.
func (lc *loanChecker) borrowedPlaces(v lval) []Place {
	var out []Place
	if len(v.s) == 0 {
		return nil
	}
	ref := v.s[0]
	if v.lay.flat || v.lay.root.ty == nil || v.lay.root.ty.Kind != TRef {
		ref = lc.depthView(v)[0]
	}
	for li := 0; li < lc.nLoans; li++ {
		if !ref.has(li) || lc.loans[li].origin != "" {
			continue
		}
		p := lc.loans[li].place
		through := false
		for _, pr := range p.Proj {
			if pr.Kind == PDeref && pr.Deref != DBox {
				through = true
			}
		}
		if !through {
			out = append(out, p)
		}
	}
	return out
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
		if t.Flow != nil && lc.regions != nil {
			lc.checkParamStores(st, t)
		}
		if t.Flow != nil {
			for _, req := range t.Flow.Static {
				if req.Arg < len(t.Args) {
					lc.checkStatic(st, t.Args[req.Arg], req, t.Span)
				}
			}
		}
	case TReturn:
		lc.checkReturn(st, t.Span)
	}
}

// ignoreBorrow: like rustc, borrows of places behind a shared reference or
// a raw pointer are not tracked (the data cannot be mutated through the
// base; the reborrow's lifetime is still tied to the reference's loans).
func ignoreBorrow(p Place) bool {
	for _, pr := range p.Proj {
		if pr.Kind == PDeref && (pr.Deref == DRef || pr.Deref == DRaw || pr.Deref == DRawMut) {
			return true
		}
	}
	return false
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
		if ln.origin != "" {
			continue
		}
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
					holders = append(holders, fmt.Sprintf("_%d@%d", keyLocal(i), i&(1<<slotBits-1)))
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

// checkParamStores checks values stored through `&mut` parameters against
// the lifetimes the parameter's type declares.
func (lc *loanChecker) checkParamStores(st *lstate, t Term) {
	for _, e := range t.Flow.Into {
		if e.Dst >= len(t.Args) || e.Src >= len(t.Args) {
			continue
		}
		dst := lc.depthView(lc.opVal(st, t.Args[e.Dst]))
		src := lc.depthView(lc.opVal(st, t.Args[e.Src]))
		for li := 0; li < lc.nLoans; li++ {
			if !dst[0].has(li) || lc.loans[li].origin != "" {
				continue
			}
			p := lc.loans[li].place
			pi := -1
			for i, l := range lc.params {
				if l == p.Local {
					pi = i
				}
			}
			if pi < 0 {
				continue
			}
			k := 0
			for _, pr := range p.Proj {
				if pr.Kind == PDeref && (pr.Deref == DRef || pr.Deref == DRefMut) {
					k++
				}
			}
			if k == 0 {
				continue // a local copy of the parameter
			}
			for kk := 0; e.SrcDepth+kk < maxDepth && k+e.DstDepth+kk < maxDepth; kk++ {
				set, any := lc.regions.allowed(lc.regions.params[pi], k+e.DstDepth+kk)
				if any {
					break
				}
				for oi := 0; oi < lc.nLoans; oi++ {
					if src[e.SrcDepth+kk].has(oi) && lc.loans[oi].origin != "" && !lc.regions.originOK(lc.loans[oi].origin, set) {
						lc.regionError(lc.loans[oi].origin, t.Span)
						return
					}
				}
			}
		}
	}
}

// checkStatic reports an argument holding loans of locals where 'static is
// required (E0373 for closures, E0597 otherwise).
func (lc *loanChecker) checkStatic(st *lstate, o Operand, req StaticReq, sp lexer.Span) {
	v := lc.depthView(lc.opVal(st, o))
	for d := req.Depth; d < maxDepth; d++ {
		for li := 0; li < lc.nLoans; li++ {
			if !v[d].has(li) {
				continue
			}
			ln := lc.loans[li]
			if ln.origin != "" {
				if lc.regions != nil && !lc.regions.outlives(ln.origin, "'static") {
					lc.regionError(ln.origin, sp)
					return
				}
				continue
			}
			ld := lc.body.Locals[ln.place.Local]
			if ld.Kind == LStatic || ld.Kind == LUpvar {
				continue
			}
			name := lc.describe(Place{Local: ln.place.Local})
			if o.Ty != nil && o.Ty.Kind == TClosure {
				lc.errorf(sp, "E0373", "closure may outlive the current function, but it borrows `%s`, which is owned by the current function", name)
			} else {
				lc.errorf(ln.span, "E0597", "`%s` does not live long enough", name)
			}
			return
		}
		if !req.Deep {
			break
		}
	}
}

// returnOK reports whether an origin may be returned in slot (at depth d)
// of the return value.
func (lc *loanChecker) returnOK(origin string, slot, d int) bool {
	r := lc.regions
	if lc.sigSlots && slot < len(lc.retLabels) && lc.retLabels[slot] != "" {
		lb := lc.retLabels[slot]
		if lb[0] != '\'' {
			return true // a type parameter's instance: any region
		}
		if lb == "'?" {
			lb = r.elided
			if lb == "" {
				return true
			}
		}
		return r.originOK(origin, map[string]bool{lb: true})
	}
	if d >= maxDepth {
		return true
	}
	set, any := r.allowed(r.out, d)
	return any || r.originOK(origin, set)
}

// checkReturn reports returned references to locals (E0515).
func (lc *loanChecker) checkReturn(st *lstate, sp lexer.Span) {
	rl := lc.localLayout(0)
	for slot, d := range rl.depth {
		h, ok := st.holds[skey(0, slot)]
		if !ok {
			continue
		}
		for li := 0; li < lc.nLoans; li++ {
			if !h.has(li) {
				continue
			}
			ln := lc.loans[li]
			if ln.origin != "" {
				if lc.regions != nil && !lc.returnOK(ln.origin, slot, d) {
					lc.regionError(ln.origin, sp)
					return
				}
				continue
			}
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
