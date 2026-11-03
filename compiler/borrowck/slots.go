package borrowck

import (
	"strconv"
	"strings"
	"sync"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
	"rustgo/compiler/typeck"
)

// Region slots: the loans a value holds are tracked per region of its type
// ("slot"), not merged by reference depth. A value of type T has one slot
// per reference in T and per lifetime parameter of each ADT in T (type
// arguments included), so reading `t.0` out of `(&b, &a)` or `self.a` out
// of `Pair<'a, 'b>` only yields the loans of that component. Each slot also
// has a reference depth (rustc's regions do not, but the signature flows of
// calls are described by depth where they cannot be related precisely):
// references and ADTs with lifetime parameters add a level, an ADT's type
// arguments sit below its lifetimes (Iter<'a, T> holds &'a T). A closure
// built in a checked body holds the slots of its captured operands (like a
// tuple, see registerClosureCaps). Types whose regions are unknown (type
// parameters, trait objects, other closures, opaque types) get one slot
// per depth ("wild" slots). Whenever two layouts cannot be related, loans
// are mapped by depth (the coarse model).

const maxSlots = 64

type lnode struct {
	ty         Ty
	start, end int // slots of this subtree in the root layout
	depth      int // depth of the node's own slots
	nlt        int // own slots: 1 for a reference, the lifetimes of an ADT
	kids       []*lnode
	wild       bool
}

type layout struct {
	root  *lnode
	depth []int
	key   string // equal keys: the same slots (values convert slot by slot)
	flat  bool   // no structure: one slot per depth
}

func (l *layout) n() int { return len(l.depth) }

var (
	layoutMu    sync.Mutex
	layoutCache = map[Ty]*layout{}
	flatLayout  = func() *layout {
		l := &layout{key: "#flat", flat: true}
		for d := 0; d < maxDepth; d++ {
			l.depth = append(l.depth, d)
		}
		l.root = &lnode{start: 0, end: maxDepth, wild: true}
		return l
	}()
	ltCountCache = map[*Def][]string{}
	// closureCaps: the types of the values closures of the bodies built
	// so far capture; such a closure's slots are those of its captures
	// (like a tuple's), others' are wild
	closureCaps = map[Ty][]Ty{}
)

// registerClosureCaps records the captured operand types of closure type t
// (unless its layout is in use already).
func registerClosureCaps(t Ty, caps []Ty) {
	if t == nil || t.Kind != TClosure {
		return
	}
	layoutMu.Lock()
	defer layoutMu.Unlock()
	if _, ok := layoutCache[t]; ok {
		return
	}
	if _, ok := closureCaps[t]; ok {
		return
	}
	for _, c := range caps {
		if c == nil {
			return
		}
	}
	closureCaps[t] = caps
}

// capturesOf returns the captured operand types of a closure type with a
// structured layout (ok false: wild).
func capturesOf(t Ty) ([]Ty, bool) {
	if t == nil || t.Kind != TClosure {
		return nil, false
	}
	c, ok := closureCaps[t]
	return c, ok
}

// adtLifetimes returns the names of an ADT's lifetime parameters.
func adtLifetimes(d *Def) []string {
	if d == nil || d.Item == nil {
		return nil
	}
	if l, ok := ltCountCache[d]; ok {
		return l
	}
	var g *ast.Generics
	switch k := d.Item.Kind.(type) {
	case *ast.StructItem:
		g = k.Generics
	case *ast.EnumItem:
		g = k.Generics
	case *ast.UnionItem:
		g = k.Generics
	}
	var out []string
	if g != nil {
		for _, p := range g.Params {
			if p.Kind == ast.GPLifetime {
				out = append(out, p.Name.Name)
			}
		}
	}
	ltCountCache[d] = out
	return out
}

func isWildKind(t Ty) bool {
	if t == nil {
		return true
	}
	switch t.Kind {
	case TDyn, TClosure, TOpaque, TParam, TProjection, TInfer, TError:
		return true
	}
	return false
}

// layoutOf returns the slot layout of values of type t (nil: unknown).
func layoutOf(t Ty) *layout {
	if t == nil {
		return flatLayout
	}
	layoutMu.Lock()
	defer layoutMu.Unlock()
	if l, ok := layoutCache[t]; ok {
		return l
	}
	l := &layout{}
	l.root = l.walk(t, 0, 0)
	if len(l.depth) > maxSlots {
		fl := &layout{key: "#flat:" + t.String(), flat: true}
		for d := 0; d < maxDepth; d++ {
			fl.depth = append(fl.depth, d)
		}
		fl.root = &lnode{ty: t, start: 0, end: maxDepth, wild: true}
		l = fl
	} else {
		l.key = slotKey(t, 0)
	}
	layoutCache[t] = l
	return l
}

func (l *layout) walk(t Ty, d, nest int) *lnode {
	n := &lnode{ty: t, start: len(l.depth), depth: d}
	if len(l.depth) > maxSlots || nest > 12 {
		n.end = len(l.depth)
		return n
	}
	if caps, ok := capturesOf(t); ok {
		for _, c := range caps {
			n.kids = append(n.kids, l.walk(c, d, nest+1))
		}
		n.end = len(l.depth)
		return n
	}
	if isWildKind(t) {
		n.wild = true
		for k := 0; k < maxDepth; k++ {
			l.depth = append(l.depth, d+k)
		}
		n.end = len(l.depth)
		return n
	}
	switch t.Kind {
	case TRef:
		n.nlt = 1
		l.depth = append(l.depth, d)
		n.kids = []*lnode{l.walk(t.Elem, d+1, nest+1)}
	case TTuple:
		for _, e := range t.Elems {
			n.kids = append(n.kids, l.walk(e, d, nest+1))
		}
	case TArray, TSlice:
		n.kids = []*lnode{l.walk(t.Elem, d, nest+1)}
	case TAdt:
		n.nlt = len(adtLifetimes(t.Def))
		for k := 0; k < n.nlt; k++ {
			l.depth = append(l.depth, d)
		}
		inner := d
		if n.nlt > 0 {
			inner = d + 1
		}
		for _, a := range t.Args {
			n.kids = append(n.kids, l.walk(a, inner, nest+1))
		}
	}
	n.end = len(l.depth)
	return n
}

// slotKey describes the slot structure of t ("" when it has no slots).
func slotKey(t Ty, nest int) string {
	if nest > 12 {
		return "!"
	}
	if caps, ok := capturesOf(t); ok {
		var parts []string
		any := false
		for _, c := range caps {
			k := slotKey(c, nest+1)
			if k != "" {
				any = true
			}
			parts = append(parts, k)
		}
		if !any {
			return ""
		}
		return "C(" + strings.Join(parts, ",") + ")"
	}
	if isWildKind(t) {
		return "?"
	}
	switch t.Kind {
	case TRef:
		return "&" + slotKey(t.Elem, nest+1)
	case TTuple:
		var parts []string
		any := false
		for _, e := range t.Elems {
			k := slotKey(e, nest+1)
			if k != "" {
				any = true
			}
			parts = append(parts, k)
		}
		if !any {
			return ""
		}
		return "(" + strings.Join(parts, ",") + ")"
	case TArray, TSlice:
		return slotKey(t.Elem, nest+1)
	case TAdt:
		nlt := len(adtLifetimes(t.Def))
		var parts []string
		any := nlt > 0
		for _, a := range t.Args {
			k := slotKey(a, nest+1)
			if k != "" {
				any = true
			}
			parts = append(parts, k)
		}
		if !any {
			return ""
		}
		return "A" + strconv.Itoa(t.Def.ID) + "<" + strconv.Itoa(nlt) + ";" + strings.Join(parts, ",") + ">"
	}
	return ""
}

// collapseMap maps each slot of `to` to the slots of `from` at its depth
// plus shift (the coarse, depth-based model).
func collapseMap(from, to *layout, shift int) [][]int {
	m := make([][]int, to.n())
	for j, dj := range to.depth {
		for i, di := range from.depth {
			if di == dj+shift {
				m[j] = append(m[j], i)
			}
		}
	}
	return m
}

func rangeMap(start, n int) [][]int {
	m := make([][]int, n)
	for j := range m {
		m[j] = []int{start + j}
	}
	return m
}

// ---------------------------------------------------------------------------
// Fields of ADTs

type fieldKey struct {
	t        Ty
	variant  int
	field    int
	haveTcx  bool
	normalTy bool
}

type fieldInfo struct {
	ty Ty
	m  [][]int // field slot -> slots of the ADT value
}

var fieldCache = map[fieldKey]*fieldInfo{}

// adtField returns the type of a field of an ADT value and how its slots
// map to the ADT's (nil when unknown).
func adtField(tcx *typeck.Tcx, t Ty, variant, field int) *fieldInfo {
	if t == nil || t.Kind != TAdt || tcx == nil {
		return nil
	}
	k := fieldKey{t: t, variant: variant, field: field}
	if fi, ok := fieldCache[k]; ok {
		return fi
	}
	fi := computeAdtField(tcx, t, variant, field)
	fieldCache[k] = fi
	return fi
}

func computeAdtField(tcx *typeck.Tcx, t Ty, variant, field int) (fi *fieldInfo) {
	defer func() {
		if recover() != nil {
			fi = nil
		}
	}()
	d := t.Def
	tcx.AdtFields(d)
	if d.Adt == nil || variant >= len(d.Adt.Variants) {
		return nil
	}
	v := d.Adt.Variants[variant]
	if field >= len(v.Fields) {
		return nil
	}
	f := v.Fields[field]
	if f.Ty == nil {
		return nil
	}
	ft := tcx.Normalize(Subst(f.Ty, t.Args))
	fi = &fieldInfo{ty: ft}
	pl := layoutOf(t)
	cl := layoutOf(ft)
	if pl.flat || cl.flat {
		fi.m = collapseMap(pl, cl, 0)
		return fi
	}
	lts := adtLifetimes(d)
	ltIdx := map[string]int{}
	for i, n := range lts {
		ltIdx[n] = pl.root.start + i
	}
	allLts := make([]int, len(lts))
	for i := range lts {
		allLts[i] = pl.root.start + i
	}
	var astTy ast.Type
	if f.Ast != nil {
		astTy = f.Ast.Ty
	}
	fw := &fieldWalker{p: pl, ltIdx: ltIdx, allLts: allLts, scope: d.Scope}
	fw.walk(astTy, f.Ty, 0)
	if fw.bad || len(fw.m) != cl.n() {
		fi.m = collapseMap(pl, cl, 0)
		return fi
	}
	fi.m = fw.m
	return fi
}

type fieldWalker struct {
	p      *layout
	ltIdx  map[string]int
	allLts []int
	m      [][]int
	bad    bool
	scope  *Module           // where the field's type names resolve
	ren    map[string]string // lifetime parameters of a type alias -> arguments
}

// aliasTy returns the type a type alias named by a stands for (`type Row
// = &'static str;`), with the module its names resolve in and its
// lifetime parameters' arguments (ren, given the renaming in effect
// where a is written). Only single-segment paths to aliases without type
// parameters are followed.
func aliasTy(scope *Module, a ast.Type, outer map[string]string) (ast.Type, *Module, map[string]string, bool) {
	pt, ok := a.(*ast.PathType)
	if !ok || scope == nil || pt.QSelf != nil || pt.Path == nil || len(pt.Path.Segments) != 1 {
		return nil, nil, nil, false
	}
	seg := pt.Path.Segments[0]
	name := seg.Ident.Name
	if name == "Self" {
		return nil, nil, nil, false
	}
	// names of the enclosing blocks, then of the module
	for m, n := scope, 0; m != nil && n < 64; m, n = m.Parent, n+1 {
		b := m.Lookup(name, TypeNS)
		if b == nil {
			if !m.Block {
				break
			}
			continue
		}
		d := b.Res.Def
		if d == nil || d.Kind != DefTyAlias || d.Alias == nil || d.Alias.TyAst == nil || d.Item == nil {
			return nil, nil, nil, false
		}
		ta, ok := d.Item.Kind.(*ast.TypeAliasItem)
		if !ok {
			return nil, nil, nil, false
		}
		var ps []string
		if ta.Generics != nil {
			for _, gp := range ta.Generics.Params {
				if gp.Kind != ast.GPLifetime {
					return nil, nil, nil, false
				}
				ps = append(ps, gp.Name.Name)
			}
		}
		var args []string
		if seg.Args != nil {
			if seg.Args.Parenthesized || len(seg.Args.Bindings) > 0 {
				return nil, nil, nil, false
			}
			for _, ga := range seg.Args.Args {
				if ga.Lifetime == "" || ga.Lifetime == "'_" {
					return nil, nil, nil, false
				}
				args = append(args, ga.Lifetime)
			}
		}
		if len(args) != len(ps) {
			return nil, nil, nil, false
		}
		var ren map[string]string
		if len(ps) > 0 {
			ren = map[string]string{}
			for i, p := range ps {
				x := args[i]
				if r, ok := outer[x]; ok {
					x = r
				}
				ren[p] = x
			}
		}
		return d.Alias.TyAst, d.Scope, ren, true
	}
	return nil, nil, nil, false
}

func (w *fieldWalker) lt(name string) []int {
	if r, ok := w.ren[name]; ok {
		name = r
	}
	if name == "'static" {
		return nil
	}
	if i, ok := w.ltIdx[name]; ok {
		return []int{i}
	}
	return w.allLts
}

// walk follows the field's declared type g (in terms of the ADT's generic
// parameters), with its syntax a when it still matches.
func (w *fieldWalker) walk(a ast.Type, g Ty, nest int) {
	if w.bad || nest > 12 {
		w.bad = true
		return
	}
	for {
		p, ok := a.(*ast.ParenType)
		if !ok {
			break
		}
		a = p.Elem
	}
	if g == nil {
		w.bad = true
		return
	}
	if t, sc, ren, ok := aliasTy(w.scope, a, w.ren); ok && g.Kind != TParam {
		oldScope, oldRen := w.scope, w.ren
		w.scope, w.ren = sc, ren
		w.walk(t, g, nest+1)
		w.scope, w.ren = oldScope, oldRen
		return
	}
	if g.Kind == TParam {
		root := w.p.root
		if g.Index < len(root.kids) {
			kid := root.kids[g.Index]
			w.m = append(w.m, rangeMap(kid.start, kid.end-kid.start)...)
			return
		}
		w.bad = true
		return
	}
	if isWildKind(g) {
		// associated types, trait objects: anything the ADT holds
		all := make([]int, 0, w.p.n())
		for i := 0; i < w.p.n(); i++ {
			all = append(all, i)
		}
		if g.Kind == TProjection {
			w.bad = true // its normalized layout differs
			return
		}
		for k := 0; k < maxDepth; k++ {
			w.m = append(w.m, all)
		}
		return
	}
	switch g.Kind {
	case TRef:
		var elem ast.Type
		if r, ok := a.(*ast.RefType); ok {
			w.m = append(w.m, w.lt(r.Lifetime))
			elem = r.Elem
		} else {
			w.m = append(w.m, w.allLts)
		}
		w.walk(elem, g.Elem, nest+1)
	case TTuple:
		tt, _ := a.(*ast.TupleType)
		for i, e := range g.Elems {
			var ea ast.Type
			if tt != nil && len(tt.Elems) == len(g.Elems) {
				ea = tt.Elems[i]
			}
			w.walk(ea, e, nest+1)
		}
	case TArray:
		var ea ast.Type
		if at, ok := a.(*ast.ArrayType); ok {
			ea = at.Elem
		}
		w.walk(ea, g.Elem, nest+1)
	case TSlice:
		var ea ast.Type
		if at, ok := a.(*ast.SliceType); ok {
			ea = at.Elem
		}
		w.walk(ea, g.Elem, nest+1)
	case TAdt:
		lts := adtLifetimes(g.Def)
		var ltArgs []string
		var tyArgs []ast.Type
		if pt, ok := a.(*ast.PathType); ok && pt.QSelf == nil && pt.Path != nil && len(pt.Path.Segments) > 0 {
			seg := pt.Path.Segments[len(pt.Path.Segments)-1]
			if seg.Ident.Name == g.Def.Name && seg.Args != nil {
				for _, ga := range seg.Args.Args {
					if ga.Lifetime != "" {
						ltArgs = append(ltArgs, ga.Lifetime)
					} else if ga.Type != nil {
						tyArgs = append(tyArgs, ga.Type)
					}
				}
			}
			if seg.Ident.Name != g.Def.Name {
				ltArgs, tyArgs = nil, nil
			}
		}
		for i := range lts {
			if len(ltArgs) == len(lts) {
				w.m = append(w.m, w.lt(ltArgs[i]))
			} else {
				w.m = append(w.m, w.allLts)
			}
		}
		for i, ga := range g.Args {
			var ea ast.Type
			if i < len(tyArgs) && len(tyArgs) <= len(g.Args) {
				ea = tyArgs[i]
			}
			w.walk(ea, ga, nest+1)
		}
	}
}

// ---------------------------------------------------------------------------
// Implied bounds of ADTs

var adtOutlivesCache = map[*Def][][2]string{}

// adtOutlives returns the outlives relations between an ADT's lifetime
// parameters (and 'static) that its well-formedness implies (pairs {x,
// y}: x: y): its declared bounds and those its fields' types imply
// (`&'b mut Vec<&'a str>` implies 'a: 'b), also through the ADTs they
// mention.
func adtOutlives(tcx *typeck.Tcx, d *Def) (out [][2]string) {
	if d == nil || d.Item == nil || tcx == nil {
		return nil
	}
	if r, ok := adtOutlivesCache[d]; ok {
		return r
	}
	adtOutlivesCache[d] = nil // cycles: nothing more
	defer func() {
		if recover() != nil {
			out = nil
		}
		adtOutlivesCache[d] = out
	}()
	lts := adtLifetimes(d)
	if len(lts) == 0 {
		return nil
	}
	own := map[string]bool{"'static": true}
	for _, n := range lts {
		own[n] = true
	}
	seen := map[[2]string]bool{}
	add := func(x, y string) {
		k := [2]string{x, y}
		if x != y && own[x] && own[y] && !seen[k] {
			seen[k] = true
			out = append(out, k)
		}
	}
	var g *ast.Generics
	switch k := d.Item.Kind.(type) {
	case *ast.StructItem:
		g = k.Generics
	case *ast.EnumItem:
		g = k.Generics
	case *ast.UnionItem:
		g = k.Generics
	}
	if g != nil {
		for _, gp := range g.Params {
			if gp.Kind == ast.GPLifetime {
				for _, bd := range gp.Bounds {
					add(gp.Name.Name, bd.Lifetime)
				}
			}
		}
		for _, wp := range g.Where {
			if wp.Lifetime != "" {
				for _, bd := range wp.Bounds {
					add(wp.Lifetime, bd.Lifetime)
				}
			}
		}
	}
	tcx.AdtFields(d)
	if d.Adt == nil {
		return out
	}
	iw := &impliedWalker{tcx: tcx, add: add, scope: d.Scope}
	for _, v := range d.Adt.Variants {
		for _, f := range v.Fields {
			if f.Ast != nil && f.Ty != nil {
				iw.walk(f.Ast.Ty, f.Ty, 0)
			}
		}
	}
	return out
}

// impliedWalker collects the outlives relations a type implies: each
// lifetime outlives the references and ADT lifetimes it is nested in. The
// syntax a is followed while it matches the type g.
type impliedWalker struct {
	tcx   *typeck.Tcx
	add   func(x, y string)
	outer []string
	scope *Module
	ren   map[string]string
}

func (w *impliedWalker) name(lt string) string {
	if r, ok := w.ren[lt]; ok {
		return r
	}
	return lt
}

func (w *impliedWalker) walk(a ast.Type, g Ty, nest int) {
	if a == nil || g == nil || nest > 12 {
		return
	}
	for {
		p, ok := a.(*ast.ParenType)
		if !ok {
			break
		}
		a = p.Elem
	}
	if t, sc, ren, ok := aliasTy(w.scope, a, w.ren); ok {
		oldScope, oldRen := w.scope, w.ren
		w.scope, w.ren = sc, ren
		w.walk(t, g, nest+1)
		w.scope, w.ren = oldScope, oldRen
		return
	}
	nested := func(lts []string, kids func()) {
		for _, x := range lts {
			for _, o := range w.outer {
				w.add(x, o)
			}
		}
		n := len(w.outer)
		w.outer = append(w.outer, lts...)
		kids()
		w.outer = w.outer[:n]
	}
	switch g.Kind {
	case TRef:
		r, ok := a.(*ast.RefType)
		if !ok || r.Lifetime == "" || r.Lifetime == "'_" {
			return
		}
		nested([]string{w.name(r.Lifetime)}, func() { w.walk(r.Elem, g.Elem, nest+1) })
	case TTuple:
		if tt, ok := a.(*ast.TupleType); ok && len(tt.Elems) == len(g.Elems) {
			for i, e := range g.Elems {
				w.walk(tt.Elems[i], e, nest+1)
			}
		}
	case TArray:
		if at, ok := a.(*ast.ArrayType); ok {
			w.walk(at.Elem, g.Elem, nest+1)
		}
	case TSlice:
		if at, ok := a.(*ast.SliceType); ok {
			w.walk(at.Elem, g.Elem, nest+1)
		}
	case TAdt:
		pt, ok := a.(*ast.PathType)
		if !ok || pt.QSelf != nil || pt.Path == nil || len(pt.Path.Segments) == 0 {
			return
		}
		seg := pt.Path.Segments[len(pt.Path.Segments)-1]
		if seg.Ident.Name != g.Def.Name || seg.Args == nil || seg.Args.Parenthesized {
			return
		}
		var ltArgs []string
		var tyArgs []ast.Type
		for _, ga := range seg.Args.Args {
			if ga.Lifetime != "" {
				ltArgs = append(ltArgs, ga.Lifetime)
			} else if ga.Type != nil {
				tyArgs = append(tyArgs, ga.Type)
			}
		}
		params := adtLifetimes(g.Def)
		if len(ltArgs) != len(params) {
			ltArgs = nil
		}
		sub := map[string]string{"'static": "'static"}
		for i, p := range params {
			if ltArgs != nil && ltArgs[i] != "'_" {
				sub[p] = w.name(ltArgs[i])
			}
		}
		// the nested ADT's own implied bounds, in terms of its arguments
		for _, pr := range adtOutlives(w.tcx, g.Def) {
			x, okx := sub[pr[0]]
			y, oky := sub[pr[1]]
			if okx && oky {
				w.add(x, y)
			}
		}
		var lts []string
		for _, p := range params {
			if x, ok := sub[p]; ok {
				lts = append(lts, x)
			}
		}
		nested(lts, func() {
			if len(tyArgs) > len(g.Args) {
				return
			}
			for i, ta := range tyArgs { // defaults: not written
				w.walk(ta, g.Args[i], nest+1)
			}
		})
	}
}

// impliedByLabels adds to r the outlives relations the ADTs in a value of
// layout l imply, given the labels of its slots.
func impliedByLabels(tcx *typeck.Tcx, r *sigRegions, l *layout, labels []string) {
	if l == nil || l.flat {
		return
	}
	var walk func(n *lnode)
	walk = func(n *lnode) {
		if n == nil {
			return
		}
		if n.ty != nil && n.ty.Kind == TAdt && n.nlt > 0 {
			params := adtLifetimes(n.ty.Def)
			sub := map[string]string{"'static": "'static"}
			for i, p := range params {
				if k := n.start + i; i < n.nlt && k < len(labels) && len(labels[k]) > 1 && labels[k][0] == '\'' && labels[k] != "'?" {
					sub[p] = labels[k]
				}
			}
			for _, pr := range adtOutlives(tcx, n.ty.Def) {
				x, okx := sub[pr[0]]
				y, oky := sub[pr[1]]
				if okx && oky && x != y {
					r.addOutlives(x, y)
				}
			}
		}
		for _, k := range n.kids {
			walk(k)
		}
	}
	walk(l.root)
}
