package typeck

// Checks of generic parameter lists and of the lifetimes item signatures
// name (mirrors parts of rustc_resolve's late resolution): a generic
// parameter name is declared once (E0403), `'static` is not a parameter
// name (E0262), and every named lifetime in a signature is declared by the
// item, an enclosing impl or trait, or a `for<..>` binder (E0261).

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	"rustgo/compiler/resolve"
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
			// An associated item's parameters may not shadow the
			// enclosing impl's or trait's (E0403 / E0496).
			if d.Kind == DefFn && d.Parent != nil && (d.Parent.Kind == DefImpl || d.Parent.Kind == DefTrait) && d.Crate != nil && d.Crate.Local {
				if pg := genericsAst(d.Parent); pg != nil {
					outer := map[string]bool{}
					for _, p := range pg.Params {
						outer[p.Name.Name] = true
					}
					for _, p := range ga.Params {
						n := p.Name.Name
						if !outer[n] {
							continue
						}
						if p.Kind == ast.GPLifetime {
							if n != "'_" && n != "'static" {
								t.ErrorCode(p.Name.Span, "E0496", "lifetime name `%s` shadows a lifetime name that is already in scope", n)
							}
						} else {
							t.ErrorCode(p.Name.Span, "E0403", "the name `%s` is already used for a generic parameter in this item's generic parameters", n)
						}
					}
				}
			}
		}
		if d.Item.Vis.RestrictedPath() != nil && d.Scope != nil && d.Crate != nil && d.Crate.Local {
			// `pub(in path)`: path must name an ancestor module (E0742)
			if m := resolve.RestrictedModule(d.Item.Vis, d.Scope); m != nil {
				anc := false
				for x := d.Scope; x != nil; x = x.Parent {
					if x == m || x.Normal == m {
						anc = true
						break
					}
				}
				if !anc {
					t.ErrorCode(d.Item.Vis.RestrictedPath().Span, "E0742", "visibilities can only be restricted to ancestor modules")
				}
			}
		}
		if ga != nil && d.Crate != nil && d.Crate.Local {
			// const parameters are integers, `bool` or `char` (on stable)
			for _, p := range ga.Params {
				if p.Kind != ast.GPConst || p.ConstTy == nil {
					continue
				}
				var ct Ty
				if g := t.Generics(d); g != nil {
					for _, gp := range g.Own() {
						if gp.Const && gp.Ast == p {
							ct = gp.ConstTy
						}
					}
				}
				if ct == nil || ct.Kind == TInt || ct.Kind == TBool || ct.Kind == TChar || ct.HasError() {
					continue
				}
				what := "is forbidden as the type of a const generic parameter"
				if ct.HasParam() {
					t.ErrorCode(p.Name.Span, "E0770", "the type of const parameters must not depend on other generic parameters")
					continue
				}
				t.Errorf(p.Name.Span, "`%s` %s", ct, what)
			}
		}
		if en, ok := d.Item.Kind.(*ast.EnumItem); ok && d.Kind == DefEnum && d.Crate != nil && d.Crate.Local {
			// enum variants and their fields are as visible as the enum
			for _, v := range en.Variants {
				if v.Vis != ast.VisPrivate {
					t.ErrorCode(v.Name.Span, "E0449", "visibility qualifiers are not permitted here")
				}
				for _, f := range v.Fields {
					if f.Vis != ast.VisPrivate {
						t.ErrorCode(f.Span, "E0449", "visibility qualifiers are not permitted here")
					}
				}
			}
		}
		if d.Kind == DefImpl && d.Item.Vis != ast.VisPrivate && d.Crate != nil && d.Crate.Local {
			t.ErrorCode(d.Item.Span, "E0449", "visibility qualifiers are not permitted here")
		}
		t.checkSigLifetimes(d)
		t.checkBinderOutputs(ga)
		t.checkNeverType(d)
		t.checkGenericConstPatterns(d)
		if d.Kind != DefVariant {
			t.checkAttrTargets(d)
		}
		switch d.Kind {
		case DefStruct, DefEnum, DefUnion, DefTyAlias:
			t.checkUnusedParams(d)
		}
		if d.InTrait() && d.Item.Vis != ast.VisPrivate && d.Crate != nil && d.Crate.Local {
			t.ErrorCode(d.Item.Span, "E0449", "visibility qualifiers are not permitted here")
		}
		if fa, ok := d.Item.Kind.(*ast.FnItem); ok && fa.Const && d.Kind == DefFn && d.Crate != nil && d.Crate.Local {
			if d.InTrait() || (d.Parent != nil && d.Parent.Kind == DefImpl && d.Parent.Impl != nil && d.Parent.Impl.Ast != nil && d.Parent.Impl.Ast.Trait != nil) {
				t.ErrorCode(d.Item.Span, "E0379", "functions in traits cannot be declared const")
			}
		}
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

// typeLifetimes adds the named lifetimes a type mentions to out.
func typeLifetimes(n any, out map[string]bool) {
	if n == nil {
		return
	}
	ast.Walk(n, func(x any) bool {
		switch x := x.(type) {
		case *ast.RefType:
			out[x.Lifetime] = true
		case *ast.Path:
			for _, sg := range x.Segments {
				if sg.Args != nil {
					for _, a := range sg.Args.Args {
						out[a.Lifetime] = true
					}
				}
			}
		case *ast.DynTraitType:
			for _, b := range x.Bounds {
				out[b.Lifetime] = true
			}
		case *ast.ImplTraitType:
			for _, b := range x.Bounds {
				out[b.Lifetime] = true
			}
		}
		return true
	})
}

// checkBinderOutputs reports lifetimes of a `for<'a>` binder on an Fn*
// bound that only its output mentions (E0582): `for<'a> Fn() -> &'a T`
// would return a reference of any lifetime the caller picks.
func (t *Tcx) checkBinderOutputs(g *ast.Generics) {
	if g == nil {
		return
	}
	check := func(b *ast.Bound, outer []string) {
		if b.Path == nil || len(b.Path.Segments) == 0 {
			return
		}
		binder := append(append([]string(nil), outer...), b.ForLifetimes...)
		if len(binder) == 0 {
			return
		}
		ga := b.Path.Segments[len(b.Path.Segments)-1].Args
		if ga == nil || !ga.Parenthesized || ga.Output == nil {
			return
		}
		in, out := map[string]bool{}, map[string]bool{}
		for _, a := range ga.Inputs {
			typeLifetimes(a, in)
		}
		typeLifetimes(ga.Output, out)
		for _, lt := range binder {
			if lt != "" && lt != "'_" && out[lt] && !in[lt] {
				t.ErrorCode(b.Span, "E0582", "binding for associated type `Output` references lifetime `%s`, which does not appear in the trait input types", lt)
				return
			}
		}
	}
	for _, p := range g.Params {
		for _, b := range p.Bounds {
			check(b, nil)
		}
	}
	for _, w := range g.Where {
		for _, b := range w.Bounds {
			check(b, w.ForLifetimes)
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

// checkNeverType reports the `!` type outside a function's (or fn
// pointer's) return type: elsewhere it is unstable (E0658).
func (t *Tcx) checkNeverType(d *Def) {
	var visit func(n any) bool
	walkRet := func(ret ast.Type) {
		if _, ok := ret.(*ast.NeverType); ok || ret == nil {
			return
		}
		ast.Walk(ret, visit)
	}
	visit = func(n any) bool {
		switch x := n.(type) {
		case *ast.ItemStmt, *ast.MacroType:
			return false
		case *ast.NeverType:
			t.ErrorCode(x.Span, "E0658", "the `!` type is experimental")
			return false
		case *ast.FnPtrType:
			for _, in := range x.Inputs {
				ast.Walk(in, visit)
			}
			walkRet(x.Output)
			return false
		case *ast.ClosureExpr:
			walkRet(x.Ret)
			ast.Walk(x.Body, visit)
			return false
		}
		return true
	}
	switch k := d.Item.Kind.(type) {
	case *ast.FnItem:
		if k.Decl != nil {
			for _, p := range k.Decl.Params {
				ast.Walk(p.Ty, visit)
			}
			walkRet(k.Decl.Output)
		}
		if k.Body != nil {
			ast.Walk(k.Body, visit)
		}
	case *ast.StructItem:
		for _, f := range k.Fields {
			ast.Walk(f.Ty, visit)
		}
	case *ast.EnumItem:
		for _, v := range k.Variants {
			for _, f := range v.Fields {
				ast.Walk(f.Ty, visit)
			}
		}
	case *ast.TypeAliasItem:
		if k.Ty != nil {
			ast.Walk(k.Ty, visit)
		}
	}
}

// checkGenericConstPatterns reports patterns naming an associated constant
// of a generic type (`Self::MAX` in a trait's default method, `T::MIN`):
// their value is not known when the body is checked (E0158).
func (t *Tcx) checkGenericConstPatterns(d *Def) {
	fi, ok := d.Item.Kind.(*ast.FnItem)
	if !ok || fi.Body == nil {
		return
	}
	params := map[string]bool{}
	inTrait := d.Parent != nil && d.Parent.Kind == DefTrait
	if inTrait {
		params["Self"] = true
	}
	for g := d; g != nil; g = genericParent(g) {
		if ga := genericsAst(g); ga != nil {
			for _, p := range ga.Params {
				if p.Kind == ast.GPType {
					params[p.Name.Name] = true
				}
			}
		}
	}
	if len(params) == 0 {
		return
	}
	check := func(p *ast.Path, sp ast.Span) {
		if p == nil || len(p.Segments) != 2 || p.Global || !params[p.Segments[0].Ident.Name] || p.Segments[0].Ident.Hyg != 0 {
			return
		}
		// (an enum variant through `Self::V` in an impl is fine: only
		// trait bodies and plain type parameters are generic here)
		if p.Segments[0].Ident.Name == "Self" && !inTrait {
			return
		}
		t.ErrorCode(sp, "E0158", "constant pattern cannot depend on generic parameters")
	}
	ast.Walk(fi.Body, func(n any) bool {
		switch x := n.(type) {
		case *ast.ItemStmt, *ast.ClosureExpr:
			if _, ok := x.(*ast.ItemStmt); ok {
				return false
			}
		case *ast.PathPat:
			if x.QSelf == nil {
				check(x.Path, x.Span)
			}
		}
		return true
	})
}

// checkAttrTargets reports built-in attributes on items they do not apply
// to: `#[inline]` on anything but a function (E0518), `#[repr(..)]` on
// anything but a struct, enum or union, integer representations on
// anything but an enum (E0517), and two integer representations (E0566).
func (t *Tcx) checkAttrTargets(d *Def) {
	if d.Item == nil || d.Crate == nil || !d.Crate.Local {
		return
	}
	var what string
	isFn, isEnum, isAdt := false, false, false
	switch d.Item.Kind.(type) {
	case *ast.FnItem:
		what, isFn = "function", true
	case *ast.StructItem:
		what, isAdt = "struct", true
	case *ast.EnumItem:
		what, isAdt, isEnum = "enum", true, true
	case *ast.UnionItem:
		what, isAdt = "union", true
	case *ast.TraitItem:
		what = "trait"
	case *ast.TypeAliasItem:
		what = "type alias"
	case *ast.ConstItem:
		what = "constant"
	case *ast.StaticItem:
		what = "static item"
	case *ast.ModItem:
		what = "module"
	default:
		return
	}
	for _, a := range d.Item.Attrs {
		switch a.Path {
		case "inline":
			if !isFn {
				t.ErrorCode(a.Span, "E0518", "the `inline` attribute cannot be used on %ss", what)
			}
		case "repr":
			if len(a.Tokens) == 0 || a.Tokens[0].Delim != '(' {
				continue
			}
			nint := 0
			for _, tt := range a.Tokens[0].Inner {
				if tt.Delim != 0 || tt.Tok.Kind != lexer.Ident {
					continue
				}
				name := tt.Tok.Text
				_, isInt := IntTyByName(name)
				switch {
				case isInt:
					nint++
					if !isEnum {
						t.ErrorCode(tt.Tok.Span, "E0517", "the `repr(%s)` attribute cannot be used on %ss", name, what)
					}
				case name == "C" || name == "transparent" || name == "packed" || name == "align" || name == "simd":
					if !isAdt {
						t.ErrorCode(a.Span, "E0517", "the `repr(%s)` attribute cannot be used on %ss", name, what)
					}
				}
			}
			if nint > 1 {
				t.ErrorCode(a.Span, "E0566", "conflicting representation hints")
			}
		}
	}
}
