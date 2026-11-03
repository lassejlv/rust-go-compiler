// Package resolve builds the module tree of each crate, creates definitions
// for all items, resolves `use` imports and offers path resolution to later
// passes (mirrors rustc_resolve).
package resolve

import (
	"fmt"
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
)

// Resolver owns all crates and definitions.
type Resolver struct {
	Crates    map[string]*Crate
	CrateList []*Crate
	Std       *Crate
	Prelude   *Module
	Defs      []*Def
	Impls     []*Def
	Traits    []*Def
	LangItems map[string]*Def
	// BlockMods maps a Block node id to the anonymous module holding items
	// declared in that block.
	BlockMods map[ast.NodeID]*Module
	// FnBodyScope maps item node ids of fns to the def.
	ItemDefs map[*ast.Item]*Def
	Errors   []string
	// IsBuiltinMacro reports names of builtin macros (imports of them,
	// e.g. `use std::pin::pin;`, are accepted).
	IsBuiltinMacro func(string) bool
	// IsCrateMacro reports whether the named crate defines a macro_rules!
	// macro `name`. Macros are expanded (and their definitions removed)
	// before resolution, so imports/re-exports of them (`pub(crate) use m;`,
	// `use crate::m;`) are accepted without binding anything.
	IsCrateMacro func(crate, name string) bool

	// LinkArgs are the linker arguments for the `#[link(name = "..")]`
	// attributes of extern blocks.
	LinkArgs []string
	linkSeen []string

	// bindingImport maps import bindings to the import that created them
	// (an import must not resolve to its own binding).
	bindingImport map[*NameBinding]*Import
	// forcing is set once import resolution has stalled: names that might
	// still be defined by pending imports are then taken as absent.
	forcing bool
	// changed records that an import pass added or changed a binding.
	changed bool
	// macroReexports: macros a crate re-exports by `pub use` (per crate).
	macroReexports map[*Crate]map[string]bool
}

func (r *Resolver) isMacro(c *Crate, name string) bool {
	if r.IsBuiltinMacro != nil && r.IsBuiltinMacro(name) {
		return true
	}
	if c != nil && r.macroReexports[c][name] {
		return true
	}
	return c != nil && r.IsCrateMacro != nil && r.IsCrateMacro(c.Name, name)
}

func New() *Resolver {
	return &Resolver{
		Crates:    map[string]*Crate{},
		LangItems: map[string]*Def{},
		BlockMods: map[ast.NodeID]*Module{},
		ItemDefs:  map[*ast.Item]*Def{},

		bindingImport: map[*NameBinding]*Import{},
	}
}

func (r *Resolver) errorf(sp lexer.Span, format string, args ...any) {
	r.Errors = append(r.Errors, fmt.Sprintf("%s: error: %s", sp.Lo, fmt.Sprintf(format, args...)))
}

// Err returns accumulated errors.
func (r *Resolver) Err() error {
	if len(r.Errors) == 0 {
		return nil
	}
	return fmt.Errorf("%s", strings.Join(r.Errors, "\n"))
}

func (r *Resolver) newDef(kind DefKind, name string, parent *Def) *Def {
	d := NewDef(kind, name, parent)
	r.Defs = append(r.Defs, d)
	return d
}

// AddCrate builds the module tree and defs for a parsed+expanded crate.
func (r *Resolver) AddCrate(name string, c *ast.Crate) *Crate {
	cr := &Crate{Name: name, Ast: c}
	cd := r.newDef(DefCrate, name, nil)
	cd.Crate = cr
	cr.Def = cd
	cr.Root = NewModule(cd, nil, cr, false)
	cd.Mod = cr.Root
	cd.Scope = cr.Root
	r.Crates[name] = cr
	r.CrateList = append(r.CrateList, cr)
	if name == "std" {
		r.Std = cr
		r.Crates["core"] = cr
		r.Crates["alloc"] = cr
	}
	for _, it := range c.Items {
		r.collectItem(it, cr.Root, cd)
	}
	return cr
}

func langItemName(attrs []*ast.Attr) string {
	for _, a := range attrs {
		if a.Path == "lang" && len(a.Tokens) >= 2 && a.Tokens[0].Tok.Is("=") {
			return a.Tokens[1].Tok.Text
		}
	}
	return ""
}

func (r *Resolver) define(m *Module, name string, ns Namespace, res Res, sp lexer.Span) {
	r.defineVis(m, name, ns, res, sp, ast.VisPub)
}

func (r *Resolver) defineVis(m *Module, name string, ns Namespace, res Res, sp lexer.Span, vis ast.Visibility) {
	if name == "" || name == "_" {
		return
	}
	if r.Std == nil || m.Crate != r.Std {
		tbl := m.Types
		if ns == ValueNS {
			tbl = m.Values
		}
		if old, ok := tbl[name]; ok && !old.Import && old.Res != res {
			msg := fmt.Sprintf("%s: error[E0428]: the name `%s` is defined multiple times", sp.Lo, name)
			if n := len(r.Errors); n == 0 || r.Errors[n-1] != msg {
				r.Errors = append(r.Errors, msg)
			}
		}
	}
	m.Define(name, ns, &NameBinding{Res: res, Span: sp, Vis: vis})
}

func defRes(d *Def) Res { return Res{Kind: ResDef, Def: d} }

// collectItem creates the def(s) for an item declared in module m.
func (r *Resolver) collectItem(it *ast.Item, m *Module, parent *Def) *Def {
	name := it.Name.Name
	var d *Def
	switch k := it.Kind.(type) {
	case *ast.FnItem:
		d = r.newDef(DefFn, name, parent)
		d.Fn = &FnInfo{Ast: k, HasSelf: k.Decl.SelfParam != nil}
		r.defineVis(m, name, ValueNS, defRes(d), it.Name.Span, it.Vis)
		if k.Body != nil {
			r.collectBody(k.Body, m, d)
		}
	case *ast.StructItem:
		d = r.newDef(DefStruct, name, parent)
		d.Adt = &AdtDef{Kind: AdtStruct}
		r.defineVis(m, name, TypeNS, defRes(d), it.Name.Span, it.Vis)
		if k.Shape != ast.ShapeNamed {
			r.defineVis(m, name, ValueNS, Res{Kind: ResCtor, Def: d}, it.Name.Span, it.Vis)
		}
	case *ast.UnionItem:
		d = r.newDef(DefUnion, name, parent)
		d.Adt = &AdtDef{Kind: AdtUnion}
		r.defineVis(m, name, TypeNS, defRes(d), it.Name.Span, it.Vis)
	case *ast.EnumItem:
		d = r.newDef(DefEnum, name, parent)
		d.Adt = &AdtDef{Kind: AdtEnum}
		d.Mod = NewModule(d, m, m.Crate, false)
		d.Mod.Normal = d.Mod
		r.defineVis(m, name, TypeNS, defRes(d), it.Name.Span, it.Vis)
		for i, v := range k.Variants {
			vd := r.newDef(DefVariant, v.Name.Name, d)
			vd.Item = it
			vd.Span = v.Span
			vd.Attrs = v.Attrs
			vd.Scope = m
			vd.Variant = &VariantDef{Def: vd, Name: v.Name.Name, Index: i, Shape: v.Shape, Ast: v}
			d.Adt.Variants = append(d.Adt.Variants, vd.Variant)
			r.define(d.Mod, v.Name.Name, TypeNS, defRes(vd), v.Name.Span)
			if v.Shape != ast.ShapeNamed {
				r.define(d.Mod, v.Name.Name, ValueNS, Res{Kind: ResCtor, Def: vd}, v.Name.Span)
			}
			if ln := langItemName(v.Attrs); ln != "" {
				r.LangItems[ln] = vd
				vd.LangItem = ln
			}
		}
	case *ast.TraitItem:
		d = r.newDef(DefTrait, name, parent)
		d.Trait = &TraitInfo{Items: map[string]*Def{}, Auto: k.Auto}
		d.Mod = NewModule(d, m, m.Crate, false)
		d.Mod.Normal = d.Mod
		r.defineVis(m, name, TypeNS, defRes(d), it.Name.Span, it.Vis)
		r.Traits = append(r.Traits, d)
		for _, sub := range k.Items {
			sd := r.collectAssoc(sub, m, d)
			if sd == nil {
				continue
			}
			d.Trait.Items[sd.Name] = sd
			d.Trait.Order = append(d.Trait.Order, sd)
			ns := ValueNS
			if sd.Kind == DefAssocTy {
				ns = TypeNS
			}
			r.define(d.Mod, sd.Name, ns, defRes(sd), sub.Name.Span)
		}
		d.Trait.Marker = len(k.Items) == 0
	case *ast.ImplItem:
		d = r.newDef(DefImpl, "", parent)
		d.Impl = &ImplInfo{Items: map[string]*Def{}, Negative: k.Negative, Ast: k}
		r.Impls = append(r.Impls, d)
		for _, sub := range k.Items {
			sd := r.collectAssoc(sub, m, d)
			if sd == nil {
				continue
			}
			d.Impl.Items[sd.Name] = sd
			d.Impl.Order = append(d.Impl.Order, sd)
		}
	case *ast.TypeAliasItem:
		d = r.newDef(DefTyAlias, name, parent)
		d.Alias = &AliasInfo{TyAst: k.Ty}
		r.defineVis(m, name, TypeNS, defRes(d), it.Name.Span, it.Vis)
	case *ast.ConstItem:
		d = r.newDef(DefConst, name, parent)
		d.Const = &ConstInfo{Expr: k.Expr, TyAst: k.Ty}
		r.defineVis(m, name, ValueNS, defRes(d), it.Name.Span, it.Vis)
		if k.Ty != nil {
			r.collectBody(k.Ty, m, d) // blocks in array lengths
		}
		if k.Expr != nil {
			r.collectBody(k.Expr, m, d)
		}
	case *ast.StaticItem:
		d = r.newDef(DefStatic, name, parent)
		d.Const = &ConstInfo{Expr: k.Expr, TyAst: k.Ty, Mut: k.Mut, Static: true}
		r.defineVis(m, name, ValueNS, defRes(d), it.Name.Span, it.Vis)
		if k.Ty != nil {
			r.collectBody(k.Ty, m, d) // blocks in array lengths
		}
		if k.Expr != nil {
			r.collectBody(k.Expr, m, d)
		}
	case *ast.ModItem:
		d = r.newDef(DefMod, name, parent)
		d.Mod = NewModule(d, m, m.Crate, false)
		r.defineVis(m, name, TypeNS, defRes(d), it.Name.Span, it.Vis)
		for _, sub := range k.Items {
			r.collectItem(sub, d.Mod, d)
		}
	case *ast.UseItem:
		r.collectUse(k.Tree, nil, false, m, it.Vis)
		return nil
	case *ast.ExternCrateItem:
		bind := name
		if k.Rename != nil {
			bind = k.Rename.Name
		}
		if cr := r.ExternCrate(m.Crate, name); cr != nil {
			r.defineVis(m, bind, TypeNS, defRes(cr.Def), it.Name.Span, it.Vis)
			// 2018+: `extern crate x as y;` at the crate root adds `y` to
			// the extern prelude (usable from every module)
			if bind != name && m == m.Crate.Root {
				if m.Crate.Externs == nil {
					m.Crate.Externs = map[string]*Crate{}
				}
				if _, exists := m.Crate.Externs[bind]; !exists {
					m.Crate.Externs[bind] = cr
				}
			}
		} else if name == "std" || name == "core" || name == "alloc" {
			// std may not be loaded yet if we are std itself; resolve lazily via path.
			m.Imports = append(m.Imports, &Import{Module: m, Path: []string{name}, Global: true, Name: bind, Span: it.Span})
		}
		return nil
	case *ast.ExternBlockItem:
		for _, a := range it.Attrs {
			if a.Path == "link" {
				r.addLinkAttr(a)
			}
		}
		for _, sub := range k.Items {
			sd := r.collectItem(sub, m, parent)
			if sd != nil && sd.Fn != nil {
				sd.Fn.Extern = true
				if k.Abi == "rust-intrinsic" {
					sd.Intrinsic = sd.Name
				}
			}
		}
		return nil
	default:
		return nil
	}
	d.Item = it
	d.Span = it.Span
	d.Attrs = it.Attrs
	d.Scope = m
	r.ItemDefs[it] = d
	m.Items = append(m.Items, d)
	if ln := langItemName(it.Attrs); ln != "" {
		r.LangItems[ln] = d
		d.LangItem = ln
	}
	if d.Kind == DefFn && d.Fn.Ast.Body == nil && d.Fn.Ast.Abi == "rust-intrinsic" {
		d.Intrinsic = d.Name
	}
	for _, a := range it.Attrs {
		if a.Path == "rustc_intrinsic" {
			d.Intrinsic = d.Name
		}
	}
	return d
}

// collectAssoc creates defs for items inside traits and impls.
func (r *Resolver) collectAssoc(it *ast.Item, m *Module, parent *Def) *Def {
	var d *Def
	switch k := it.Kind.(type) {
	case *ast.FnItem:
		d = r.newDef(DefFn, it.Name.Name, parent)
		d.Fn = &FnInfo{Ast: k, HasSelf: k.Decl.SelfParam != nil}
		if k.Body != nil {
			r.collectBody(k.Body, m, d)
		}
	case *ast.ConstItem:
		d = r.newDef(DefConst, it.Name.Name, parent)
		d.Const = &ConstInfo{Expr: k.Expr, TyAst: k.Ty}
		if k.Ty != nil {
			r.collectBody(k.Ty, m, d) // blocks in array lengths
		}
		if k.Expr != nil {
			r.collectBody(k.Expr, m, d)
		}
	case *ast.TypeAliasItem:
		d = r.newDef(DefAssocTy, it.Name.Name, parent)
		d.AssocTy = &AssocTyInfo{TyAst: k.Ty, Default: k.Ty}
	default:
		return nil
	}
	d.Item = it
	d.Span = it.Span
	d.Attrs = it.Attrs
	d.Scope = m
	r.ItemDefs[it] = d
	if ln := langItemName(it.Attrs); ln != "" {
		r.LangItems[ln] = d
		d.LangItem = ln
	}
	for _, a := range it.Attrs {
		if a.Path == "rustc_intrinsic" {
			d.Intrinsic = d.Name
		}
	}
	return d
}

// collectBody finds blocks containing items inside a body and creates
// anonymous modules for them.
func (r *Resolver) collectBody(body any, m *Module, owner *Def) {
	var walk func(n any, cur *Module)
	walk = func(n any, cur *Module) {
		ast.Walk(n, func(x any) bool {
			b, ok := x.(*ast.Block)
			if !ok {
				return true
			}
			hasItems := false
			for _, s := range b.Stmts {
				if _, ok := s.(*ast.ItemStmt); ok {
					hasItems = true
					break
				}
			}
			if !hasItems {
				return true
			}
			bm := NewModule(owner, cur, cur.Crate, true)
			r.BlockMods[b.ID] = bm
			for _, s := range b.Stmts {
				if is, ok := s.(*ast.ItemStmt); ok {
					r.collectItem(is.Item, bm, owner)
				}
			}
			// Walk non-item statements with the block module as scope.
			for _, s := range b.Stmts {
				if _, ok := s.(*ast.ItemStmt); !ok {
					walk(s, bm)
				}
			}
			return false
		})
	}
	walk(body, m)
}

// collectUse flattens a use tree into imports.
func (r *Resolver) collectUse(t *ast.UseTree, prefix []string, global bool, m *Module, vis ast.Visibility) {
	segs := append([]string{}, prefix...)
	if t.Prefix != nil {
		if t.Prefix.Global && len(prefix) == 0 {
			global = true
		}
		for _, s := range t.Prefix.Segments {
			segs = append(segs, s.Ident.Name)
		}
	}
	switch t.Kind {
	case ast.UseSimple:
		if len(segs) == 0 {
			return
		}
		name := segs[len(segs)-1]
		path := segs
		if name == "self" {
			// use a::b::{self} -> binds b
			path = segs[:len(segs)-1]
			if len(path) == 0 {
				return
			}
			name = path[len(path)-1]
		}
		if t.Rename != nil {
			name = t.Rename.Name
		}
		m.Imports = append(m.Imports, &Import{Module: m, Path: path, Global: global, Name: name, Span: t.Span, Vis: vis})
	case ast.UseGlob:
		m.Imports = append(m.Imports, &Import{Module: m, Path: segs, Global: global, Glob: true, Span: t.Span, Vis: vis})
	case ast.UseNested:
		for _, n := range t.Nested {
			r.collectUse(n, segs, global, m, vis)
		}
	}
}

// ---------------------------------------------------------------------------
// Import resolution

func (r *Resolver) allModules() []*Module {
	var out []*Module
	seen := map[*Module]bool{}
	var visit func(m *Module)
	visit = func(m *Module) {
		if m == nil || seen[m] {
			return
		}
		seen[m] = true
		out = append(out, m)
		for _, d := range m.Items {
			if d.Kind == DefMod {
				visit(d.Mod)
			}
		}
	}
	for _, c := range r.CrateList {
		visit(c.Root)
	}
	for _, bm := range r.BlockMods {
		visit(bm) // including modules declared inside blocks
	}
	return out
}

// ResolveImports resolves all `use` declarations to a fixpoint. As in
// rustc, an import is only settled once each namespace of the imported name
// is determined: found, or certainly absent because no pending import can
// still define it. If resolution stalls (cyclic dependencies between
// imports), undetermined names are taken as absent and resolution resumes.
func (r *Resolver) ResolveImports() {
	if r.Std != nil {
		if pm := r.Std.Root.Lookup("prelude", TypeNS); pm != nil && pm.Res.Def != nil && pm.Res.Def.Mod != nil {
			r.Prelude = pm.Res.Def.Mod
		}
	}
	mods := r.allModules()
	pass := func() (progress bool, pending int) {
		r.changed = false
		for _, m := range mods {
			for _, imp := range m.Imports {
				if imp.Done {
					continue
				}
				if r.tryImport(imp, false) {
					imp.Done = true
					progress = true
				} else {
					pending++
				}
			}
		}
		return progress || r.changed, pending
	}
	for iter := 0; iter < 10000; iter++ {
		progress, pending := pass()
		if pending == 0 {
			break
		}
		if progress {
			continue
		}
		r.forcing = true
		progress, _ = pass()
		r.forcing = false
		if !progress {
			break
		}
	}
	r.forcing = true
	for _, m := range mods {
		for _, imp := range m.Imports {
			if !imp.Done {
				r.tryImport(imp, true)
			}
		}
	}
	r.forcing = false
	// prelude may be defined via imports
	if r.Std != nil && r.Prelude == nil {
		if pm := r.Std.Root.Lookup("prelude", TypeNS); pm != nil && pm.Res.Def != nil && pm.Res.Def.Mod != nil {
			r.Prelude = pm.Res.Def.Mod
		}
	}
}

// undetermined reports whether a pending import (other than self) of module
// m, or of a module m glob-imports, may still define name.
func (r *Resolver) undetermined(m *Module, name string, self *Import) bool {
	if r.forcing {
		return false
	}
	return r.undeterminedIn(m, name, self, map[*Module]bool{})
}

func (r *Resolver) undeterminedIn(m *Module, name string, self *Import, seen map[*Module]bool) bool {
	if m == nil || seen[m] {
		return false
	}
	seen[m] = true
	for _, imp := range m.Imports {
		if imp.Done || imp == self {
			continue
		}
		if imp.Glob || imp.Name == name {
			return true
		}
	}
	for _, g := range m.Globs {
		if r.undeterminedIn(g, name, self, seen) {
			return true
		}
	}
	return false
}

// lookupDet looks up name in module m on behalf of import self (whose own
// binding is skipped). determined=false means a pending import may still
// define the name there.
func (r *Resolver) lookupDet(m *Module, name string, ns Namespace, self *Import) (b *NameBinding, determined bool) {
	tbl := m.Types
	if ns == ValueNS {
		tbl = m.Values
	}
	// Items are final; so are import bindings, which are only made once
	// their namespace is determined.
	if b, ok := tbl[name]; ok && (self == nil || r.bindingImport[b] != self) {
		return b, true
	}
	if r.undetermined(m, name, self) {
		return nil, false
	}
	for _, g := range m.Globs {
		if b := g.Lookup(name, ns); b != nil && (self == nil || r.bindingImport[b] != self) {
			return b, true
		}
	}
	return nil, true
}

// lookupUseFirst resolves the first segment of a 2018 `use` path (or a
// single-segment import) from module/block scope m: items and imports of
// the enclosing blocks and module, then the extern prelude, then the std
// prelude.
func (r *Resolver) lookupUseFirst(m *Module, name string, ns Namespace, self *Import) (res Res, found, determined bool) {
	for x := m; x != nil; x = x.Parent {
		b, det := r.lookupDet(x, name, ns, self)
		if !det {
			return Res{}, false, false
		}
		if b != nil {
			return b.Res, true, true
		}
		if !x.Block {
			break
		}
	}
	if ns == TypeNS {
		if cr := r.ExternCrate(m.Crate, name); cr != nil {
			return defRes(cr.Def), true, true
		}
	}
	if r.Prelude != nil {
		b, det := r.lookupDet(r.Prelude, name, ns, self)
		if !det {
			return Res{}, false, false
		}
		if b != nil {
			return b.Res, true, true
		}
	}
	// `use u64;` / `pub use bool;` (how core::primitive is defined)
	if ns == TypeNS && primTypes[name] {
		return Res{Kind: ResPrim, Prim: name}, true, true
	}
	return Res{}, false, true
}

func (r *Resolver) tryImport(imp *Import, report bool) bool {
	m := imp.Module
	if imp.Glob {
		res, ok := r.resolveModulePath(m, imp.Path, imp.Global, report, imp.Span, imp)
		if !ok {
			return false
		}
		if res.Def == nil || res.Def.Mod == nil {
			if report {
				r.errorf(imp.Span, "cannot glob-import from `%s`", strings.Join(imp.Path, "::"))
			}
			return report
		}
		m.Globs = append(m.Globs, res.Def.Mod)
		r.changed = true
		return true
	}
	last := imp.Path[len(imp.Path)-1]
	// lookup finds the imported name in one namespace.
	var lookup func(ns Namespace) (res Res, found, determined bool)
	var macroCrate *Crate
	// privacy of the imported item (same crate; E0603)
	var privRes Res
	pubSeen := false
	if len(imp.Path) == 1 {
		// `use foo;` / `use foo as bar;`
		macroCrate = m.Crate
		switch {
		case imp.Global && !m.Crate.Is2015():
			lookup = func(ns Namespace) (Res, bool, bool) {
				if ns == TypeNS {
					if cr := r.ExternCrate(m.Crate, last); cr != nil {
						return defRes(cr.Def), true, true
					}
				}
				return Res{}, false, true
			}
		case m.Crate.Is2015() && !(m.Block && !imp.Global):
			// 2015: relative to the crate root, then extern crates
			root := m.Crate.Root
			lookup = func(ns Namespace) (Res, bool, bool) {
				if !imp.Global || !r.injectedStd(m.Crate, last) {
					b, det := r.lookupDet(root, last, ns, imp)
					if !det {
						return Res{}, false, false
					}
					if b != nil {
						return b.Res, true, true
					}
				}
				if ns == TypeNS {
					if cr := r.ExternCrate(m.Crate, last); cr != nil {
						return defRes(cr.Def), true, true
					}
				}
				return Res{}, false, true
			}
		default:
			lookup = func(ns Namespace) (Res, bool, bool) {
				return r.lookupUseFirst(m, last, ns, imp)
			}
		}
	} else {
		prefix := imp.Path[:len(imp.Path)-1]
		res, ok := r.resolveModulePath(m, prefix, imp.Global, report, imp.Span, imp)
		if !ok {
			return false
		}
		if res.Def == nil || res.Def.Mod == nil {
			if report {
				r.errorf(imp.Span, "unresolved import `%s`: `%s` is not a module", strings.Join(imp.Path, "::"), strings.Join(prefix, "::"))
			}
			return report
		}
		target := res.Def.Mod
		macroCrate = target.Crate
		lookup = func(ns Namespace) (Res, bool, bool) {
			b, det := r.lookupDet(target, last, ns, imp)
			if b == nil {
				return Res{}, false, det
			}
			// privacy: another crate's private items are not importable
			// (`use memchr::memchr` must not pick up memchr's private
			// `mod memchr` in the type namespace)
			if target.Crate != m.Crate && !b.Import && b.Res.Kind == ResDef && b.Res.Def != nil &&
				b.Res.Def.Item != nil && b.Res.Def.Item.Vis != ast.VisPub && target.Def != nil &&
				target.Def.Kind != DefEnum && target.Def.Kind != DefTrait {
				return Res{}, false, true
			}
			if target.Crate == m.Crate {
				if r.BindingAccessible(target, last, ns, b, m) {
					pubSeen = true
				} else if privRes.Def == nil {
					privRes = b.Res
				}
			}
			return b.Res, true, true
		}
	}
	found, determined := false, true
	for _, ns := range []Namespace{TypeNS, ValueNS} {
		res, ok, det := lookup(ns)
		if !det {
			determined = false
			continue
		}
		if ok {
			r.bindImport(m, imp, ns, res)
			found = true
		}
	}
	if !determined {
		return false
	}
	if found && privRes.Def != nil && !pubSeen {
		kind := privRes.Def.Kind.String()
		if privRes.Kind == ResCtor && privRes.Def.Kind == DefStruct {
			kind = "tuple struct constructor"
		}
		r.Errors = append(r.Errors, fmt.Sprintf("%s: error[E0603]: %s `%s` is private", imp.Span.Lo, kind, last))
	}
	if !found && r.isMacro(macroCrate, last) {
		// macros live in a namespace we don't model; remember re-exports
		// (`pub use dep::some_macro;`) so other crates can import them
		if macroCrate != m.Crate && imp.Name != "" && imp.Name != "_" {
			if r.macroReexports == nil {
				r.macroReexports = map[*Crate]map[string]bool{}
			}
			if r.macroReexports[m.Crate] == nil {
				r.macroReexports[m.Crate] = map[string]bool{}
			}
			r.macroReexports[m.Crate][imp.Name] = true
		}
		return true
	}
	if !found {
		if report {
			if len(imp.Path) == 1 {
				r.errorf(imp.Span, "unresolved import `%s`", last)
			} else {
				r.errorf(imp.Span, "unresolved import `%s`: no `%s` in `%s`", strings.Join(imp.Path, "::"), last, strings.Join(imp.Path[:len(imp.Path)-1], "::"))
			}
			return true
		}
		return false
	}
	return true
}

func (r *Resolver) bindImport(m *Module, imp *Import, ns Namespace, res Res) {
	name := imp.Name
	if name == "_" {
		if res.Def != nil && res.Def.Kind == DefTrait {
			for _, t := range m.AnonTraits {
				if t == res.Def {
					return
				}
			}
			m.AnonTraits = append(m.AnonTraits, res.Def)
			r.changed = true
		}
		return
	}
	if name == "" {
		return
	}
	tbl := m.Types
	if ns == ValueNS {
		tbl = m.Values
	}
	old, ok := tbl[name]
	if ok && !old.Import {
		return // items shadow imports
	}
	if ok && old.Res == res {
		return
	}
	b := &NameBinding{Res: res, Import: true, Span: imp.Span, Vis: imp.Vis}
	tbl[name] = b
	r.bindingImport[b] = imp
	r.changed = true
}

// resolveModulePath resolves a path whose every segment must be a module-like
// (module, crate, enum, trait) on behalf of import imp. ok=false without a
// reported error means the path may still resolve once pending imports are.
func (r *Resolver) resolveModulePath(m *Module, segs []string, global bool, report bool, sp lexer.Span, imp *Import) (Res, bool) {
	if len(segs) == 0 {
		return defRes(m.Normal.Def), true
	}
	var cur Res
	start := 0
	switch {
	case global && m.Crate.Is2015() && !r.injectedStd(m.Crate, segs[0]):
		// 2015: `::a::b` is relative to the crate root
		cur = defRes(m.Crate.Def)
	case global:
		cr := r.ExternCrate(m.Crate, segs[0])
		if cr == nil {
			if report {
				r.errorf(sp, "unresolved crate `%s`", segs[0])
			}
			return Res{}, false
		}
		cur = defRes(cr.Def)
		start = 1
	case m.Crate.Is2015() && segs[0] != "self" && segs[0] != "super" && segs[0] != "crate" && segs[0] != "Self":
		// 2015 `use` paths are relative to the crate root
		b, det := r.lookupDet(m.Crate.Root, segs[0], TypeNS, imp)
		if !det {
			return Res{}, false
		}
		if b != nil {
			cur = b.Res
			start = 1
		} else if cr := r.ExternCrate(m.Crate, segs[0]); cr != nil {
			cur = defRes(cr.Def)
			start = 1
		} else {
			if report {
				r.errorf(sp, "unresolved import: cannot find `%s` in crate root", segs[0])
			}
			return Res{}, false
		}
	case segs[0] == "crate":
		cur = defRes(m.Crate.Def)
		start = 1
	case segs[0] == "self":
		cur = defRes(m.Normal.Def)
		start = 1
	case segs[0] == "super":
		pm := m.ParentModule()
		start = 1
		for start < len(segs) && segs[start] == "super" {
			pm = pm.ParentModule()
			start++
		}
		cur = defRes(pm.Def)
	default:
		res, ok, det := r.lookupUseFirst(m, segs[0], TypeNS, imp)
		if !det {
			return Res{}, false
		}
		if !ok {
			if report {
				r.errorf(sp, "unresolved import: cannot find `%s`", segs[0])
			}
			return Res{}, false
		}
		cur = res
		start = 1
	}
	for _, s := range segs[start:] {
		if cur.Def == nil || cur.Def.Mod == nil {
			if report {
				r.errorf(sp, "`%s` is not a module", s)
			}
			return Res{}, false
		}
		b, det := r.lookupDet(cur.Def.Mod, s, TypeNS, imp)
		if !det {
			return Res{}, false
		}
		if b == nil {
			if report {
				r.errorf(sp, "cannot find `%s` in `%s`", s, cur.Def.Name)
			}
			return Res{}, false
		}
		cur = b.Res
	}
	return cur, true
}

// ---------------------------------------------------------------------------
// Lexical lookup

var primTypes = map[string]bool{
	"bool": true, "char": true, "str": true, "i8": true, "i16": true, "i32": true, "i64": true, "i128": true, "isize": true,
	"u8": true, "u16": true, "u32": true, "u64": true, "u128": true, "usize": true, "f32": true, "f64": true,
}

// IsPrimName reports whether s names a primitive type.
func IsPrimName(s string) bool { return primTypes[s] }

func (r *Resolver) lookupLexicalNoPrelude(m *Module, name string, ns Namespace) (Res, bool) {
	for x := m; x != nil; x = x.Parent {
		if b := x.Lookup(name, ns); b != nil {
			return b.Res, true
		}
		if !x.Block {
			break
		}
	}
	return Res{}, false
}

// LookupLexical looks up a single name from module/block scope m: the
// block chain, the enclosing module, extern crates, the std prelude and the
// primitive types.
func (r *Resolver) LookupLexical(m *Module, name string, ns Namespace) (Res, bool) {
	if res, ok := r.lookupLexicalNoPrelude(m, name, ns); ok {
		return res, true
	}
	if ns == TypeNS {
		if cr := r.ExternCrate(m.Crate, name); cr != nil {
			return defRes(cr.Def), true
		}
	}
	if r.Prelude != nil {
		if b := r.Prelude.Lookup(name, ns); b != nil {
			return b.Res, true
		}
	}
	if ns == TypeNS && primTypes[name] {
		return Res{Kind: ResPrim, Prim: name}, true
	}
	return Res{}, false
}

// PathResult is the result of resolving a path: Res for the longest
// resolvable prefix, Consumed is how many segments it covers. If
// Consumed < len(segments), the remaining segments are type-relative
// (associated items) and must be resolved by the type checker.
type PathResult struct {
	Res      Res
	Consumed int
	// Private is set when a segment named an item that is not visible from
	// the module the path appears in (rustc E0603): PrivSeg is the index of
	// the first such segment, PrivRes what it resolved to.
	Private bool
	PrivSeg int
	PrivRes Res
}

// FirstLookup lets the caller override lookup of the first segment (for
// generic params, Self, locals). Return ok=false to fall back.
type FirstLookup func(name string, ns Namespace) (Res, bool)

// ResolvePath resolves segments of a path appearing in scope m. ns is the
// namespace of the final segment.
//
// Like rustc, a path `prim::rest` whose first segment names a primitive
// type but resolves to a module (`use core::char;`) falls back to the
// primitive type when the path fails to resolve or resolves to a module:
// `char::from(b)` is `<char>::from(b)` since `core::char` has no `from`.
func (r *Resolver) ResolvePath(m *Module, segs []string, global bool, ns Namespace, first FirstLookup) (PathResult, error) {
	pr, err := r.resolvePath(m, segs, global, ns, first)
	if !global && len(segs) > 0 && primTypes[segs[0]] && (ns == TypeNS || len(segs) > 1) {
		isMod := err == nil && pr.Consumed == len(segs) && pr.Res.Kind == ResDef && pr.Res.Def != nil &&
			(pr.Res.Def.Kind == DefMod || pr.Res.Def.Kind == DefCrate)
		if err != nil || isMod {
			return PathResult{Res: Res{Kind: ResPrim, Prim: segs[0]}, Consumed: 1}, nil
		}
	}
	return pr, err
}

func (r *Resolver) resolvePath(m *Module, segs []string, global bool, ns Namespace, first FirstLookup) (PathResult, error) {
	if len(segs) == 0 {
		return PathResult{}, fmt.Errorf("empty path")
	}
	var cur Res
	var priv PathResult // privacy violation found so far
	i := 0
	nsFor := func(idx int) Namespace {
		if idx == len(segs)-1 {
			return ns
		}
		return TypeNS
	}
	switch {
	case global && m.Crate.Is2015() && !r.injectedStd(m.Crate, segs[0]):
		cur = defRes(m.Crate.Def)
	case global:
		cr := r.ExternCrate(m.Crate, segs[0])
		if cr == nil {
			return PathResult{}, fmt.Errorf("cannot find crate `%s`", segs[0])
		}
		cur = defRes(cr.Def)
		i = 1
	case segs[0] == "crate":
		cur = defRes(m.Crate.Def)
		i = 1
	case segs[0] == "super":
		pm := m.ParentModule()
		i = 1
		for i < len(segs) && segs[i] == "super" {
			pm = pm.ParentModule()
			i++
		}
		cur = defRes(pm.Def)
	case segs[0] == "self" && len(segs) > 1:
		cur = defRes(m.Normal.Def)
		i = 1
	default:
		var ok bool
		if first != nil {
			cur, ok = first(segs[0], nsFor(0))
		}
		if !ok {
			cur, ok = r.LookupLexical(m, segs[0], nsFor(0))
		}
		if !ok && len(segs) > 1 && nsFor(0) != TypeNS {
			// shouldn't happen: non-final segments use TypeNS
		}
		if !ok {
			// A single-segment path in value NS may name a unit/tuple
			// struct... already covered. Try the type namespace for
			// type-relative paths like `Vec::new` (first seg is a type).
			return PathResult{}, fmt.Errorf("cannot find `%s` in this scope", segs[0])
		}
		i = 1
	}
	for i < len(segs) {
		if cur.Kind != ResDef || cur.Def == nil {
			return priv.with(cur, i), nil
		}
		d := cur.Def
		if segs[i] == "super" && d.Mod != nil && (d.Kind == DefMod || d.Kind == DefCrate) {
			cur = defRes(d.Mod.ParentModule().Def)
			i++
			continue
		}
		switch d.Kind {
		case DefMod, DefCrate:
			b := d.Mod.Lookup(segs[i], nsFor(i))
			if b == nil && nsFor(i) == ValueNS {
				// maybe a type followed by nothing (unit struct in type ns)
				b = d.Mod.Lookup(segs[i], TypeNS)
				if b != nil && i == len(segs)-1 {
					// a type used in value position: only ok for ctors; report
					// the type so the caller can complain or handle `Type::`
					pr := PathResult{Res: b.Res, Consumed: i + 1}
					if priv.Private {
						pr.Private, pr.PrivSeg, pr.PrivRes = true, priv.PrivSeg, priv.PrivRes
					} else if !r.BindingAccessible(d.Mod, segs[i], TypeNS, b, m) {
						pr.Private, pr.PrivSeg, pr.PrivRes = true, i, b.Res
					}
					return pr, nil
				}
			}
			if b == nil {
				// Try the type namespace when looking for a non-final value
				return PathResult{}, fmt.Errorf("cannot find `%s` in `%s`", segs[i], d.Name)
			}
			if !priv.Private && !r.BindingAccessible(d.Mod, segs[i], nsFor(i), b, m) {
				priv.Private, priv.PrivSeg, priv.PrivRes = true, i, b.Res
			}
			cur = b.Res
			i++
		case DefEnum:
			b := d.Mod.Lookup(segs[i], nsFor(i))
			if b == nil {
				b = d.Mod.Lookup(segs[i], TypeNS)
			}
			if b == nil {
				// type-relative (inherent assoc item on the enum)
				return priv.with(cur, i), nil
			}
			cur = b.Res
			i++
		default:
			// structs, traits, aliases, variants, params: the rest is
			// type-relative.
			return priv.with(cur, i), nil
		}
	}
	return priv.with(cur, i), nil
}

func (p PathResult) with(res Res, consumed int) PathResult {
	p.Res, p.Consumed = res, consumed
	return p
}

// BindingAccessible reports whether binding b, found under name in module
// owner, may be named from module from (privacy). Bindings that come from
// glob imports, and everything involving the std crate (whose
// visibilities do not mirror the real std's), count as accessible.
func (r *Resolver) BindingAccessible(owner *Module, name string, ns Namespace, b *NameBinding, from *Module) bool {
	if from == nil || owner == nil || b == nil {
		return true
	}
	if r.Std != nil && (owner.Crate == r.Std || from.Crate == r.Std) {
		return true
	}
	tbl := owner.Types
	if ns == ValueNS {
		tbl = owner.Values
	}
	if tbl[name] != b {
		return true // via a glob import
	}
	if owner.Def != nil && (owner.Def.Kind == DefEnum || owner.Def.Kind == DefTrait) {
		return true // variants and trait items are public
	}
	if !VisibleFrom(b.Vis, owner, from) {
		return false
	}
	// a tuple struct's constructor is as visible as its least visible field
	if b.Res.Kind == ResCtor && b.Res.Def != nil && b.Res.Def.Kind == DefStruct && b.Res.Def.Item != nil {
		if st, ok := b.Res.Def.Item.Kind.(*ast.StructItem); ok {
			for _, f := range st.Fields {
				if !VisibleFrom(f.Vis, owner, from) {
					return false
				}
			}
		}
	}
	return true
}

// GlobAmbiguous reports whether name, looked up lexically from m, is only
// found through two glob imports that bring different items (rustc
// E0659). Only items defined directly in the glob-imported modules (and
// visible from the importing one) are considered; std items are ignored.
func (r *Resolver) GlobAmbiguous(m *Module, name string, ns Namespace) bool {
	for x := m; x != nil; x = x.Parent {
		tbl := x.Types
		if ns == ValueNS {
			tbl = x.Values
		}
		if _, ok := tbl[name]; ok {
			return false
		}
		var found []Res
		for _, g := range x.Globs {
			gt := g.Types
			if ns == ValueNS {
				gt = g.Values
			}
			b, ok := gt[name]
			if !ok || b.Res.Def == nil || (r.Std != nil && b.Res.Def.Crate == r.Std) {
				continue
			}
			pub := g.Def != nil && (g.Def.Kind == DefEnum || g.Def.Kind == DefTrait)
			if !pub && !VisibleFrom(b.Vis, g, x) {
				continue
			}
			dup := false
			for _, f := range found {
				if f == b.Res {
					dup = true
				}
			}
			if !dup {
				found = append(found, b.Res)
			}
		}
		if len(found) > 1 {
			return true
		}
		if len(found) == 1 || x.Lookup(name, ns) != nil || !x.Block {
			return false
		}
	}
	return false
}

// VisibleFrom reports whether something with visibility vis declared in
// module owner is visible from module from: private items in owner and its
// descendants, pub(super) ones in owner's parent and its descendants,
// pub(crate)/pub(in path) ones in the whole crate.
func VisibleFrom(vis ast.Visibility, owner, from *Module) bool {
	if owner == nil || from == nil {
		return true
	}
	switch vis {
	case ast.VisPub:
		return true
	case ast.VisCrate, ast.VisRestricted:
		return owner.Crate == from.Crate
	}
	if owner.Crate != from.Crate {
		return false
	}
	anc := owner.Normal
	if anc == nil {
		anc = owner
	}
	if vis == ast.VisSuper {
		anc = anc.ParentModule()
	}
	for x := from; x != nil; x = x.Parent {
		if x == anc || x.Normal == anc {
			return true
		}
	}
	return false
}

// injectedStd reports whether a 2015 crate-root path `::name` refers to the
// implicitly injected `extern crate std;` (or core/alloc) because the root
// defines nothing else under that name.
func (r *Resolver) injectedStd(c *Crate, name string) bool {
	if name != "std" && name != "core" && name != "alloc" {
		return false
	}
	return c.Root == nil || c.Root.Lookup(name, TypeNS) == nil
}

// ExternCrate looks up a crate name in from's extern prelude.
func (r *Resolver) ExternCrate(from *Crate, name string) *Crate {
	if from != nil && from.Externs != nil {
		if c, ok := from.Externs[name]; ok {
			return c
		}
		if name == from.Name {
			return from
		}
		if name == "std" || name == "core" || name == "alloc" {
			return r.Std
		}
		// `$crate` paths of macros from crates that are not direct
		// dependencies expand to `::name`; allow any loaded crate.
		return r.Crates[name]
	}
	return r.Crates[name]
}

// Lang returns a lang item def or nil.
func (r *Resolver) Lang(name string) *Def { return r.LangItems[name] }

// StdPath resolves an absolute std path like "option::Option".
func (r *Resolver) StdPath(path string) *Def {
	if r.Std == nil {
		return nil
	}
	segs := strings.Split(path, "::")
	cur := r.Std.Root
	for i, s := range segs {
		ns := TypeNS
		b := cur.Lookup(s, ns)
		if b == nil && i == len(segs)-1 {
			b = cur.Lookup(s, ValueNS)
		}
		if b == nil || b.Res.Def == nil {
			return nil
		}
		if i == len(segs)-1 {
			return b.Res.Def
		}
		if b.Res.Def.Mod == nil {
			return nil
		}
		cur = b.Res.Def.Mod
	}
	return nil
}

// addLinkAttr records the library named by a `#[link(name = "...", kind =
// "...")]` attribute on an extern block (Linux: dylib and static kinds;
// frameworks, raw-dylib and cfg-conditional links are ignored).
func (r *Resolver) addLinkAttr(a *ast.Attr) {
	var args []ast.TokenTree
	for _, t := range a.Tokens {
		if t.Delim == '(' {
			args = t.Inner
			break
		}
	}
	var firstStr func(ts []ast.TokenTree) (string, bool)
	firstStr = func(ts []ast.TokenTree) (string, bool) {
		for _, t := range ts {
			if t.Delim == 0 && t.Tok.Kind == lexer.Str {
				return strings.Trim(t.Tok.Text, "\""), true
			}
			if t.Delim != 0 {
				if v, ok := firstStr(t.Inner); ok {
					return v, true
				}
			}
		}
		return "", false
	}
	kv := map[string]string{}
	var seg []ast.TokenTree
	flush := func() {
		if len(seg) > 0 && seg[0].Delim == 0 && seg[0].Tok.Kind == lexer.Ident {
			key := seg[0].Tok.Text
			if v, ok := firstStr(seg[1:]); ok {
				kv[key] = v
			} else {
				kv[key] = ""
			}
		}
		seg = nil
	}
	for _, t := range args {
		if t.Delim == 0 && t.Tok.Kind == lexer.Punct && t.Tok.Text == "," {
			flush()
			continue
		}
		seg = append(seg, t)
	}
	flush()
	name := kv["name"]
	if name == "" {
		return
	}
	if _, ok := kv["cfg"]; ok {
		return
	}
	kind, mods, _ := strings.Cut(kv["kind"], ":")
	if m, ok := kv["modifiers"]; ok {
		mods = m
	}
	var args2 []string
	switch kind {
	case "", "dylib":
		args2 = []string{"-l" + name}
	case "static":
		lib := []string{"-Wl,-Bstatic", "-l" + name, "-Wl,-Bdynamic"}
		if strings.Contains(mods, "+whole-archive") {
			lib = append(append([]string{"-Wl,--whole-archive"}, lib...), "-Wl,--no-whole-archive")
		}
		args2 = lib
	default: // framework, raw-dylib, link-arg: not for Linux targets
		return
	}
	key := strings.Join(args2, " ")
	for _, have := range r.linkSeen {
		if have == key {
			return
		}
	}
	r.linkSeen = append(r.linkSeen, key)
	r.LinkArgs = append(r.LinkArgs, args2...)
}
