package borrowck

import (
	"fmt"
	"os"
	"strings"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
	"rustgo/compiler/typeck"
)

// opaqueItemLabels labels, in the hidden type of an `impl Trait<Assoc =
// X>` result (layout rl), the nodes of the concrete type of Assoc (the
// items an iterator stores, `IntoIter<(&'s T, &'a U)>`) by X, so that
// what is extracted from them gets X's regions instead of everything
// the result holds at their depth. okParam vets the type parameters X
// labels slots with.
func opaqueItemLabels(tcx *typeck.Tcx, d *Def, it *ast.ImplTraitType, rl *layout, params map[string]bool, selfAst ast.Type, labels []string, okParam func(string, *lnode) bool) {
	if tcx == nil || d.Fn == nil || rl.flat || rl.root.ty == nil || isWildKind(rl.root.ty) {
		return
	}
	idx := -1
	for i, a := range d.Fn.AstOpaques {
		if a == it {
			idx = i
		}
	}
	if idx < 0 || idx >= len(d.Fn.OpaqueBounds) {
		return
	}
	for _, b := range it.Bounds {
		if b.Path == nil || len(b.Path.Segments) == 0 {
			continue
		}
		seg := b.Path.Segments[len(b.Path.Segments)-1]
		if seg.Args == nil || seg.Args.Parenthesized {
			continue
		}
		for _, bd := range seg.Args.Bindings {
			if bd.Ty == nil {
				continue
			}
			var trait *Def
			for _, p := range d.Fn.OpaqueBounds[idx] {
				if p == nil || p.Trait == nil || len(p.Trait.Args) != 1 {
					continue
				}
				for _, pb := range p.Bindings {
					if pb.Name == bd.Name.Name && len(pb.Args) == 0 {
						if trait != nil && trait != p.Trait.Trait {
							return
						}
						trait = p.Trait.Trait
					}
				}
			}
			if trait == nil {
				continue
			}
			item := tcx.Normalize(Projection(trait, []Ty{rl.root.ty}, bd.Name.Name))
			if item == nil || item.HasProj() || item.HasOpaque() || item.HasParam() || item.HasInfer() || item.HasError() {
				continue
			}
			// (not the captures of closures, which hold other things
			// of that type)
			var found []*lnode
			var find func(n *lnode)
			find = func(n *lnode) {
				switch {
				case n.ty == item:
					found = append(found, n)
				case n.ty != nil && n.ty.Kind == TClosure:
				default:
					for _, k := range n.kids {
						find(k)
					}
				}
			}
			find(rl.root)
			for _, n := range found {
				if n == rl.root || n.start == n.end {
					continue
				}
				sub := newLabeler(params, selfAst)
				sub.out = true
				sub.labels = make([]string, len(labels))
				sub.proj = map[int]string{}
				sub.occ = map[string][]*lnode{}
				sub.walk(bd.Ty, n, 0)
				good := len(sub.proj) == 0
				for k := n.start; k < n.end && good; k++ {
					if sub.labels[k] == "" || labels[k] != "" {
						good = false
					}
				}
				for name, ns := range sub.occ {
					for _, x := range ns {
						if !okParam(name, x) {
							good = false
						}
					}
				}
				if good {
					copy(labels[n.start:n.end], sub.labels[n.start:n.end])
				}
			}
		}
	}
}

// slotFlow derives the precise (per region slot) loan flow of a call from
// the callee's declared signature, for the concrete argument and result
// types: a result slot declared with lifetime 'a receives the argument
// slots declared with 'a (or a lifetime declared to outlive it), the slots
// of a type parameter's instance receive those of the same parameter in
// the arguments. Results built from what other parameters hold
// (associated types `Self::Item`, outputs only bounds relate to the inputs
// like `collect::<B>()` or `F: FnMut(T) -> U`) are aligned by type or by
// reference depth. Slots the signature does not describe keep the
// depth-based flow.
func slotFlow(tcx *typeck.Tcx, fl *Flow, d *Def, selfAst ast.Type, params map[string]bool, gens []*ast.Generics, argTys []Ty, ret Ty, elided string, mutIn []bool) {
	decl := sigDecl(d)
	if ret == nil {
		return
	}
	for _, t := range argTys {
		if t == nil {
			return
		}
	}
	rl := layoutOf(ret)
	if rl.flat {
		return
	}
	selfPath := &ast.PathType{Path: &ast.Path{Segments: []*ast.PathSegment{{Ident: ast.Ident{Name: "Self"}}}}}
	var inAst []ast.Type
	if sp := decl.SelfParam; sp != nil {
		switch sp.Kind {
		case ast.SelfValue:
			inAst = append(inAst, selfPath)
		case ast.SelfRef, ast.SelfRefMut:
			lt := sp.Lifetime
			if lt == "" {
				lt = "'self"
			}
			inAst = append(inAst, &ast.RefType{Lifetime: lt, Mut: sp.Kind == ast.SelfRefMut, Elem: selfPath})
		default:
			inAst = append(inAst, sp.Ty)
		}
	}
	for _, p := range decl.Params {
		inAst = append(inAst, p.Ty)
	}
	if len(inAst) != len(argTys) {
		return
	}
	type argInfo struct {
		lay       *layout
		labels    []string
		proj      map[int]string
		projAssoc map[int]string
		occ    map[string][]*lnode
		full   bool
	}
	w := newLabeler(params, selfAst)
	if p := d.Parent; p != nil && p.Item != nil && selfAst != nil {
		if im, ok := p.Item.Kind.(*ast.ImplItem); ok {
			w.assoc = implAssocTypes(im)
		}
	}
	args := make([]argInfo, len(argTys))
	for i, t := range argTys {
		l := layoutOf(t)
		if l.flat {
			return
		}
		w.label(inAst[i], l)
		args[i] = argInfo{lay: l, labels: w.labels, proj: w.proj, occ: w.occ, full: w.full}
	}
	w.out = true
	w.label(decl.Output, rl)
	out := argInfo{lay: rl, labels: w.labels, proj: w.proj, projAssoc: w.projAssoc, occ: w.occ}
	// a type parameter must have the same instance everywhere (it does,
	// unless the syntax and the types went out of step)
	inst := map[string]string{}
	bad := map[string]bool{}
	note := func(occ map[string][]*lnode) {
		for name, ns := range occ {
			for _, n := range ns {
				k := layoutOf(n.ty).key
				if prev, ok := inst[name]; ok && prev != k {
					bad[name] = true
				}
				inst[name] = k
			}
		}
	}
	for _, a := range args {
		note(a.occ)
	}
	note(out.occ)
	unlabel := func(a *argInfo) {
		for k, lb := range a.labels {
			if i := strings.IndexByte(lb, '#'); i > 0 && lb[0] != '\'' && bad[lb[:i]] {
				a.labels[k] = ""
				a.full = false
			}
		}
	}
	for i := range args {
		unlabel(&args[i])
	}
	unlabel(&out)
	inParams := map[string]bool{}
	for _, a := range args {
		for name := range a.occ {
			inParams[name] = true
		}
	}
	if it, ok := decl.Output.(*ast.ImplTraitType); ok {
		// `impl Iterator<Item = (&'s T, &'a U)>`: the items within the
		// hidden type have the regions the binding declares
		opaqueItemLabels(tcx, d, it, rl, params, selfAst, out.labels, func(name string, n *lnode) bool {
			return inParams[name] && !bad[name] && inst[name] == layoutOf(n.ty).key
		})
	}
	// declared outlives relations between lifetimes ('b: 'a)
	outlive := map[string][]string{} // 'a -> lifetimes that outlive it
	for _, g := range gens {
		if g == nil {
			continue
		}
		for _, gp := range g.Params {
			if gp.Kind == ast.GPLifetime {
				for _, bd := range gp.Bounds {
					if bd.Lifetime != "" {
						outlive[bd.Lifetime] = append(outlive[bd.Lifetime], gp.Name.Name)
					}
				}
			}
		}
		for _, wp := range g.Where {
			if wp.Lifetime != "" {
				for _, bd := range wp.Bounds {
					if bd.Lifetime != "" {
						outlive[bd.Lifetime] = append(outlive[bd.Lifetime], wp.Lifetime)
					}
				}
			}
		}
	}
	bySlot := map[string][][2]int{} // label -> (arg, slot) of full arguments
	for i, a := range args {
		if !a.full {
			continue
		}
		for k, lb := range a.labels {
			bySlot[lb] = append(bySlot[lb], [2]int{i, k})
		}
	}
	fl.RetLay = rl
	fl.ArgLays = make([]*layout, len(args))
	fl.ArgFull = make([]bool, len(args))
	fl.ArgLabels = make([][]string, len(args))
	for i, a := range args {
		fl.ArgLays[i] = a.lay
		fl.ArgFull[i] = a.full
		fl.ArgLabels[i] = a.labels
	}
	fl.Covered = make([]bool, rl.n())
	seen := map[SlotEdge]bool{}
	add := func(e SlotEdge) {
		if !seen[e] {
			seen[e] = true
			fl.Slots = append(fl.Slots, e)
		}
	}
	for j, lb := range out.labels {
		switch {
		case lb == "":
		case lb[0] == '\'':
			name := lb
			if name == "'?" {
				name = elided
				if name == "" {
					continue
				}
			}
			fl.Covered[j] = true
			for _, src := range bySlot[name] {
				add(SlotEdge{Arg: src[0], From: src[1], To: j})
			}
			// transitively: lifetimes declared to outlive it
			vis := map[string]bool{name: true}
			stack := []string{name}
			for len(stack) > 0 {
				x := stack[len(stack)-1]
				stack = stack[:len(stack)-1]
				for _, y := range outlive[x] {
					if !vis[y] {
						vis[y] = true
						stack = append(stack, y)
						for _, src := range bySlot[y] {
							add(SlotEdge{Arg: src[0], From: src[1], To: j})
						}
					}
				}
			}
		default:
			name := lb[:strings.IndexByte(lb, '#')]
			if !inParams[name] {
				continue // see below
			}
			fl.Covered[j] = true
			for _, src := range bySlot[lb] {
				add(SlotEdge{Arg: src[0], From: src[1], To: j})
			}
		}
	}
	// align: the slots of result node n hold what argument node s holds
	// (values produced from it: an iterator's items, a closure's result
	// from its argument). caps: also what closures in s capture.
	align := func(i int, s, n *lnode, caps bool) {
		al := args[i].lay
		if m := findSame(s, n.ty); len(m) > 0 {
			for _, x := range m {
				for k := 0; k < n.end-n.start; k++ {
					add(SlotEdge{Arg: i, From: x.start + k, To: n.start + k})
				}
			}
			return
		}
		// a wrapper without regions of its own around the values
		// (Option<Self::Item>): the values themselves, if s holds them
		// (and no closure, which may make them or hold other values of
		// their type)
		for x := n; !hasClosure(s) && x.ty != nil && x.ty.Kind == TAdt && x.nlt == 0 && len(x.kids) == 1 && x.kids[0].start == x.start && x.kids[0].end == x.end; {
			x = x.kids[0]
			if m := findSame(s, x.ty); len(m) > 0 {
				for _, y := range m {
					for k := 0; k < x.end-x.start; k++ {
						add(SlotEdge{Arg: i, From: y.start + k, To: x.start + k})
					}
				}
				return
			}
		}
		// values made by a closure: its summary (caps: also from what it
		// captures; otherwise from the values it is applied to)
		if es, ok := capturedItems(i, s, n, al, rl); ok && (caps || len(es) > 0) {
			for _, e := range es {
				add(e)
			}
			return
		}
		clos := map[int]bool{}
		if !caps {
			clos = closureSlots(s)
		}
		shift := refLevels(s.ty, 0) - refLevels(n.ty, 0)
		if shift < 0 {
			shift = 0
		}
		for j := n.start; j < n.end; j++ {
			dj := rl.depth[j] - n.depth
			for k := s.start; k < s.end; k++ {
				if !clos[k] && al.depth[k]-s.depth == dj+shift {
					add(SlotEdge{Arg: i, From: k, To: j})
				}
			}
		}
	}
	cover := func(n *lnode) {
		for j := n.start; j < n.end; j++ {
			fl.Covered[j] = true
		}
	}
	// the items of iterator x (an argument's type parameter or Self),
	// by the impls of their types (see items.go): the edges into result
	// node n, rel giving the item slots each slot of n holds; nil when
	// that is not known
	itemEdges := func(x string, n *lnode, rel func(im *itemMap) [][]int) []SlotEdge {
		if !iterParam(d, x, gens) {
			return nil
		}
		// closures that make x's items too (`F: FnMut(Self::Item,
		// Self::Item) -> Self::Item` of reduce)
		type node struct {
			arg int
			n   *lnode
		}
		var prods, xs []node
		for i, a := range args {
			for y, ys := range a.occ {
				if y == x {
					for _, s := range ys {
						xs = append(xs, node{i, s})
					}
					continue
				}
				if len(ys) > 0 && boundMentions(y, x, gens, params) && !onlyClosureInputs(y, x, gens, params) {
					if !producesItems(y, x, gens) {
						return nil
					}
					for _, yn := range ys {
						prods = append(prods, node{i, yn})
					}
				}
			}
		}
		if len(xs) == 0 || len(prods) > 0 && len(xs) > 1 {
			return nil
		}
		var edges []SlotEdge
		for _, xn := range xs {
			s := xn.n
			if layoutOf(s.ty).n() != s.end-s.start {
				return nil
			}
			im := iterItems(tcx, s.ty, 0)
			if debugLoans {
				fmt.Fprintf(os.Stderr, "borrowck: items of %v: %v\n", s.ty, im)
			}
			if im == nil {
				return nil
			}
			rm := rel(im)
			if rm == nil || len(rm) != n.end-n.start {
				return nil
			}
			// the sources of each item slot
			type src struct{ arg, slot int }
			srcs := make([][]src, len(im.m))
			has := func(k int, x src) bool {
				for _, y := range srcs[k] {
					if y == x {
						return true
					}
				}
				return false
			}
			for k, ms := range im.m {
				for _, m := range ms {
					srcs[k] = append(srcs[k], src{xn.arg, s.start + m})
				}
			}
			for _, pr := range prods {
				sm := closSummaries[pr.n.ty]
				if pr.n.ty == nil || pr.n.ty.Kind != TClosure || sm == nil || sm.ret == nil || sm.ret.flat || sm.imprecise || !sameSlots(sm.ret.root.ty, im.lay.root.ty) || len(sm.fromArg) != len(srcs) || len(sm.capVague) != len(srcs) || len(sm.capSlots) != len(srcs) {
					return nil
				}
				for _, pl := range sm.params {
					if pl == nil || pl.flat || !sameSlots(pl.root.ty, im.lay.root.ty) {
						return nil
					}
				}
				for j := range srcs {
					if sm.capVague[j] {
						return nil
					}
					for _, dd := range sm.capSlots[j] {
						if dd >= pr.n.end-pr.n.start {
							return nil
						}
						if x := (src{pr.arg, pr.n.start + dd}); !has(j, x) {
							srcs[j] = append(srcs[j], x)
						}
					}
				}
			}
			// the results the closures return come back as their
			// arguments
			for changed := len(prods) > 0; changed; {
				changed = false
				for _, pr := range prods {
					sm := closSummaries[pr.n.ty]
					for j := range srcs {
						for _, fa := range sm.fromArg[j] {
							if fa[1] >= len(srcs) {
								return nil
							}
							for _, x := range srcs[fa[1]] {
								if !has(j, x) {
									srcs[j] = append(srcs[j], x)
									changed = true
								}
							}
						}
					}
				}
			}
			for j, ks := range rm {
				for _, k := range ks {
					if k >= len(srcs) {
						return nil
					}
					for _, x := range srcs[k] {
						edges = append(edges, SlotEdge{Arg: x.arg, From: x.slot, To: n.start + j})
					}
				}
			}
		}
		if edges == nil {
			edges = []SlotEdge{}
		}
		return edges
	}
	// the items of iterator x when exactly one argument node is x
	itemsOf := func(x string) (int, *lnode, *itemMap) {
		if !iterParam(d, x, gens) {
			return 0, nil, nil
		}
		ai, an := -1, (*lnode)(nil)
		for i, a := range args {
			for _, s := range a.occ[x] {
				if an != nil {
					return 0, nil, nil
				}
				ai, an = i, s
			}
		}
		if an == nil || layoutOf(an.ty).n() != an.end-an.start {
			return 0, nil, nil
		}
		for _, a := range args {
			for y, ys := range a.occ {
				if y != x && len(ys) > 0 && boundMentions(y, x, gens, params) && !onlyClosureInputs(y, x, gens, params) {
					return 0, nil, nil
				}
			}
		}
		im := iterItems(tcx, an.ty, 0)
		if im == nil {
			return 0, nil, nil
		}
		return ai, an, im
	}
	// outputs only the bounds relate to the inputs
	for name, ns := range out.occ {
		if inParams[name] || bad[name] || name == "Self" {
			continue
		}
		// collected items (`B: FromIterator<Self::Item>`)
		if tr, x := itemBoundOf(tcx, d, name, gens, params); tr != nil {
			var all []SlotEdge
			ok := true
			for _, n := range ns {
				es := itemEdges(x, n, func(im *itemMap) [][]int {
					return traitArgSlots(tcx, tr, n.ty, im.lay.root.ty, 0)
				})
				if es == nil {
					ok = false
					break
				}
				all = append(all, es...)
			}
			if ok {
				for _, e := range all {
					add(e)
				}
				for _, n := range ns {
					cover(n)
				}
				continue
			}
		}
		srcs, producers := boundSources(name, gens, params)
		if len(srcs) == 0 && len(producers) == 1 {
			// made by a closure (`F: FnOnce(T) -> U`): its summary says
			// which slots of its arguments and captures the result holds
			argOcc := make([]map[string][]*lnode, len(args))
			for i, a := range args {
				argOcc[i] = a.occ
			}
			var edges []SlotEdge
			ok := true
			for f, inputs := range producers {
				for _, n := range ns {
					es, good := closureProduced(f, inputs, argOcc, n, params, itemsOf)
					if !good {
						ok = false
						break
					}
					edges = append(edges, es...)
				}
			}
			if ok {
				for _, e := range edges {
					add(e)
				}
				for _, n := range ns {
					cover(n)
				}
				continue
			}
		}
		// what the closures producing the values (F: FnMut(T) -> U) make
		// them from: the inputs their results hold loans of
		caps := false
		for f, inputs := range producers {
			used := make([]bool, len(inputs))
			for k := range used {
				used[k] = true
			}
			for _, a := range args {
				for _, fn := range a.occ[f] {
					if sm := closSummaries[fn.ty]; fn.ty != nil && fn.ty.Kind == TClosure && sm != nil {
						for k := range used {
							used[k] = false
						}
						for _, xs := range sm.fromArg {
							for _, x := range xs {
								if x[0] < len(used) {
									used[x[0]] = true
								}
							}
						}
						if sm.fromCaps() {
							caps = true
						}
					}
				}
			}
			for k, in := range inputs {
				if used[k] {
					typeNames(in, params, srcs)
				}
			}
		}
		delete(srcs, name)
		for _, n := range ns {
			cover(n)
			for x := range srcs {
				for i, a := range args {
					for _, s := range a.occ[x] {
						// items of an iterator made by a closure
						if o, c := producedBy([]*lnode{s}); !o {
							align(i, s, n, c)
						}
					}
				}
			}
			if caps {
				for f := range producers {
					for i, a := range args {
						for _, s := range a.occ[f] {
							align(i, s, n, true)
						}
					}
				}
			}
		}
	}
	// associated types of parameters (Self::Item)
	for _, pn := range projNodes(out.proj, rl) {
		x := out.proj[pn.start]
		if out.projAssoc[pn.start] == "Item" {
			es := itemEdges(x, pn, func(im *itemMap) [][]int {
				// the projection's node (projNodes gives the outermost
				// node with its slots: Option<Self::Item>)
				for n := pn; n != nil; {
					if sameSlots(im.lay.root.ty, n.ty) {
						return rangeMap(0, pn.end-pn.start)
					}
					var next *lnode
					for _, k := range n.kids {
						if k.start == pn.start && k.end == pn.end {
							next = k
						}
					}
					n = next
				}
				return nil
			})
			if es != nil {
				for _, e := range es {
					add(e)
				}
				cover(pn)
				continue
			}
		}
		found := false
		for i, a := range args {
			for y, ys := range a.occ {
				// X itself, or what yields X's items too (Chain<A, B>
				// with B: Iterator<Item = A::Item>)
				if y != x && !boundMentions(y, x, gens, params) {
					continue
				}
				for _, s := range ys {
					found = true
					owned, caps := producedBy([]*lnode{s})
					if !owned {
						align(i, s, pn, caps)
					}
				}
			}
		}
		if found {
			cover(pn)
		}
	}
	// `K: Borrow<Q>` with K and Q instances of the same type: only the
	// identity impl applies, so they are the same type, regions included
	// (HashMap::get(&self, k: &Q) with K = &'static str needs a key of
	// type &&'static str). Behind `&'s Map<K, ..>` (covariant) K may
	// shrink, but K: 's: the result's 's slots hold what the key's Q
	// holds; behind `&mut` (invariant) the key's regions are the map's,
	// as if stored into its K slots.
	for _, pr := range borrowPairs(gens, params) {
		type at struct {
			arg int
			n   *lnode
		}
		var as, bs []at
		for i, a := range args {
			for _, n := range a.occ[pr[0]] {
				as = append(as, at{i, n})
			}
			for _, n := range a.occ[pr[1]] {
				bs = append(bs, at{i, n})
			}
		}
		same := len(as) > 0 && len(bs) > 0
		for _, x := range append(append([]at{}, as...), bs...) {
			if x.n.ty == nil || x.n.ty != as[0].n.ty || x.n.end-x.n.start != as[0].n.end-as[0].n.start {
				same = false
			}
		}
		if !same || as[0].n.end == as[0].n.start || hasClosure(as[0].n) {
			continue
		}
		for _, a := range as {
			ref := refAbove(args[a.arg].lay.root, a.n)
			if ref == nil || ref.ty == nil {
				continue
			}
			if !ref.ty.Mut {
				lt := args[a.arg].labels[ref.start]
				if lt == "" || lt[0] != '\'' || lt == "'static" {
					continue
				}
				for j, lb := range out.labels {
					if lb == lt || (lb == "'?" && elided != "" && elided == lt) {
						for _, b := range bs {
							for k := 0; k < b.n.end-b.n.start; k++ {
								add(SlotEdge{Arg: b.arg, From: b.n.start + k, To: j})
							}
						}
					}
				}
				continue
			}
			if ref != args[a.arg].lay.root || a.arg >= len(mutIn) || !mutIn[a.arg] {
				continue
			}
			for _, b := range bs {
				if b.arg == a.arg {
					continue
				}
				for k := 0; k < b.n.end-b.n.start; k++ {
					fl.IntoSlots = append(fl.IntoSlots, IntoSlot{Dst: a.arg, DstSlot: a.n.start + k, Src: b.arg, SrcSlot: b.n.start + k, Name: pr[0]})
				}
				fl.Into = append(fl.Into, IntoEdge{Dst: a.arg, DstDepth: a.n.depth - 1, Src: b.arg, SrcDepth: b.n.depth, Name: pr[0]})
			}
		}
	}
	if debugLoans {
		var al [][]string
		for _, a := range args {
			al = append(al, a.labels)
		}
		fmt.Fprintf(os.Stderr, "borrowck: slotFlow %s args=%q out=%q proj=%v covered=%v full=%v edges=%v\n", d.Name, al, out.labels, out.proj, fl.Covered, fl.ArgFull, fl.Slots)
	}
	// stores through `&mut` arguments of what a bound relates
	// (`extend<I: IntoIterator<Item = T>>(&mut self, iter: I)`)
	for di, a := range args {
		if di >= len(mutIn) || !mutIn[di] || a.lay.root.ty == nil || a.lay.root.ty.Kind != TRef {
			continue
		}
		for name, ns := range a.occ {
			for _, n := range ns {
				if n.start < 1 {
					continue
				}
				for si, sa := range args {
					if si == di {
						continue
					}
					for x, xs := range sa.occ {
						if x == name {
							// the same type parameter (`push(&mut self,
							// value: T)`): slot by slot
							for _, s := range xs {
								alignSame(fl, si, s, di, n, name)
							}
							continue
						}
						if !boundMentions(x, name, gens, params) {
							continue
						}
						for _, s := range xs {
							alignInto(fl, si, s, sa.lay, di, n, a.lay)
						}
					}
				}
			}
		}
	}
	// stores of what a lifetime of the signature relates (`set_b(&mut
	// self, b: &'b str)` into the 'b slots of the referent)
	for di, a := range args {
		if di >= len(mutIn) || !mutIn[di] || a.lay.root.ty == nil || a.lay.root.ty.Kind != TRef {
			continue
		}
		for j := 1; j < len(a.labels); j++ {
			lb := a.labels[j]
			if lb == "" || lb[0] != '\'' || lb == "'?" || lb == "'static" {
				continue
			}
			for si, sa := range args {
				if si == di {
					continue
				}
				for k, l2 := range sa.labels {
					if l2 == lb {
						fl.IntoSlots = append(fl.IntoSlots, IntoSlot{Dst: di, DstSlot: j, Src: si, SrcSlot: k, Name: lb})
					}
				}
			}
		}
	}
}

// closureProduced returns the flow into result node n (the result of
// closure F, whose parameters are the bound's inputs) from the closure's
// summary: its parameters' slots (inputs `X` or `&X` with X a type
// parameter of the arguments) and the slots of the closure value its
// captures sit in. good is false when that cannot be related precisely.
func closureProduced(f string, inputs []ast.Type, occ []map[string][]*lnode, n *lnode, params map[string]bool, itemsOf func(x string) (int, *lnode, *itemMap)) (edges []SlotEdge, good bool) {
	fi := -1
	var fn *lnode
	for i, o := range occ {
		for _, x := range o[f] {
			if fn != nil {
				return nil, false
			}
			fi, fn = i, x
		}
	}
	if fn == nil || fn.ty == nil || fn.ty.Kind != TClosure {
		return nil, false
	}
	sm := closSummaries[fn.ty]
	if sm == nil || sm.ret == nil || sm.ret.flat || sm.imprecise || len(sm.params) != len(inputs) {
		return nil, false
	}
	// the result node within the closure's result
	off := -1
	if sm.ret.root.ty == n.ty {
		off = 0
	} else if hits := findSame(sm.ret.root, n.ty); len(hits) == 1 {
		off = hits[0].start
	}
	if off < 0 || off+(n.end-n.start) > sm.ret.n() {
		return nil, false
	}
	// the argument node each closure parameter's slots come from (refs:
	// the parameter's slot 0 is a reference the callee makes)
	type src struct {
		arg   int
		node  *lnode
		shift int
		items *itemMap // an iterator's items (`X::Item`): their slots in node
	}
	srcOf := func(k int) (src, bool) {
		in := inputs[k]
		for {
			p, ok := in.(*ast.ParenType)
			if !ok {
				break
			}
			in = p.Elem
		}
		if pt, ok := in.(*ast.PathType); ok && itemsOf != nil && pt.QSelf == nil && pt.Path != nil && len(pt.Path.Segments) == 2 && pt.Path.Segments[1].Ident.Name == "Item" && pt.Path.Segments[0].Args == nil && pt.Path.Segments[1].Args == nil {
			i, nd, im := itemsOf(pt.Path.Segments[0].Ident.Name)
			if im == nil || sm.params[k] == nil || sm.params[k].flat || !sameSlots(sm.params[k].root.ty, im.lay.root.ty) {
				return src{}, false
			}
			return src{arg: i, node: nd, items: im}, true
		}
		shift := 0
		if r, ok := in.(*ast.RefType); ok {
			shift = 1
			in = r.Elem
		}
		pt, ok := in.(*ast.PathType)
		if !ok || pt.QSelf != nil || pt.Path == nil || len(pt.Path.Segments) != 1 || pt.Path.Segments[0].Args != nil {
			return src{}, false
		}
		x := pt.Path.Segments[0].Ident.Name
		if !params[x] || x == "Self" {
			return src{}, false
		}
		var s src
		found := false
		for i, o := range occ {
			for _, nd := range o[x] {
				if found {
					return src{}, false
				}
				s, found = src{arg: i, node: nd, shift: shift}, true
			}
		}
		if !found {
			return src{}, false
		}
		pl := sm.params[k]
		xl := layoutOf(s.node.ty)
		if pl.flat || xl.flat || pl.n() != xl.n()+shift || pl.key != slotKeyPrefix(shift)+xl.key {
			return src{}, false
		}
		return s, true
	}
	for j := 0; j < n.end-n.start; j++ {
		rj := off + j
		if sm.capVague[rj] {
			return nil, false
		}
		for _, a := range sm.fromArg[rj] {
			s, ok := srcOf(a[0])
			if ok && s.items != nil {
				if a[1] >= len(s.items.m) {
					return nil, false
				}
				for _, x := range s.items.m[a[1]] {
					edges = append(edges, SlotEdge{Arg: s.arg, From: s.node.start + x, To: n.start + j})
				}
				continue
			}
			if !ok || a[1] < s.shift {
				return nil, false
			}
			edges = append(edges, SlotEdge{Arg: s.arg, From: s.node.start + a[1] - s.shift, To: n.start + j})
		}
		for _, d := range sm.capSlots[rj] {
			if d >= fn.end-fn.start {
				return nil, false
			}
			edges = append(edges, SlotEdge{Arg: fi, From: fn.start + d, To: n.start + j})
		}
	}
	return edges, true
}

// capturedItems relates the values made by the one closure within node s
// of argument i that produces values (an iterator's items: Map<I, F>) to
// result node n from its summary: the slots of the closure value its
// captures sit in and, when it also returns what its arguments hold, the
// other slots of s by depth. ok is false when the summary does not say.
func capturedItems(i int, s, n *lnode, al, rl *layout) (edges []SlotEdge, ok bool) {
	var c *lnode
	var sm *closSummary
	ok = true
	var walk func(x *lnode)
	walk = func(x *lnode) {
		if x.ty != nil && x.ty.Kind == TClosure {
			m := closSummaries[x.ty]
			switch {
			case m == nil || m.ret == nil:
				ok = false
			case m.ret.n() == 0:
				// produces nothing that holds loans (a predicate)
			case c != nil:
				ok = false
			default:
				c, sm = x, m
			}
			return
		}
		for _, k := range x.kids {
			walk(k)
		}
	}
	walk(s)
	if !ok || c == nil || sm.ret.flat || sm.imprecise {
		return nil, false
	}
	for _, v := range sm.capVague {
		if v {
			return nil, false
		}
	}
	pairs := relateNodes(n, sm.ret)
	if len(pairs) == 0 {
		return nil, false
	}
	for _, pr := range pairs {
		m, h := pr[0], pr[1]
		if m.end-m.start != h.end-h.start {
			return nil, false
		}
		for j := 0; j < h.end-h.start; j++ {
			for _, d := range sm.capSlots[h.start+j] {
				if d >= c.end-c.start {
					return nil, false
				}
				edges = append(edges, SlotEdge{Arg: i, From: c.start + d, To: m.start + j})
			}
		}
	}
	if sm.fromArgs() {
		// the values the closure is applied to, where s holds them
		// (outside closures) with the type of its one parameter
		// (Map<vec::IntoIter<(&'s T, &'a U)>, F>): the result slots
		// take the slots of the parameter the summary names
		if len(sm.params) == 1 && !sm.params[0].flat && sm.params[0].root.ty != nil {
			var ins []*lnode
			var find func(x *lnode)
			find = func(x *lnode) {
				switch {
				case x.ty == sm.params[0].root.ty:
					ins = append(ins, x)
				case x.ty != nil && x.ty.Kind == TClosure:
				default:
					for _, k := range x.kids {
						find(k)
					}
				}
			}
			find(s)
			good := len(ins) > 0
			for _, x := range ins {
				if x.end-x.start != sm.params[0].n() {
					good = false
				}
			}
			var es []SlotEdge
			for _, pr := range pairs {
				m, h := pr[0], pr[1]
				for j := 0; good && j < h.end-h.start; j++ {
					for _, a := range sm.fromArg[h.start+j] {
						if a[0] != 0 {
							good = false
							break
						}
						for _, x := range ins {
							es = append(es, SlotEdge{Arg: i, From: x.start + a[1], To: m.start + j})
						}
					}
				}
			}
			if good {
				return append(edges, es...), true
			}
		}
		// the items of what the closure is applied to, by depth
		clos := closureSlots(s)
		shift := max(refLevels(s.ty, 0)-refLevels(n.ty, 0), 0)
		for j := n.start; j < n.end; j++ {
			dj := rl.depth[j] - n.depth
			for k := s.start; k < s.end; k++ {
				if !clos[k] && al.depth[k]-s.depth == dj+shift {
					edges = append(edges, SlotEdge{Arg: i, From: k, To: j})
				}
			}
		}
	}
	return edges, true
}

// relateNodes relates the nodes of result node n to those of a closure's
// result layout r that hold the same values: n is (or holds) r's type
// (`collect` into Vec<R>), r holds n's (`next` of Option<N>), or both hold
// a type T once in r and in n's every slot (Option<&T> collected into
// Vec<&T>). Pairs are {node of n, node of r}; nil when unrelated.
func relateNodes(n *lnode, r *layout) [][2]*lnode {
	if r == nil || r.flat || n == nil {
		return nil
	}
	var out [][2]*lnode
	if n.ty == r.root.ty {
		return [][2]*lnode{{n, r.root}}
	}
	if xs := findSame(n, r.root.ty); len(xs) > 0 {
		for _, x := range xs {
			out = append(out, [2]*lnode{x, r.root})
		}
		return out
	}
	if hs := findSame(r.root, n.ty); len(hs) == 1 {
		return [][2]*lnode{{n, hs[0]}}
	}
	var try func(x *lnode) [][2]*lnode
	try = func(x *lnode) [][2]*lnode {
		if x != n && x.end > x.start {
			if hs := findSame(r.root, x.ty); len(hs) == 1 {
				xs := findSame(n, x.ty)
				covered := 0
				for _, y := range xs {
					covered += y.end - y.start
				}
				if covered == n.end-n.start {
					var ps [][2]*lnode
					for _, y := range xs {
						ps = append(ps, [2]*lnode{y, hs[0]})
					}
					return ps
				}
			}
		}
		for _, k := range x.kids {
			if ps := try(k); ps != nil {
				return ps
			}
		}
		return nil
	}
	return try(n)
}

// slotKeyPrefix is the prefix references add to a slot key.
func slotKeyPrefix(refs int) string {
	return strings.Repeat("&", refs)
}

// alignInto records stores of argument si's node s into node n of the
// referent of `&mut` argument di, aligned by depth.
// alignSame stores the slots of node s (argument si) into the same slots
// of node n (in the referent of `&mut` argument di), both instances of type
// parameter name.
func alignSame(fl *Flow, si int, s *lnode, di int, n *lnode, name string) {
	if s.ty != n.ty || s.end-s.start != n.end-n.start || len(closureSlots(s)) > 0 && hasTrue(closureSlots(s)) {
		return
	}
	for k := 0; k < s.end-s.start; k++ {
		fl.IntoSlots = append(fl.IntoSlots, IntoSlot{Dst: di, DstSlot: n.start + k, Src: si, SrcSlot: s.start + k, Name: name})
	}
}

func hasTrue(m map[int]bool) bool {
	for _, v := range m {
		if v {
			return true
		}
	}
	return false
}

func alignInto(fl *Flow, si int, s *lnode, sl *layout, di int, n *lnode, dl *layout) {
	clos := closureSlots(s)
	// the item type itself inside s (`[(&'a str, &'b str); 1]` extending
	// a Vec<(&'a str, &'b str)>): slot by slot
	if hits := findSame(s, n.ty); len(hits) > 0 && len(clos) == 0 {
		ok := true
		for _, h := range hits {
			if h.end-h.start != n.end-n.start {
				ok = false
			}
		}
		if ok {
			for _, h := range hits {
				for k := 0; k < n.end-n.start; k++ {
					fl.IntoSlots = append(fl.IntoSlots, IntoSlot{Dst: di, DstSlot: n.start + k, Src: si, SrcSlot: h.start + k})
				}
			}
			return
		}
	}
	shift := refLevels(s.ty, 0) - refLevels(n.ty, 0)
	if shift < 0 {
		shift = 0
	}
	for j := n.start; j < n.end; j++ {
		dj := dl.depth[j] - n.depth
		for k := s.start; k < s.end; k++ {
			if !clos[k] && sl.depth[k]-s.depth == dj+shift {
				fl.IntoSlots = append(fl.IntoSlots, IntoSlot{Dst: di, DstSlot: j, Src: si, SrcSlot: k})
			}
		}
	}
}

// boundMentions reports whether a bound of type parameter x mentions y
// (`I: IntoIterator<Item = T>`).
func boundMentions(x, y string, gens []*ast.Generics, params map[string]bool) bool {
	m := map[string]bool{}
	for _, g := range gens {
		if g == nil {
			continue
		}
		for _, gp := range g.Params {
			if gp.Kind == ast.GPType && gp.Name.Name == x {
				for _, b := range gp.Bounds {
					if b.Path != nil {
						for _, seg := range b.Path.Segments {
							genericArgNames(seg.Args, params, m)
						}
					}
				}
			}
		}
		for _, wp := range g.Where {
			pt, ok := wp.Ty.(*ast.PathType)
			if !ok || pt.QSelf != nil || pt.Path == nil || len(pt.Path.Segments) != 1 || pt.Path.Segments[0].Ident.Name != x {
				continue
			}
			for _, b := range wp.Bounds {
				if b.Path != nil {
					for _, seg := range b.Path.Segments {
						genericArgNames(seg.Args, params, m)
					}
				}
			}
		}
	}
	return m[y]
}

// findSame returns the nodes of the subtree s (s included) of type t.
// hasClosure reports whether the subtree s holds a closure (or a value of
// a type whose regions are unknown, which may be one).
func hasClosure(s *lnode) bool {
	if s == nil {
		return false
	}
	if s.wild || (s.ty != nil && s.ty.Kind == TClosure) {
		return true
	}
	for _, k := range s.kids {
		if hasClosure(k) {
			return true
		}
	}
	return false
}

func findSame(s *lnode, t Ty) []*lnode {
	if s == nil || t == nil {
		return nil
	}
	if s.ty == t {
		return []*lnode{s}
	}
	var out []*lnode
	for _, k := range s.kids {
		out = append(out, findSame(k, t)...)
	}
	return out
}

// closureSlots marks the slots of closures within s: an iterator's items
// never hold what its closures capture (closure results do not borrow from
// the closure, see the CallClosure flow).
func closureSlots(s *lnode) map[int]bool {
	m := map[int]bool{}
	var walk func(n *lnode)
	walk = func(n *lnode) {
		if n.ty != nil && n.ty.Kind == TClosure {
			for k := n.start; k < n.end; k++ {
				m[k] = true
			}
			return
		}
		for _, k := range n.kids {
			walk(k)
		}
	}
	walk(s)
	return m
}

// projNodes returns the maximal nodes of l whose slots are all marked in
// proj (with the same parameter).
func projNodes(proj map[int]string, l *layout) []*lnode {
	if len(proj) == 0 {
		return nil
	}
	var out []*lnode
	var walk func(n *lnode)
	walk = func(n *lnode) {
		if n.end > n.start {
			x, ok := proj[n.start]
			all := ok
			for k := n.start; all && k < n.end; k++ {
				if proj[k] != x {
					all = false
				}
			}
			if all {
				out = append(out, n)
				return
			}
		}
		for _, k := range n.kids {
			walk(k)
		}
	}
	walk(l.root)
	return out
}

// producedBy reports, for values produced by the closures in nodes (their
// results): owned when a closure's result holds nothing of its arguments
// or captures (`|s| Cow::Owned(s.clone())`); caps when one returns what
// it captures. Closures without a summary count as producing from their
// arguments.
func producedBy(nodes []*lnode) (owned, caps bool) {
	var walk func(n *lnode)
	walk = func(n *lnode) {
		if n.ty != nil && n.ty.Kind == TClosure {
			sm := closSummaries[n.ty]
			if sm == nil || sm.ret.n() == 0 {
				return
			}
			if sm.fromCaps() {
				caps = true
			} else if !sm.fromArgs() {
				owned = true
			}
			return
		}
		for _, k := range n.kids {
			walk(k)
		}
	}
	for _, n := range nodes {
		walk(n)
	}
	return
}
