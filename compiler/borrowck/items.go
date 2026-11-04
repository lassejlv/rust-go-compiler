package borrowck

import (
	"fmt"
	"os"
	"strings"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
	"rustgo/compiler/typeck"
)

// Items of iterators by impl: which slots of an iterator value the items
// it yields hold, read off the impl that defines its `Item` (`impl<'a, K,
// V> Iterator for Keys<'a, K, V> { type Item = &'a K; }`: the item's
// reference is the iterator's 'a slot, its referent K's slots). Adapters
// relate their items to those of the iterators they wrap through their
// bounds (`I: Iterator<Item = &'a T>` of Copied, `F: FnMut(I::Item) -> B`
// of Map, with the closure's summary). Types carry no lifetimes, so the
// alignment by type of the generic flow cannot tell the keys of a
// HashMap<&'static str, &'a str> from its values; the impls can.

// itemMap: the layout of an iterator's items and, per item slot, the
// slots of the iterator (in layoutOf of its type) the item holds.
type itemMap struct {
	lay *layout
	m   [][]int
}

// ibind: what a type parameter (or `X::Item`) of an impl stands for: its
// instance type and, per slot of it, the source slots.
type ibind struct {
	ty Ty
	m  [][]int
}

type ienv struct {
	tcx     *typeck.Tcx
	params  map[string]bool
	iterish map[string]bool // parameters bounded by an iterator trait
	nest    int
	lts     map[string][]int
	tys     map[string]*ibind
	subs    map[string]*itemMap // X -> items of X's instance (for X::Item)
	failed  map[string]bool
	changed bool
	bad     bool
}

func unionInts(dst, src []int) ([]int, bool) {
	ch := false
	for _, x := range src {
		found := false
		for _, y := range dst {
			if x == y {
				found = true
				break
			}
		}
		if !found {
			dst = append(dst, x)
			ch = true
		}
	}
	return dst, ch
}

func namedLt(lb string) bool {
	return len(lb) > 1 && lb[0] == '\'' && lb != "'?" && lb != "'_" && !strings.HasPrefix(lb, "'#") && !strings.HasPrefix(lb, "'Self#")
}

func sameSlots(a, b Ty) bool {
	if a == b {
		return true
	}
	la, lb := layoutOf(a), layoutOf(b)
	return !la.flat && !lb.flat && la.key == lb.key && la.n() == lb.n()
}

func (e *ienv) addLt(name string, src []int) {
	if !namedLt(name) || name == "'static" {
		return
	}
	cur, ok := e.lts[name]
	u, ch := unionInts(cur, src)
	if !ok || ch {
		if u == nil {
			u = []int{}
		}
		e.lts[name] = u
		e.changed = true
	}
}

func (e *ienv) bind(name string, ty Ty) *ibind {
	b := e.tys[name]
	if b == nil {
		l := layoutOf(ty)
		if l.flat {
			e.bad = true
			return nil
		}
		b = &ibind{ty: ty, m: make([][]int, l.n())}
		e.tys[name] = b
		e.changed = true
		return b
	}
	if !sameSlots(b.ty, ty) {
		e.bad = true
		return nil
	}
	return b
}

func (e *ienv) addSlot(b *ibind, k int, src []int) {
	if k >= len(b.m) {
		e.bad = true
		return
	}
	u, ch := unionInts(b.m[k], src)
	if ch {
		b.m[k] = u
		e.changed = true
	}
}

type projNode struct {
	n   *lnode
	key string // "X::Item"
}

// labelAst labels layout l by the impl type a (lifetimes, parameters and
// `X::Assoc` projections of the impl's parameters).
func labelAst(params map[string]bool, a ast.Type, l *layout) (labels []string, occ map[string][]*lnode, projs []projNode) {
	w := newLabeler(params, nil)
	w.out = true
	w.label(a, l)
	for _, pn := range w.projAt {
		if pn.end > pn.start {
			projs = append(projs, projNode{n: pn, key: w.proj[pn.start] + "::" + w.projAssoc[pn.start]})
		}
	}
	return w.labels, w.occ, projs
}

// item returns the binding of `x::Item` (nil: unknown).
func (e *ienv) item(x string) *ibind {
	key := x + "::Item"
	if e.tys[x] == nil && strings.HasSuffix(x, "::Item") && e.iterish[x] {
		e.item(strings.TrimSuffix(x, "::Item"))
	}
	xb := e.tys[x]
	if xb == nil || !e.iterish[x] || e.failed[x] {
		return e.tys[key]
	}
	sub := e.subs[x]
	if sub == nil {
		sub = iterItems(e.tcx, xb.ty, e.nest+1)
		if sub == nil {
			e.failed[x] = true
			e.bad = true
			return nil
		}
		e.subs[x] = sub
	}
	b := e.bind(key, sub.lay.root.ty)
	if b == nil {
		return nil
	}
	for k, ss := range sub.m {
		var src []int
		for _, s := range ss {
			if s >= len(xb.m) {
				e.bad = true
				return nil
			}
			src, _ = unionInts(src, xb.m[s])
		}
		e.addSlot(b, k, src)
	}
	return b
}

// unify binds the names of impl type a, laid over layout l whose slots
// hold src(k).
func (e *ienv) unify(a ast.Type, l *layout, src func(k int) []int) {
	labels, occ, projs := labelAst(e.params, a, l)
	for k, lb := range labels {
		if lb != "" && lb[0] == '\'' {
			e.addLt(lb, src(k))
		}
	}
	for name, ns := range occ {
		for _, n := range ns {
			b := e.bind(name, n.ty)
			if b == nil {
				return
			}
			for k := n.start; k < n.end; k++ {
				e.addSlot(b, k-n.start, src(k))
			}
		}
	}
	for _, p := range projs {
		b := e.bind(p.key, p.n.ty)
		if b == nil {
			return
		}
		for k := p.n.start; k < p.n.end; k++ {
			e.addSlot(b, k-p.n.start, src(k))
		}
	}
}

// resolve returns the sources of each slot of layout l declared as impl
// type a (done[k]: slot k is known).
func (e *ienv) resolve(a ast.Type, l *layout) (out [][]int, done []bool) {
	out = make([][]int, l.n())
	done = make([]bool, l.n())
	labels, occ, projs := labelAst(e.params, a, l)
	for k, lb := range labels {
		if lb == "'static" {
			done[k] = true
		} else if s, ok := e.lts[lb]; ok && namedLt(lb) {
			out[k], done[k] = s, true
		}
	}
	setNode := func(b *ibind, n *lnode) {
		if b == nil || !sameSlots(b.ty, n.ty) {
			return
		}
		for k := n.start; k < n.end; k++ {
			out[k], done[k] = b.m[k-n.start], true
		}
	}
	for name, ns := range occ {
		for _, n := range ns {
			setNode(e.tys[name], n)
		}
	}
	for _, p := range projs {
		x := strings.TrimSuffix(p.key, "::Item")
		if x != p.key {
			setNode(e.item(x), p.n)
		} else {
			setNode(e.tys[p.key], p.n)
		}
	}
	return out, done
}

func allTrue(bs []bool) bool {
	for _, b := range bs {
		if !b {
			return false
		}
	}
	return true
}

// implBound is a bound of an impl's generics on a single-name subject.
type implBound struct {
	subject string
	b       *ast.Bound
}

func implBounds(g *ast.Generics) []implBound {
	var out []implBound
	if g == nil {
		return nil
	}
	for _, gp := range g.Params {
		if gp.Kind == ast.GPType {
			for _, b := range gp.Bounds {
				out = append(out, implBound{gp.Name.Name, b})
			}
		}
	}
	for _, wp := range g.Where {
		pt, ok := wp.Ty.(*ast.PathType)
		if !ok || pt.QSelf != nil || pt.Path == nil || len(pt.Path.Segments) != 1 || pt.Path.Segments[0].Args != nil {
			continue
		}
		for _, b := range wp.Bounds {
			out = append(out, implBound{pt.Path.Segments[0].Ident.Name, b})
		}
	}
	return out
}

func boundLastSeg(b *ast.Bound) *ast.PathSegment {
	if b == nil || b.Path == nil || len(b.Path.Segments) == 0 {
		return nil
	}
	return b.Path.Segments[len(b.Path.Segments)-1]
}

func isIterTraitName(s string) bool {
	switch s {
	case "Iterator", "IntoIterator", "DoubleEndedIterator", "ExactSizeIterator", "FusedIterator":
		return true
	}
	return false
}

// iterItems returns the item map of iterator (or IntoIterator) type t
// (nil: unknown).
func iterItems(tcx *typeck.Tcx, t Ty, nest int) (im *itemMap) {
	if tcx == nil || t == nil || nest > 6 || isWildKind(t) || t.HasParam() || t.HasInfer() || t.HasError() {
		return nil
	}
	tl := layoutOf(t)
	if tl.flat {
		return nil
	}
	defer func() {
		if recover() != nil {
			im = nil
		}
	}()
	for _, name := range []string{"iterator", "into_iterator"} {
		tr := tcx.LangOpt(name)
		if tr == nil {
			return nil
		}
		impl, iargs := tcx.SelectImplSimple(&TraitRef{Trait: tr, Args: []Ty{t}})
		if impl == nil {
			if tcx.Implements(t, tr) {
				return nil
			}
			continue
		}
		return implItems(tcx, t, tl, tr, impl, iargs, nest)
	}
	return nil
}

func implItems(tcx *typeck.Tcx, t Ty, tl *layout, tr *Def, impl *Def, iargs []Ty, nest int) *itemMap {
	if impl.Impl == nil || impl.Impl.Ast == nil {
		return nil
	}
	ia := impl.Impl.Ast
	itemAst := implAssocTypes(ia)["Item"]
	if itemAst == nil || ia.SelfTy == nil {
		return nil
	}
	itemTy := tcx.Normalize(Projection(tr, []Ty{t}, "Item"))
	if itemTy == nil || itemTy.HasProj() || itemTy.HasParam() || itemTy.HasInfer() || itemTy.HasError() || itemTy.HasOpaque() {
		return nil
	}
	il := layoutOf(itemTy)
	if il.flat {
		return nil
	}
	e := &ienv{tcx: tcx, params: map[string]bool{}, iterish: map[string]bool{}, nest: nest,
		lts: map[string][]int{}, tys: map[string]*ibind{}, subs: map[string]*itemMap{}, failed: map[string]bool{}}
	if ia.Generics != nil {
		for _, gp := range ia.Generics.Params {
			if gp.Kind == ast.GPType {
				e.params[gp.Name.Name] = true
			}
		}
	}
	bounds := implBounds(ia.Generics)
	for _, ib := range bounds {
		if seg := boundLastSeg(ib.b); seg != nil && isIterTraitName(seg.Ident.Name) {
			e.iterish[ib.subject] = true
		}
	}
	if ia.Generics != nil {
		// I::Item: IntoIterator
		for _, wp := range ia.Generics.Where {
			if k := projKeyOf(wp.Ty, e.params); strings.HasSuffix(k, "::Item") {
				for _, b := range wp.Bounds {
					if seg := boundLastSeg(b); seg != nil && isIterTraitName(seg.Ident.Name) {
						e.iterish[k] = true
					}
				}
			}
		}
	}
	// the impl's self type laid over the iterator
	e.unify(ia.SelfTy, tl, func(k int) []int { return []int{k} })
	if e.bad {
		return nil
	}
	var out [][]int
	var done []bool
	for round, pass := 0, 0; ; round++ {
		for ; ; pass++ {
			if pass > 12 {
				return nil
			}
			e.changed = false
			for x := range e.subs {
				e.item(x)
			}
			for _, ib := range bounds {
				seg := boundLastSeg(ib.b)
				if seg == nil || seg.Args == nil {
					continue
				}
				if e.tys[ib.subject] == nil {
					continue
				}
				switch {
				case isIterTraitName(seg.Ident.Name) && !seg.Args.Parenthesized:
					// Y: Iterator<Item = Z>
					for _, bd := range seg.Args.Bindings {
						if bd.Name.Name != "Item" || bd.Ty == nil {
							continue
						}
						yb := e.item(ib.subject)
						if yb == nil {
							return nil
						}
						m := yb.m
						e.unify(bd.Ty, layoutOf(yb.ty), func(k int) []int { return m[k] })
					}
				case seg.Args.Parenthesized && seg.Args.Output != nil && (seg.Ident.Name == "Fn" || seg.Ident.Name == "FnMut" || seg.Ident.Name == "FnOnce"):
					e.applyClosure(e.tys[ib.subject], seg.Args.Inputs, seg.Args.Output)
				}
				if e.bad {
					return nil
				}
			}
			if !e.changed {
				break
			}
		}
		// (the items of parameters it needs are made here: then once
		// more with them)
		e.changed = false
		out, done = e.resolve(itemAst, il)
		if !e.changed || e.bad {
			break
		}
		if round > 4 {
			return nil
		}
	}
	if debugLoans {
		fmt.Fprintf(os.Stderr, "borrowck: implItems %v item %v: %v %v bad=%v lts=%v\n", t, itemTy, out, done, e.bad, e.lts)
	}
	if e.bad || !allTrue(done) {
		return nil
	}
	for k := range out {
		for _, s := range out[k] {
			if s < 0 || s >= tl.n() {
				return nil
			}
		}
	}
	return &itemMap{lay: il, m: out}
}

// applyClosure binds the names of output R of `F: FnMut(Args) -> R` from
// the summary of the closure F stands for (its result slots: the slots of
// its arguments, or of the closure value its captures sit in).
func (e *ienv) applyClosure(fb *ibind, inputs []ast.Type, output ast.Type) {
	if fb == nil || fb.ty == nil || fb.ty.Kind != TClosure {
		return
	}
	sm := closSummaries[fb.ty]
	if sm == nil || sm.ret == nil || sm.ret.flat || sm.imprecise || len(sm.params) != len(inputs) || len(sm.fromArg) != sm.ret.n() {
		return
	}
	for _, v := range sm.capVague {
		if v {
			return
		}
	}
	argSrc := make([][][]int, len(inputs))
	argDone := make([][]bool, len(inputs))
	for p, in := range inputs {
		if sm.params[p] == nil || sm.params[p].flat {
			return
		}
		argSrc[p], argDone[p] = e.resolve(in, sm.params[p])
	}
	srcs := make([][]int, sm.ret.n())
	for j := range srcs {
		for _, a := range sm.fromArg[j] {
			p, k := a[0], a[1]
			if p >= len(argSrc) || k >= len(argSrc[p]) || !argDone[p][k] {
				return
			}
			srcs[j], _ = unionInts(srcs[j], argSrc[p][k])
		}
		if j < len(sm.capSlots) {
			for _, d := range sm.capSlots[j] {
				if d >= len(fb.m) {
					return
				}
				srcs[j], _ = unionInts(srcs[j], fb.m[d])
			}
		}
	}
	e.unify(output, sm.ret, func(k int) []int { return srcs[k] })
}

// traitArgSlots relates, for `self: Trait<arg>` (FromIterator, Extend),
// each slot of self to the slots of arg the impl puts there (`impl<K, V,
// S> FromIterator<(K, V)> for HashMap<K, V, S>`). nil: unknown.
func traitArgSlots(tcx *typeck.Tcx, tr *Def, self, arg Ty, nest int) (res [][]int) {
	if tcx == nil || tr == nil || self == nil || arg == nil || nest > 4 {
		return nil
	}
	sl, al := layoutOf(self), layoutOf(arg)
	if sl.flat || al.flat {
		return nil
	}
	if sl.n() == 0 {
		return [][]int{}
	}
	defer func() {
		if recover() != nil {
			res = nil
		}
	}()
	impl, _ := tcx.SelectImplSimple(&TraitRef{Trait: tr, Args: []Ty{self, arg}})
	if impl == nil || impl.Impl == nil || impl.Impl.Ast == nil {
		return nil
	}
	ia := impl.Impl.Ast
	seg := boundLastSeg(&ast.Bound{Path: ia.Trait})
	if seg == nil || seg.Args == nil || seg.Args.Parenthesized || len(seg.Args.Bindings) > 0 {
		return nil
	}
	var targ ast.Type
	for _, ga := range seg.Args.Args {
		if ga.Type != nil {
			if targ != nil {
				return nil
			}
			targ = ga.Type
		}
	}
	if targ == nil {
		return nil
	}
	e := &ienv{tcx: tcx, params: map[string]bool{}, iterish: map[string]bool{}, nest: nest,
		lts: map[string][]int{}, tys: map[string]*ibind{}, subs: map[string]*itemMap{}, failed: map[string]bool{}}
	if ia.Generics != nil {
		for _, gp := range ia.Generics.Params {
			if gp.Kind == ast.GPType {
				e.params[gp.Name.Name] = true
			}
		}
	}
	e.unify(targ, al, func(k int) []int { return []int{k} })
	if e.bad {
		return nil
	}
	labels, occ, projs := labelAst(e.params, ia.SelfTy, sl)
	if len(projs) > 0 {
		return nil
	}
	res = make([][]int, sl.n())
	done := make([]bool, sl.n())
	for k, lb := range labels {
		if lb != "" && lb[0] == '\'' {
			// a lifetime the argument does not mention holds none of it
			res[k], done[k] = e.lts[lb], true
		}
	}
	for name, ns := range occ {
		b := e.tys[name]
		for _, n := range ns {
			if b == nil {
				// V: Trait<A> with A of the argument (Option<V> from
				// Option<A> items)
				b = e.nested(tr, impl, ia, name, n.ty)
			}
			if b == nil || !sameSlots(b.ty, n.ty) {
				return nil
			}
			for k := n.start; k < n.end; k++ {
				res[k], done[k] = b.m[k-n.start], true
			}
		}
	}
	if !allTrue(done) {
		return nil
	}
	return res
}

// nested binds parameter v of a self type through a bound `V: Trait<A>`
// (the same trait) with A bound by the trait argument.
func (e *ienv) nested(tr *Def, impl *Def, ia *ast.ImplItem, v string, vty Ty) *ibind {
	var found *ibind
	for _, ib := range implBounds(ia.Generics) {
		if ib.subject != v {
			continue
		}
		seg := boundLastSeg(ib.b)
		if seg == nil || seg.Ident.Name != tr.Name || seg.Args == nil || seg.Args.Parenthesized || len(seg.Args.Args) != 1 || seg.Args.Args[0].Type == nil {
			continue
		}
		pt, ok := seg.Args.Args[0].Type.(*ast.PathType)
		if !ok || pt.QSelf != nil || pt.Path == nil || len(pt.Path.Segments) != 1 || pt.Path.Segments[0].Args != nil {
			return nil
		}
		ab := e.tys[pt.Path.Segments[0].Ident.Name]
		if ab == nil || found != nil {
			return nil
		}
		m := traitArgSlots(e.tcx, tr, vty, ab.ty, e.nest+1)
		if m == nil {
			return nil
		}
		b := &ibind{ty: vty, m: make([][]int, len(m))}
		for k, ss := range m {
			for _, s := range ss {
				if s >= len(ab.m) {
					return nil
				}
				b.m[k], _ = unionInts(b.m[k], ab.m[s])
			}
		}
		found = b
	}
	return found
}

// genericBounds returns the bounds of single-name subject x in gens.
func genericBounds(x string, gens []*ast.Generics) []*ast.Bound {
	var out []*ast.Bound
	for _, g := range gens {
		for _, ib := range implBounds(g) {
			if ib.subject == x {
				out = append(out, ib.b)
			}
		}
	}
	return out
}

// iterParam reports whether `x::Item` in the signature of d is an
// iterator's item: x is Self of an iterator trait's method or a type
// parameter bounded by an iterator trait.
func iterParam(d *Def, x string, gens []*ast.Generics) bool {
	if x == "Self" {
		return d != nil && d.Parent != nil && d.Parent.Kind == DefTrait && isIterTraitName(d.Parent.Name)
	}
	for _, b := range genericBounds(x, gens) {
		if seg := boundLastSeg(b); seg != nil && isIterTraitName(seg.Ident.Name) {
			return true
		}
	}
	return false
}

// onlyClosureInputs reports whether the bounds of y mention x only in the
// inputs of closure bounds (`P: FnMut(&Self::Item) -> bool`: y does not
// produce x's items).
func onlyClosureInputs(y, x string, gens []*ast.Generics, params map[string]bool) bool {
	for _, b := range genericBounds(y, gens) {
		if b.Path == nil {
			continue
		}
		for i, seg := range b.Path.Segments {
			if seg.Args == nil {
				continue
			}
			if i != len(b.Path.Segments)-1 || !seg.Args.Parenthesized {
				m := map[string]bool{}
				genericArgNames(seg.Args, params, m)
				if m[x] {
					return false
				}
				continue
			}
			if seg.Args.Output != nil {
				m := map[string]bool{}
				typeNames(seg.Args.Output, params, m)
				if m[x] {
					return false
				}
			}
		}
	}
	return true
}

// itemBoundOf recognizes an output parameter p made from an iterator's
// items by a trait (`B: FromIterator<Self::Item>`): its only bound with
// arguments is `Trait<X::Item>`, or `Trait<A>` with A a parameter in the
// item type a bound declares (`FromA: Extend<A>` with `Self: Iterator<Item
// = (A, B)>` of unzip). It returns the trait, X and, in the second form,
// A and the declared item type.
func itemBoundOf(tcx *typeck.Tcx, d *Def, p string, gens []*ast.Generics, params map[string]bool) (*Def, string, string, ast.Type) {
	if tcx == nil || d == nil {
		return nil, "", "", nil
	}
	if _, producers := boundSources(p, gens, params); len(producers) > 0 {
		return nil, "", "", nil
	}
	var trName, x, a string
	var z ast.Type
	for _, b := range genericBounds(p, gens) {
		if b.Path == nil {
			continue
		}
		for i, seg := range b.Path.Segments {
			if seg.Args == nil {
				continue
			}
			if trName != "" || i != len(b.Path.Segments)-1 || seg.Args.Parenthesized || len(seg.Args.Bindings) > 0 || len(seg.Args.Args) != 1 || seg.Args.Args[0].Type == nil {
				return nil, "", "", nil
			}
			pt, ok := seg.Args.Args[0].Type.(*ast.PathType)
			if !ok || pt.QSelf != nil || pt.Path == nil {
				return nil, "", "", nil
			}
			ss := pt.Path.Segments
			switch {
			case len(ss) == 2 && ss[1].Ident.Name == "Item" && ss[0].Args == nil && ss[1].Args == nil:
				trName, x = seg.Ident.Name, ss[0].Ident.Name
			case len(ss) == 1 && ss[0].Args == nil && params[ss[0].Ident.Name] && ss[0].Ident.Name != "Self":
				// the parameter of an item type a bound declares
				a = ss[0].Ident.Name
				for _, g := range gens {
					for _, ib := range implBounds(g) {
						bs := boundLastSeg(ib.b)
						if bs == nil || !isIterTraitName(bs.Ident.Name) || bs.Args == nil || bs.Args.Parenthesized {
							continue
						}
						for _, bd := range bs.Args.Bindings {
							if bd.Name.Name != "Item" || bd.Ty == nil {
								continue
							}
							m := map[string]bool{}
							typeNames(bd.Ty, params, m)
							if m[a] {
								if z != nil {
									return nil, "", "", nil
								}
								x, z = ib.subject, bd.Ty
							}
						}
					}
				}
				if z == nil {
					return nil, "", "", nil
				}
				trName = seg.Ident.Name
			default:
				return nil, "", "", nil
			}
		}
	}
	if trName == "" || (x != "Self" && !params[x]) {
		return nil, "", "", nil
	}
	// other bounds mentioning p (`F: FnMut() -> p`, `Q: From<p>`)
	for _, g := range gens {
		if g == nil {
			continue
		}
		for _, gp := range g.Params {
			if gp.Kind == ast.GPType && gp.Name.Name != p && boundMentions(gp.Name.Name, p, gens, params) {
				return nil, "", "", nil
			}
		}
	}
	var tr *Def
	for _, pr := range tcx.Generics(d).Predicates {
		if pr == nil || pr.Trait == nil || len(pr.Trait.Args) != 2 || pr.Trait.Trait == nil || pr.Trait.Trait.Name != trName {
			continue
		}
		if a := pr.Trait.Args[0]; a != nil && a.Kind == TParam && a.Name == p {
			if tr != nil && tr != pr.Trait.Trait {
				return nil, "", "", nil
			}
			tr = pr.Trait.Trait
		}
	}
	if tr == nil {
		return nil, "", "", nil
	}
	return tr, x, a, z
}

// projKeyOf names a type parameter X or projection `X::Assoc` of params
// ("" for other types).
func projKeyOf(a ast.Type, params map[string]bool) string {
	pt, ok := a.(*ast.PathType)
	if !ok || pt.QSelf != nil || pt.Path == nil || len(pt.Path.Segments) == 0 || len(pt.Path.Segments) > 2 {
		return ""
	}
	for _, sg := range pt.Path.Segments {
		if sg.Args != nil {
			return ""
		}
	}
	segs := pt.Path.Segments
	if !params[segs[0].Ident.Name] {
		return ""
	}
	if len(segs) == 1 {
		return segs[0].Ident.Name
	}
	return segs[0].Ident.Name + "::" + segs[1].Ident.Name
}

// producesItems reports whether the bounds of y that mention x are all
// closure bounds making x's items from x's items (`F: FnMut(Self::Item,
// Self::Item) -> Self::Item`).
func producesItems(y, x string, gens []*ast.Generics) bool {
	isItem := func(a ast.Type) bool {
		pt, ok := a.(*ast.PathType)
		return ok && pt.QSelf == nil && pt.Path != nil && len(pt.Path.Segments) == 2 && pt.Path.Segments[0].Ident.Name == x && pt.Path.Segments[1].Ident.Name == "Item" && pt.Path.Segments[0].Args == nil && pt.Path.Segments[1].Args == nil
	}
	any := false
	for _, b := range genericBounds(y, gens) {
		seg := boundLastSeg(b)
		if seg == nil || seg.Args == nil {
			continue
		}
		if !seg.Args.Parenthesized || (seg.Ident.Name != "Fn" && seg.Ident.Name != "FnMut" && seg.Ident.Name != "FnOnce") || len(b.Path.Segments) != 1 || seg.Args.Output == nil || !isItem(seg.Args.Output) {
			return false
		}
		for _, in := range seg.Args.Inputs {
			if !isItem(in) {
				return false
			}
		}
		any = true
	}
	return any
}

// borrowPairs returns the bounds `A: Borrow<B>` between type parameters.
func borrowPairs(gens []*ast.Generics, params map[string]bool) [][2]string {
	var out [][2]string
	for _, g := range gens {
		for _, ib := range implBounds(g) {
			seg := boundLastSeg(ib.b)
			if seg == nil || seg.Ident.Name != "Borrow" || seg.Args == nil || seg.Args.Parenthesized || len(seg.Args.Args) != 1 || seg.Args.Args[0].Type == nil || !params[ib.subject] {
				continue
			}
			if k := projKeyOf(seg.Args.Args[0].Type, params); k != "" && !strings.Contains(k, "::") && k != ib.subject {
				out = append(out, [2]string{ib.subject, k})
			}
		}
	}
	return out
}

// refAbove returns the innermost reference node of the tree root that
// holds node n (nil: none, or n is not in it).
func refAbove(root, n *lnode) *lnode {
	var found *lnode
	var walk func(x, ref *lnode) bool
	walk = func(x, ref *lnode) bool {
		if x == n {
			found = ref
			return true
		}
		if x.ty != nil && x.ty.Kind == TRef {
			ref = x
		}
		for _, k := range x.kids {
			if walk(k, ref) {
				return true
			}
		}
		return false
	}
	walk(root, nil)
	return found
}

// itemStores relates, for an argument node s of iterator parameter x
// whose bound declares `Item = Z`, the slots of the items stored into
// node n (an instance of type parameter name in the referent of `&mut`
// argument di) through Z: slot by slot of name's occurrences in Z. nil:
// not known (the stores are then aligned by type).
func itemStores(tcx *typeck.Tcx, d *Def, x, name string, s *lnode, si int, n *lnode, di int, gens []*ast.Generics, params map[string]bool) []IntoSlot {
	var z ast.Type
	for _, b := range genericBounds(x, gens) {
		seg := boundLastSeg(b)
		if seg == nil || !isIterTraitName(seg.Ident.Name) || seg.Args == nil || seg.Args.Parenthesized {
			continue
		}
		for _, bd := range seg.Args.Bindings {
			if bd.Name.Name == "Item" && bd.Ty != nil {
				if z != nil {
					return nil
				}
				z = bd.Ty
			}
		}
	}
	if z == nil || layoutOf(s.ty).n() != s.end-s.start {
		return nil
	}
	im := iterItems(tcx, s.ty, 0)
	if im == nil {
		return nil
	}
	_, occ, projs := labelAst(params, z, im.lay)
	if len(projs) > 0 {
		return nil
	}
	out := []IntoSlot{}
	for _, nn := range occ[name] {
		if !sameSlots(nn.ty, n.ty) || nn.end-nn.start != n.end-n.start {
			return nil
		}
		for k := 0; k < n.end-n.start; k++ {
			for _, src := range im.m[nn.start+k] {
				out = append(out, IntoSlot{Dst: di, DstSlot: n.start + k, Src: si, SrcSlot: s.start + src, Items: true})
			}
		}
	}
	return out
}
