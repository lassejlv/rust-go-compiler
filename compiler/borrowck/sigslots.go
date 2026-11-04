package borrowck

import (
	"strconv"
	"strings"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

// Labels of the slots of a signature's types: the declared lifetime of
// each region ("'a", elided ones numbered like sigWalker does), or the
// type parameter whose instance holds it ("T#k": slot k of T's layout).
// "" marks a slot the signature does not describe (trait objects, `impl
// Trait`, associated types, mismatched syntax); proj records, for slots
// of an associated type `X::Assoc`, the parameter X.

type labeler struct {
	params     map[string]bool // type parameter names (incl. Self in traits)
	selfAst    ast.Type        // the impl's self type (nil in traits)
	inSelf     int
	selfElided int
	elided     int
	hidden     int
	out        bool
	labels     []string
	proj       map[int]string      // slot -> X of an `X::Assoc` there
	projAssoc  map[int]string      // slot -> Assoc of an `X::Assoc` there
	projAt     []*lnode            // the nodes of `X::Assoc` projections
	occ        map[string][]*lnode // type parameter (or "Self") -> its nodes
	full       bool                // every slot is labeled
	// assoc: the associated types the impl defines (`Self::Item` is
	// labeled by its definition, in terms of the impl's parameters)
	assoc   map[string]ast.Type
	inAssoc int
	// objects: label the slots of trait objects by their (default)
	// lifetime bound (objLt: the default of the enclosing type: the
	// label of the enclosing reference, "'static" in a type argument of
	// an ADT without lifetimes, "" unknown)
	objects bool
	objLt   string
}

func newLabeler(params map[string]bool, selfAst ast.Type) *labeler {
	return &labeler{params: params, selfAst: selfAst}
}

// label labels the slots of a value of layout l declared with type a.
func (w *labeler) label(a ast.Type, l *layout) {
	w.labels = make([]string, l.n())
	w.proj = map[int]string{}
	w.projAssoc = map[int]string{}
	w.projAt = nil
	w.occ = map[string][]*lnode{}
	if !l.flat {
		w.walk(a, l.root, 0)
	}
	w.full = true
	for _, s := range w.labels {
		if s == "" {
			w.full = false
		}
	}
}

func (w *labeler) lt(lt string) string {
	if (lt == "" || lt == "'_") && w.inSelf > 0 {
		w.selfElided++
		return "'Self#" + strconv.Itoa(w.selfElided)
	}
	if lt == "" || lt == "'_" {
		if w.out {
			return "'?"
		}
		w.elided++
		return "'#" + strconv.Itoa(w.elided)
	}
	return lt
}

// hiddenLt names an elided lifetime of a path without lifetime arguments
// (`Iter<T>` for Iter<'_, T>).
func (w *labeler) hiddenLt() string {
	if w.out {
		return "'?"
	}
	if w.inSelf > 0 {
		w.selfElided++
		return "'Self#" + strconv.Itoa(w.selfElided)
	}
	w.hidden++
	return "'#h" + strconv.Itoa(w.hidden)
}

func (w *labeler) param(name string, n *lnode) {
	w.occ[name] = append(w.occ[name], n)
	for k := n.start; k < n.end; k++ {
		w.labels[k] = name + "#" + strconv.Itoa(k-n.start)
	}
}

func (w *labeler) walk(a ast.Type, n *lnode, nest int) {
	if n == nil || a == nil || nest > 12 {
		return
	}
	if n.start == n.end {
		// a closure capturing nothing that holds loans: still the
		// instance of its parameter (its summary relates its result)
		if pt, ok := a.(*ast.PathType); ok && n.ty != nil && n.ty.Kind == TClosure && pt.QSelf == nil && pt.Path != nil && len(pt.Path.Segments) == 1 && pt.Path.Segments[0].Args == nil {
			if name := pt.Path.Segments[0].Ident.Name; name != "Self" && w.params[name] {
				w.occ[name] = append(w.occ[name], n)
			}
		}
		return
	}
	switch x := a.(type) {
	case *ast.ParenType:
		w.walk(x.Elem, n, nest)
		return
	case *ast.DynTraitType:
		if !w.objects || !w.out || n.ty == nil || n.ty.Kind != TDyn || !n.wild {
			return
		}
		lb := ""
		switch lt := dynLifetimeBound(x); {
		case lt == "'static":
			lb = lt
		case lt == "" && w.objLt == "'static":
			if objectDefaultStatic(n.ty) {
				lb = "'static"
			}
		case lt == "" && w.objLt != "":
			lb = w.objLt
		}
		if lb != "" {
			for k := n.start; k < n.end; k++ {
				w.labels[k] = lb
			}
		}
		return
	case *ast.PathType:
		if x.QSelf != nil && w.projAssoc != nil && x.Path != nil && x.QSelf.Position >= 1 && len(x.Path.Segments) == x.QSelf.Position+1 {
			// <I::Item as IntoIterator>::Item (recorded for items.go)
			tseg, aseg := x.Path.Segments[x.QSelf.Position-1], x.Path.Segments[x.QSelf.Position]
			if k := projKeyOf(x.QSelf.Ty, w.params); k != "" && aseg.Args == nil && isIterTraitName(tseg.Ident.Name) {
				w.projAt = append(w.projAt, n)
				for j := n.start; j < n.end; j++ {
					w.proj[j] = k
					w.projAssoc[j] = aseg.Ident.Name
				}
			}
			return
		}
		if x.QSelf != nil || x.Path == nil || len(x.Path.Segments) == 0 {
			return
		}
		segs := x.Path.Segments
		first := segs[0].Ident.Name
		if len(segs) == 1 && segs[0].Args == nil {
			if first == "Self" && w.selfAst != nil && w.inSelf < 2 {
				if w.inSelf == 0 {
					w.selfElided = 0
				}
				w.occ["Self"] = append(w.occ["Self"], n)
				w.inSelf++
				w.walk(w.selfAst, n, nest+1)
				w.inSelf--
				return
			}
			if w.params[first] {
				w.param(first, n)
				return
			}
		}
		if len(segs) == 2 && first == "Self" && segs[1].Args == nil && w.assocLabel(segs[1].Ident.Name, n, nest) {
			return
		}
		if len(segs) >= 2 && (w.params[first] || first == "Self") {
			// X::Assoc: built from what X holds
			if w.projAssoc != nil && len(segs) == 2 && segs[1].Args == nil {
				w.projAt = append(w.projAt, n)
			}
			for k := n.start; k < n.end; k++ {
				w.proj[k] = first
				if w.projAssoc != nil && len(segs) == 2 && segs[1].Args == nil {
					w.projAssoc[k] = segs[1].Ident.Name
				}
			}
			return
		}
		if n.ty == nil || n.ty.Kind != TAdt {
			return
		}
		seg := segs[len(segs)-1]
		if seg.Ident.Name != n.ty.Def.Name {
			return
		}
		var lts []string
		var tys []ast.Type
		if seg.Args != nil {
			if seg.Args.Parenthesized {
				return
			}
			for _, ga := range seg.Args.Args {
				if ga.Lifetime != "" {
					lts = append(lts, ga.Lifetime)
				} else if ga.Type != nil {
					tys = append(tys, ga.Type)
				}
			}
		}
		switch {
		case len(lts) == n.nlt:
			for i, l := range lts {
				w.labels[n.start+i] = w.lt(l)
			}
		case len(lts) == 0:
			for i := 0; i < n.nlt; i++ {
				w.labels[n.start+i] = w.hiddenLt()
			}
		}
		if len(tys) > len(n.kids) {
			return
		}
		saved := w.objLt
		w.objLt = ""
		if n.nlt == 0 {
			w.objLt = "'static"
		}
		for i, kid := range n.kids {
			if i < len(tys) {
				w.walk(tys[i], kid, nest+1)
			}
		}
		w.objLt = saved
	case *ast.RefType:
		if n.ty != nil && n.ty.Kind == TRef {
			w.labels[n.start] = w.lt(x.Lifetime)
			saved := w.objLt
			w.objLt = w.labels[n.start]
			w.walk(x.Elem, n.kids[0], nest+1)
			w.objLt = saved
		}
	case *ast.TupleType:
		if n.ty != nil && n.ty.Kind == TTuple && len(x.Elems) == len(n.kids) {
			for i, e := range x.Elems {
				w.walk(e, n.kids[i], nest+1)
			}
		}
	case *ast.SliceType:
		if n.ty != nil && n.ty.Kind == TSlice {
			w.walk(x.Elem, n.kids[0], nest+1)
		}
	case *ast.ArrayType:
		if n.ty != nil && n.ty.Kind == TArray {
			w.walk(x.Elem, n.kids[0], nest+1)
		}
	}
}

// assocLabel labels node n, declared as `Self::name`, by the impl's
// definition of that associated type. It only succeeds when every slot
// gets a declared lifetime or a type parameter of the impl (the
// parameters' instances then flow slot by slot).
func (w *labeler) assocLabel(name string, n *lnode, nest int) bool {
	t := w.assoc[name]
	if t == nil || w.inAssoc > 2 {
		return false
	}
	sub := &labeler{params: w.params, selfAst: w.selfAst, out: w.out, assoc: w.assoc, inAssoc: w.inAssoc + 1}
	sub.labels = make([]string, len(w.labels))
	sub.proj = map[int]string{}
	sub.occ = map[string][]*lnode{}
	sub.walk(t, n, nest+1)
	if len(sub.proj) > 0 {
		return false
	}
	// the names the impl's self type mentions (the values flowing in)
	sw := &sigWalker{params: w.params}
	sw.walk(w.selfAst, 0)
	own := map[string]bool{"'static": true}
	for _, o := range sw.occs {
		own[o.name] = true
	}
	for k := n.start; k < n.end; k++ {
		lb := sub.labels[k]
		if lb == "" || lb == "'?" || strings.HasPrefix(lb, "'#") || strings.HasPrefix(lb, "'Self#") {
			return false
		}
		if lb[0] != '\'' {
			lb = lb[:strings.IndexByte(lb, '#')]
		}
		if !own[lb] {
			return false
		}
	}
	copy(w.labels[n.start:n.end], sub.labels[n.start:n.end])
	for x, ns := range sub.occ {
		w.occ[x] = append(w.occ[x], ns...)
	}
	return true
}

// implAssocTypes returns the associated types a trait impl defines
// (without generics of their own).
func implAssocTypes(im *ast.ImplItem) map[string]ast.Type {
	if im == nil || im.Trait == nil {
		return nil
	}
	var m map[string]ast.Type
	for _, it := range im.Items {
		ta, ok := it.Kind.(*ast.TypeAliasItem)
		if !ok || ta.Ty == nil || (ta.Generics != nil && len(ta.Generics.Params) > 0) {
			continue
		}
		if m == nil {
			m = map[string]ast.Type{}
		}
		m[it.Name.Name] = ta.Ty
	}
	return m
}

// typeNames collects the type parameters (and Self) a type mentions, also
// as `X::Assoc`.
func typeNames(a ast.Type, params map[string]bool, out map[string]bool) {
	if a == nil {
		return
	}
	switch x := a.(type) {
	case *ast.ParenType:
		typeNames(x.Elem, params, out)
	case *ast.RefType:
		typeNames(x.Elem, params, out)
	case *ast.SliceType:
		typeNames(x.Elem, params, out)
	case *ast.ArrayType:
		typeNames(x.Elem, params, out)
	case *ast.TupleType:
		for _, e := range x.Elems {
			typeNames(e, params, out)
		}
	case *ast.PathType:
		if x.QSelf != nil {
			typeNames(x.QSelf.Ty, params, out)
			return
		}
		if x.Path == nil || len(x.Path.Segments) == 0 {
			return
		}
		first := x.Path.Segments[0].Ident.Name
		if params[first] || first == "Self" {
			out[first] = true
		}
		for _, seg := range x.Path.Segments {
			genericArgNames(seg.Args, params, out)
		}
	case *ast.ImplTraitType:
		for _, b := range x.Bounds {
			if b.Path != nil {
				for _, seg := range b.Path.Segments {
					genericArgNames(seg.Args, params, out)
				}
			}
		}
	case *ast.DynTraitType:
		for _, b := range x.Bounds {
			if b.Path != nil {
				for _, seg := range b.Path.Segments {
					genericArgNames(seg.Args, params, out)
				}
			}
		}
	}
}

func genericArgNames(ga *ast.GenericArgs, params map[string]bool, out map[string]bool) {
	if ga == nil {
		return
	}
	for _, a := range ga.Args {
		if a.Type != nil {
			typeNames(a.Type, params, out)
		}
	}
	for _, bd := range ga.Bindings {
		if bd.Ty != nil {
			typeNames(bd.Ty, params, out)
		}
	}
	for _, in := range ga.Inputs {
		typeNames(in, params, out)
	}
	if ga.Output != nil {
		typeNames(ga.Output, params, out)
	}
}

// boundSources returns, for a type parameter P the inputs do not mention,
// the type parameters its values are built from according to the bounds:
// `P: FromIterator<Self::Item>` (Self), `F: FnMut(I::Item) -> P` (I),
// `U: From<T>` (T).
func boundSources(p string, gens []*ast.Generics, params map[string]bool) (map[string]bool, map[string][]ast.Type) {
	out := map[string]bool{}
	producers := map[string][]ast.Type{} // F of `F: Fn(Inputs) -> P`
	visit := func(ty ast.Type, bounds []*ast.Bound) {
		subject := ""
		if pt, ok := ty.(*ast.PathType); ok && pt.QSelf == nil && pt.Path != nil && len(pt.Path.Segments) == 1 {
			subject = pt.Path.Segments[0].Ident.Name
		}
		for _, b := range bounds {
			if b.Path == nil {
				continue
			}
			for _, seg := range b.Path.Segments {
				ga := seg.Args
				if ga == nil {
					continue
				}
				if subject == p {
					// P: Trait<Args>
					genericArgNames(ga, params, out)
					continue
				}
				if ga.Parenthesized && ga.Output != nil {
					// F: Fn(Args) -> P
					m := map[string]bool{}
					typeNames(ga.Output, params, m)
					if m[p] {
						if subject != "" {
							producers[subject] = ga.Inputs
						} else {
							for _, in := range ga.Inputs {
								typeNames(in, params, out)
							}
						}
					}
				}
			}
		}
	}
	for _, g := range gens {
		if g == nil {
			continue
		}
		for _, gp := range g.Params {
			if gp.Kind == ast.GPType {
				visit(&ast.PathType{Path: &ast.Path{Segments: []*ast.PathSegment{{Ident: gp.Name}}}}, gp.Bounds)
			}
		}
		for _, wp := range g.Where {
			if wp.Ty != nil {
				visit(wp.Ty, wp.Bounds)
			}
		}
	}
	delete(out, p)
	return out, producers
}

// selfRefLifetime returns the lifetime of the reference to Self in a typed
// self parameter (`self: Pin<&mut Self>`), named like sigWalker names it
// (the first parameter: elided lifetimes are '#1, '#2, ...); "" if none.
func selfRefLifetime(t ast.Type) string {
	n := 0
	isSelf := func(t ast.Type) bool {
		pt, ok := t.(*ast.PathType)
		return ok && pt.QSelf == nil && pt.Path != nil && len(pt.Path.Segments) == 1 && pt.Path.Segments[0].Ident.Name == "Self"
	}
	var walk func(t ast.Type) string
	walk = func(t ast.Type) string {
		switch x := t.(type) {
		case *ast.ParenType:
			return walk(x.Elem)
		case *ast.RefType:
			name := x.Lifetime
			if name == "" || name == "'_" {
				n++
				name = "'#" + strconv.Itoa(n)
			}
			if isSelf(x.Elem) {
				return name
			}
			return walk(x.Elem)
		case *ast.PathType:
			if x.QSelf != nil || x.Path == nil || isSelf(x) {
				return ""
			}
			for _, seg := range x.Path.Segments {
				if seg.Args == nil {
					continue
				}
				for _, a := range seg.Args.Args {
					if a.Lifetime == "'_" {
						n++
					}
					if a.Type != nil {
						if r := walk(a.Type); r != "" {
							return r
						}
					}
				}
			}
		}
		return ""
	}
	return walk(t)
}
