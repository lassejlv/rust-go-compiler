package borrowck

import (
	"fmt"
	"os"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
)

// Universal regions: the lifetimes of a function's signature. Each
// reference-typed parameter starts out holding an "origin" pseudo-loan per
// lifetime of its declared type; a value returned (or stored through a
// `&mut` parameter) may only hold origins that outlive the lifetimes the
// signature declares at that position.

// sigRegions describes a function's own signature.
type sigRegions struct {
	params  [][]occ // lifetime/type-param occurrences per parameter (self first)
	out     []occ   // in the output
	elided  string  // the lifetime elided output lifetimes refer to
	outlive map[string]map[string]bool
	names   map[string]string // origin name -> parameter name (diagnostics)
	// the declared types, for labeling slots (see sigslots.go)
	inAst   []ast.Type
	outAst  ast.Type
	tparams map[string]bool
	selfAst ast.Type
	nElided int // elided input lifetimes ('#n)
}

func (r *sigRegions) outlives(a, b string) bool {
	if a == b || a == "'static" {
		return true
	}
	seen := map[string]bool{}
	var walk func(x string) bool
	walk = func(x string) bool {
		if x == b {
			return true
		}
		if seen[x] {
			return false
		}
		seen[x] = true
		for y := range r.outlive[x] {
			if walk(y) {
				return true
			}
		}
		return false
	}
	return walk(a)
}

func (r *sigRegions) addOutlives(a, b string) {
	if r.outlive[a] == nil {
		r.outlive[a] = map[string]bool{}
	}
	r.outlive[a][b] = true
}

// regionWalker collects lifetime occurrences with the implied bounds of
// nested references (`&'a &'b T` implies 'b: 'a).
type regionWalker struct {
	sigWalker
	r     *sigRegions
	outer []string
}

func (w *regionWalker) walkR(t ast.Type, depth int) {
	if t == nil || depth > 6 {
		return
	}
	switch x := t.(type) {
	case *ast.RefType:
		lt := w.ltName(x.Lifetime, depth)
		for _, o := range w.outer {
			if lt != "" {
				w.r.addOutlives(lt, o)
			}
		}
		if lt != "" {
			w.outer = append(w.outer, lt)
		}
		w.walkR(x.Elem, depth+1)
		if lt != "" {
			w.outer = w.outer[:len(w.outer)-1]
		}
	case *ast.ParenType:
		w.walkR(x.Elem, depth)
	case *ast.SliceType:
		w.walkR(x.Elem, depth)
	case *ast.ArrayType:
		w.walkR(x.Elem, depth)
	case *ast.TupleType:
		for _, e := range x.Elems {
			w.walkR(e, depth)
		}
	case *ast.PathType:
		if x.QSelf != nil {
			w.add("?", depth)
			return
		}
		p := x.Path
		if p == nil || len(p.Segments) == 0 {
			return
		}
		first := p.Segments[0].Ident.Name
		if first == "Self" && w.selfTy != nil && w.inSelf < 2 && len(p.Segments) == 1 {
			if w.inSelf == 0 {
				w.selfElided = 0
			}
			w.inSelf++
			w.walkR(w.selfTy, depth)
			w.inSelf--
			return
		}
		if w.params[first] {
			w.add("?", depth) // a type parameter: any lifetime
			return
		}
		for _, seg := range p.Segments {
			if seg.Args == nil {
				continue
			}
			td := depth
			var lts []string
			for _, a := range seg.Args.Args {
				if a.Lifetime != "" {
					td = depth + 1
				}
			}
			for _, a := range seg.Args.Args {
				if a.Lifetime != "" {
					if lt := w.ltName(a.Lifetime, depth); lt != "" {
						for _, o := range w.outer {
							w.r.addOutlives(lt, o)
						}
						lts = append(lts, lt)
					}
				}
			}
			w.outer = append(w.outer, lts...)
			for _, a := range seg.Args.Args {
				if a.Type != nil {
					w.walkR(a.Type, td)
				}
			}
			w.outer = w.outer[:len(w.outer)-len(lts)]
			for _, bd := range seg.Args.Bindings {
				if bd.Ty != nil {
					w.walkR(bd.Ty, depth)
				}
			}
		}
	default:
		// dyn/impl Trait, fn pointers, ...: any lifetime
		w.add("?", depth)
	}
}

func (w *regionWalker) ltName(lt string, depth int) string {
	if (lt == "" || lt == "'_") && w.inSelf > 0 {
		w.selfElided++
		n := fmt.Sprintf("'Self#%d", w.selfElided)
		w.add(n, depth)
		return n
	}
	if lt == "" || lt == "'_" {
		if w.inOut {
			w.add("'?", depth)
			return ""
		}
		w.elided++
		n := fmt.Sprintf("'#%d", w.elided)
		w.add(n, depth)
		return n
	}
	w.add(lt, depth)
	return lt
}

// fnRegions analyzes the signature of the function being checked.
func fnRegions(d *Def) *sigRegions {
	if d == nil || d.Fn == nil || d.Fn.Ast == nil || d.Fn.Ast.Decl == nil {
		return nil
	}
	decl := d.Fn.Ast.Decl
	r := &sigRegions{outlive: map[string]map[string]bool{}, names: map[string]string{}}
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
			return nil // trait default bodies: Self is unknown
		}
	}
	for _, g := range gens {
		if g == nil {
			continue
		}
		for _, gp := range g.Params {
			if gp.Kind == ast.GPLifetime {
				for _, bd := range gp.Bounds {
					if bd.Lifetime != "" {
						r.addOutlives(gp.Name.Name, bd.Lifetime)
					}
				}
			}
		}
		for _, wp := range g.Where {
			if wp.Lifetime != "" {
				for _, bd := range wp.Bounds {
					if bd.Lifetime != "" {
						r.addOutlives(wp.Lifetime, bd.Lifetime)
					}
				}
			}
		}
	}
	w := &regionWalker{sigWalker: sigWalker{params: params, selfTy: selfTy}, r: r}
	r.tparams = params
	r.selfAst = selfTy
	r.outAst = decl.Output
	selfPath := &ast.PathType{Path: &ast.Path{Segments: []*ast.PathSegment{{Ident: ast.Ident{Name: "Self"}}}}}
	if sp := decl.SelfParam; sp != nil {
		switch sp.Kind {
		case ast.SelfRef, ast.SelfRefMut:
			lt := sp.Lifetime
			if lt == "" {
				lt = "'self"
			}
			r.inAst = append(r.inAst, &ast.RefType{Lifetime: lt, Mut: sp.Kind == ast.SelfRefMut, Elem: selfPath})
		case ast.SelfValue:
			r.inAst = append(r.inAst, selfPath)
		default:
			r.inAst = append(r.inAst, sp.Ty)
		}
	}
	for _, p := range decl.Params {
		r.inAst = append(r.inAst, p.Ty)
	}
	if sp := decl.SelfParam; sp != nil {
		w.occs = nil
		switch sp.Kind {
		case ast.SelfRef, ast.SelfRefMut:
			lt := sp.Lifetime
			if lt == "" {
				lt = "'self"
			}
			w.add(lt, 0)
			w.outer = append(w.outer, lt)
			if selfTy != nil {
				w.selfElided = 0
				w.inSelf++
				w.walkR(selfTy, 1)
				w.inSelf--
			}
			w.outer = nil
			r.names[lt] = "self"
		case ast.SelfValue:
			if selfTy != nil {
				w.selfElided = 0
				w.inSelf++
				w.walkR(selfTy, 0)
				w.inSelf--
			}
		default:
			w.walkR(sp.Ty, 0)
		}
		r.params = append(r.params, w.occs)
	}
	for _, p := range decl.Params {
		w.occs = nil
		w.walkR(p.Ty, 0)
		r.params = append(r.params, w.occs)
		if ip, ok := p.Pat.(*ast.IdentPat); ok {
			for _, o := range w.occs {
				if o.name[0] == '\'' {
					if _, dup := r.names[o.name]; !dup {
						r.names[o.name] = ip.Name.Name
					}
				}
			}
		}
	}
	if sp := decl.SelfParam; sp != nil && (sp.Kind == ast.SelfRef || sp.Kind == ast.SelfRefMut) {
		r.elided = sp.Lifetime
		if r.elided == "" {
			r.elided = "'self"
		}
	} else {
		lts := map[string]bool{}
		// lifetimes inside `Self` (a by-value `self`, `Box<Self>`) are
		// not input lifetime positions
		for _, os := range r.params {
			for _, o := range os {
				if o.name[0] == '\'' && !o.inSelf {
					lts[o.name] = true
				}
			}
		}
		if len(lts) == 1 {
			for n := range lts {
				r.elided = n
			}
		}
		if sp := decl.SelfParam; sp != nil && sp.Kind == ast.SelfTyped {
			if lt := selfRefLifetime(sp.Ty); lt != "" {
				r.elided = lt
			}
		}
	}
	r.nElided = w.elided
	w.occs = nil
	w.inOut = true
	w.walkR(decl.Output, 0)
	r.out = w.occs
	return r
}

// allowed returns the lifetimes the output (or a parameter) declares at
// depth d; any is true if a type parameter or opaque type appears there.
func (r *sigRegions) allowed(os []occ, d int) (map[string]bool, bool) {
	set := map[string]bool{}
	any := false
	for _, o := range os {
		if o.depth > d {
			continue
		}
		if o.name == "?" {
			any = true
			continue
		}
		if o.depth != d || o.name[0] != '\'' {
			continue
		}
		n := o.name
		if n == "'?" {
			n = r.elided
			if n == "" {
				// no lifetime to elide to (a signature rustc would
				// reject, or one we do not model): unsure
				any = true
				continue
			}
		}
		if n != "" {
			set[n] = true
		}
	}
	return set, any
}

// originOK reports whether origin o may appear where the lifetimes in set
// are declared.
func (r *sigRegions) originOK(o string, set map[string]bool) bool {
	for n := range set {
		if r.outlives(o, n) {
			return true
		}
	}
	return false
}

// regionError reports an origin flowing where the signature does not allow
// it.
func (lc *loanChecker) regionError(origin string, sp lexer.Span) {
	if debugLoans {
		fmt.Fprintf(os.Stderr, "borrowck: %s origin %s escapes (out=%v params=%v outlive=%v elided=%s)\n", sp.Lo, origin, lc.regions.out, lc.regions.params, lc.regions.outlive, lc.regions.elided)
	}
	name := lc.regions.names[origin]
	if len(origin) > 1 && origin[1] == '#' && name != "" {
		lc.errorf(sp, "E0621", "explicit lifetime required in the type of `%s`", name)
		return
	}
	msg := fmt.Sprintf("%s: error: lifetime may not live long enough", sp.Lo)
	if !lc.seen[msg] {
		lc.seen[msg] = true
		lc.diags = append(lc.diags, diag{span: sp, msg: msg})
	}
}
