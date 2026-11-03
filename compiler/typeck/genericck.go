package typeck

// Checks of generic parameter lists and of the lifetimes item signatures
// name (mirrors parts of rustc_resolve's late resolution): a generic
// parameter name is declared once (E0403), `'static` is not a parameter
// name (E0262), and every named lifetime in a signature is declared by the
// item, an enclosing impl or trait, or a `for<..>` binder (E0261).

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
)

func (t *Tcx) checkGenericParams() {
	for _, d := range t.R.Defs {
		if t.isStd(d) || d.Item == nil {
			continue
		}
		ga := genericsAst(d)
		if ga != nil {
			seen := map[string]bool{}
			for _, p := range ga.Params {
				n := p.Name.Name
				if p.Kind == ast.GPLifetime && n == "'static" {
					t.ErrorCode(p.Name.Span, "E0262", "invalid lifetime parameter name: `'static`")
					continue
				}
				if seen[n] {
					t.ErrorCode(p.Name.Span, "E0403", "the name `%s` is already used for a generic parameter in this item's generic parameters", n)
				}
				seen[n] = true
			}
		}
		t.checkSigLifetimes(d)
		for _, a := range d.Item.Attrs {
			if a.Path != "repr" || len(a.Tokens) == 0 || a.Tokens[0].Delim != '(' {
				continue
			}
			for _, tt := range a.Tokens[0].Inner {
				if tt.Delim == 0 && (tt.Tok.Kind == lexer.Str || tt.Tok.Kind == lexer.Int) {
					t.ErrorCode(tt.Tok.Span, "E0565", "meta item in `repr` must be an identifier")
				}
			}
		}
	}
}

// declaredLifetimes returns the lifetimes in scope in d's signature.
func declaredLifetimes(d *Def) map[string]bool {
	m := map[string]bool{"'static": true, "'_": true, "": true}
	for g := d; g != nil; g = genericParent(g) {
		if ga := genericsAst(g); ga != nil {
			for _, p := range ga.Params {
				if p.Kind == ast.GPLifetime {
					m[p.Name.Name] = true
				}
			}
		}
	}
	return m
}

// checkSigLifetimes reports undeclared lifetimes in d's signature.
func (t *Tcx) checkSigLifetimes(d *Def) {
	scope := declaredLifetimes(d)
	reported := map[string]bool{}
	var visit func(n any, scope map[string]bool) bool
	var walkBound func(b *ast.Bound, scope map[string]bool)
	use := func(lt string, sp ast.Span, scope map[string]bool) {
		if !scope[lt] && !reported[lt] {
			reported[lt] = true
			t.ErrorCode(sp, "E0261", "use of undeclared lifetime name `%s`", lt)
		}
	}
	with := func(scope map[string]bool, lts []string) map[string]bool {
		if len(lts) == 0 {
			return scope
		}
		m := map[string]bool{}
		for k := range scope {
			m[k] = true
		}
		for _, l := range lts {
			m[l] = true
		}
		return m
	}
	walk := func(n any, scope map[string]bool) {
		if n == nil {
			return
		}
		ast.Walk(n, func(x any) bool { return visit(x, scope) })
	}
	visit = func(n any, scope map[string]bool) bool {
		switch x := n.(type) {
		case *ast.FnPtrType, *ast.MacroType, *ast.Block, ast.Expr:
			// (fn pointer binders are not kept by the parser)
			return false
		case *ast.RefType:
			use(x.Lifetime, x.Span, scope)
		case *ast.Path:
			for _, sg := range x.Segments {
				if sg.Args == nil {
					continue
				}
				if sg.Args.Parenthesized {
					return false // Fn(&'a T) sugar: binders are implicit
				}
				for _, a := range sg.Args.Args {
					if a.Lifetime != "" {
						use(a.Lifetime, sg.Args.Span, scope)
					}
				}
			}
		case *ast.DynTraitType:
			for _, b := range x.Bounds {
				walkBound(b, scope)
			}
			return false
		case *ast.ImplTraitType:
			for _, b := range x.Bounds {
				walkBound(b, scope)
			}
			return false
		}
		return true
	}
	walkBound = func(b *ast.Bound, scope map[string]bool) {
		if b.Lifetime != "" {
			use(b.Lifetime, b.Span, scope)
		}
		if b.Path != nil {
			walk(b.Path, with(scope, b.ForLifetimes))
		}
	}
	switch k := d.Item.Kind.(type) {
	case *ast.StructItem:
		for _, f := range k.Fields {
			walk(f.Ty, scope)
		}
	case *ast.UnionItem:
		for _, f := range k.Fields {
			walk(f.Ty, scope)
		}
	case *ast.EnumItem:
		for _, v := range k.Variants {
			for _, f := range v.Fields {
				walk(f.Ty, scope)
			}
		}
	case *ast.ImplItem:
		walk(k.SelfTy, scope)
		if k.Trait != nil {
			walk(k.Trait, scope)
		}
	case *ast.FnItem:
		if k.Decl == nil {
			return
		}
		for _, p := range k.Decl.Params {
			walk(p.Ty, scope)
		}
		if k.Decl.Output != nil {
			walk(k.Decl.Output, scope)
		}
	default:
		return
	}
	if ga := genericsAst(d); ga != nil {
		for _, p := range ga.Params {
			for _, b := range p.Bounds {
				walkBound(b, scope)
			}
		}
		for _, w := range ga.Where {
			s := with(scope, w.ForLifetimes)
			if w.Lifetime != "" {
				use(w.Lifetime, w.Span, s)
			}
			walk(w.Ty, s)
			for _, b := range w.Bounds {
				walkBound(b, s)
			}
		}
	}
}
