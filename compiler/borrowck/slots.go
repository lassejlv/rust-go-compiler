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
// arguments sit below its lifetimes (Iter<'a, T> holds &'a T). Types whose
// regions are unknown (type parameters, trait objects, closures, opaque
// types) get one slot per depth ("wild" slots). Whenever two layouts cannot
// be related, loans are mapped by depth (the coarse model).

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
)

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
	fw := &fieldWalker{p: pl, ltIdx: ltIdx, allLts: allLts}
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
}

func (w *fieldWalker) lt(name string) []int {
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
