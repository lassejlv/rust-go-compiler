package borrowck

import (
	"sync"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

// Type aliases in signatures: `type Pair<'a> = (&'static str, &'a str);`
// written in a signature declares the regions of its definition. The
// signature walkers (sigWalker, labeler) read syntax, so a function's
// declaration is seen with such aliases expanded (lifetime parameters
// renamed to the arguments), as aliasTy follows them in field types.

var (
	sigDeclMu    sync.Mutex
	sigDeclCache = map[*Def]*ast.FnDecl{}
)

// sigDecl returns d's declaration with the type aliases its parameter and
// result types name expanded (the declaration itself when there are none).
func sigDecl(d *Def) *ast.FnDecl {
	decl := d.Fn.Ast.Decl
	if d.Scope == nil {
		return decl
	}
	sigDeclMu.Lock()
	defer sigDeclMu.Unlock()
	if c, ok := sigDeclCache[d]; ok {
		return c
	}
	params := map[string]bool{"Self": true}
	genericNames(d.Fn.Ast.Generics, params)
	if p := d.Parent; p != nil && p.Item != nil {
		switch k := p.Item.Kind.(type) {
		case *ast.ImplItem:
			genericNames(k.Generics, params)
		case *ast.TraitItem:
			genericNames(k.Generics, params)
		}
	}
	ex := &aliasExpander{params: params}
	nd := *decl
	nd.Params = make([]*ast.Param, len(decl.Params))
	for i, p := range decl.Params {
		if t := ex.expand(d.Scope, p.Ty, nil, 0); t != p.Ty {
			np := *p
			np.Ty = t
			nd.Params[i] = &np
		} else {
			nd.Params[i] = p
		}
	}
	nd.Output = ex.expand(d.Scope, decl.Output, nil, 0)
	if sp := decl.SelfParam; sp != nil && sp.Ty != nil {
		if t := ex.expand(d.Scope, sp.Ty, nil, 0); t != sp.Ty {
			ns := *sp
			ns.Ty = t
			nd.SelfParam = &ns
		}
	}
	out := decl
	if ex.changed {
		out = &nd
	}
	sigDeclCache[d] = out
	return out
}

type aliasExpander struct {
	params  map[string]bool // generic parameters (never aliases)
	changed bool
}

func renameLt(lt string, ren map[string]string) string {
	if r, ok := ren[lt]; ok {
		return r
	}
	return lt
}

// expand returns t with the aliases it names expanded (t itself when
// nothing changes), its lifetimes renamed by ren.
func (ex *aliasExpander) expand(scope *Module, t ast.Type, ren map[string]string, nest int) ast.Type {
	if t == nil || nest > 8 {
		return t
	}
	switch x := t.(type) {
	case *ast.PathType:
		if x.QSelf == nil && x.Path != nil && len(x.Path.Segments) == 1 && !ex.params[x.Path.Segments[0].Ident.Name] {
			if a, sc, r, ok := aliasTy(scope, x, ren); ok {
				ex.changed = true
				return ex.expand(sc, a, r, nest+1)
			}
		}
		if x.Path == nil {
			return t
		}
		var segs []*ast.PathSegment
		for i, seg := range x.Path.Segments {
			if na := ex.args(scope, seg.Args, ren, nest); na != seg.Args {
				if segs == nil {
					segs = append([]*ast.PathSegment{}, x.Path.Segments...)
				}
				segs[i] = &ast.PathSegment{Ident: seg.Ident, Args: na}
			}
		}
		var qs *ast.QSelf
		if x.QSelf != nil {
			if q := ex.expand(scope, x.QSelf.Ty, ren, nest+1); q != x.QSelf.Ty {
				qs = &ast.QSelf{Ty: q, Position: x.QSelf.Position}
			}
		}
		if segs == nil && qs == nil {
			return t
		}
		nx := *x
		if segs != nil {
			np := *x.Path
			np.Segments = segs
			nx.Path = &np
		}
		if qs != nil {
			nx.QSelf = qs
		}
		return &nx
	case *ast.RefType:
		e := ex.expand(scope, x.Elem, ren, nest+1)
		lt := renameLt(x.Lifetime, ren)
		if e == x.Elem && lt == x.Lifetime {
			return t
		}
		nx := *x
		nx.Elem, nx.Lifetime = e, lt
		return &nx
	case *ast.PtrType:
		if e := ex.expand(scope, x.Elem, ren, nest+1); e != x.Elem {
			nx := *x
			nx.Elem = e
			return &nx
		}
	case *ast.SliceType:
		if e := ex.expand(scope, x.Elem, ren, nest+1); e != x.Elem {
			nx := *x
			nx.Elem = e
			return &nx
		}
	case *ast.ArrayType:
		if e := ex.expand(scope, x.Elem, ren, nest+1); e != x.Elem {
			nx := *x
			nx.Elem = e
			return &nx
		}
	case *ast.ParenType:
		if e := ex.expand(scope, x.Elem, ren, nest+1); e != x.Elem {
			nx := *x
			nx.Elem = e
			return &nx
		}
	case *ast.TupleType:
		var es []ast.Type
		for i, e := range x.Elems {
			if ne := ex.expand(scope, e, ren, nest+1); ne != e {
				if es == nil {
					es = append([]ast.Type{}, x.Elems...)
				}
				es[i] = ne
			}
		}
		if es != nil {
			nx := *x
			nx.Elems = es
			return &nx
		}
	case *ast.DynTraitType:
		if bs := ex.bounds(scope, x.Bounds, ren, nest); bs != nil {
			nx := *x
			nx.Bounds = bs
			return &nx
		}
	case *ast.ImplTraitType:
		// (opaque types are identified by their syntax node: only
		// renamed lifetimes of an alias's definition change them)
		if ren != nil {
			if bs := ex.bounds(scope, x.Bounds, ren, nest); bs != nil {
				nx := *x
				nx.Bounds = bs
				return &nx
			}
		}
	}
	return t
}

func (ex *aliasExpander) bounds(scope *Module, bs []*ast.Bound, ren map[string]string, nest int) []*ast.Bound {
	var out []*ast.Bound
	for i, b := range bs {
		nb := b
		if lt := renameLt(b.Lifetime, ren); lt != b.Lifetime {
			c := *b
			c.Lifetime = lt
			nb = &c
		}
		if b.Path != nil {
			var segs []*ast.PathSegment
			for j, seg := range b.Path.Segments {
				if na := ex.args(scope, seg.Args, ren, nest); na != seg.Args {
					if segs == nil {
						segs = append([]*ast.PathSegment{}, b.Path.Segments...)
					}
					segs[j] = &ast.PathSegment{Ident: seg.Ident, Args: na}
				}
			}
			if segs != nil {
				c := *nb
				np := *b.Path
				np.Segments = segs
				c.Path = &np
				nb = &c
			}
		}
		if nb != b {
			if out == nil {
				out = append([]*ast.Bound{}, bs...)
			}
			out[i] = nb
		}
	}
	return out
}

func (ex *aliasExpander) args(scope *Module, ga *ast.GenericArgs, ren map[string]string, nest int) *ast.GenericArgs {
	if ga == nil {
		return nil
	}
	var n *ast.GenericArgs
	cp := func() {
		if n == nil {
			c := *ga
			c.Args = append([]ast.GenericArg{}, ga.Args...)
			c.Bindings = append([]*ast.AssocBinding{}, ga.Bindings...)
			c.Inputs = append([]ast.Type{}, ga.Inputs...)
			n = &c
		}
	}
	for i, a := range ga.Args {
		if a.Lifetime != "" {
			if lt := renameLt(a.Lifetime, ren); lt != a.Lifetime {
				cp()
				n.Args[i].Lifetime = lt
			}
		} else if a.Type != nil {
			if t := ex.expand(scope, a.Type, ren, nest+1); t != a.Type {
				cp()
				n.Args[i].Type = t
			}
		}
	}
	for i, bd := range ga.Bindings {
		if bd.Ty != nil {
			if t := ex.expand(scope, bd.Ty, ren, nest+1); t != bd.Ty {
				cp()
				nb := *bd
				nb.Ty = t
				n.Bindings[i] = &nb
			}
		}
	}
	for i, in := range ga.Inputs {
		if t := ex.expand(scope, in, ren, nest+1); t != in {
			cp()
			n.Inputs[i] = t
		}
	}
	if ga.Output != nil {
		if t := ex.expand(scope, ga.Output, ren, nest+1); t != ga.Output {
			cp()
			n.Output = t
		}
	}
	if n == nil {
		return ga
	}
	return n
}
