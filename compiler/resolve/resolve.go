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
}

func New() *Resolver {
	return &Resolver{
		Crates:    map[string]*Crate{},
		LangItems: map[string]*Def{},
		BlockMods: map[ast.NodeID]*Module{},
		ItemDefs:  map[*ast.Item]*Def{},
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
	if name == "" || name == "_" {
		return
	}
	m.Define(name, ns, &NameBinding{Res: res, Span: sp})
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
		r.define(m, name, ValueNS, defRes(d), it.Name.Span)
		if k.Body != nil {
			r.collectBody(k.Body, m, d)
		}
	case *ast.StructItem:
		d = r.newDef(DefStruct, name, parent)
		d.Adt = &AdtDef{Kind: AdtStruct}
		r.define(m, name, TypeNS, defRes(d), it.Name.Span)
		if k.Shape != ast.ShapeNamed {
			r.define(m, name, ValueNS, Res{Kind: ResCtor, Def: d}, it.Name.Span)
		}
	case *ast.UnionItem:
		d = r.newDef(DefUnion, name, parent)
		d.Adt = &AdtDef{Kind: AdtUnion}
		r.define(m, name, TypeNS, defRes(d), it.Name.Span)
	case *ast.EnumItem:
		d = r.newDef(DefEnum, name, parent)
		d.Adt = &AdtDef{Kind: AdtEnum}
		d.Mod = NewModule(d, m, m.Crate, false)
		d.Mod.Normal = d.Mod
		r.define(m, name, TypeNS, defRes(d), it.Name.Span)
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
		r.define(m, name, TypeNS, defRes(d), it.Name.Span)
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
		r.define(m, name, TypeNS, defRes(d), it.Name.Span)
	case *ast.ConstItem:
		d = r.newDef(DefConst, name, parent)
		d.Const = &ConstInfo{Expr: k.Expr, TyAst: k.Ty}
		r.define(m, name, ValueNS, defRes(d), it.Name.Span)
		if k.Expr != nil {
			r.collectBody(k.Expr, m, d)
		}
	case *ast.StaticItem:
		d = r.newDef(DefStatic, name, parent)
		d.Const = &ConstInfo{Expr: k.Expr, TyAst: k.Ty, Mut: k.Mut, Static: true}
		r.define(m, name, ValueNS, defRes(d), it.Name.Span)
		if k.Expr != nil {
			r.collectBody(k.Expr, m, d)
		}
	case *ast.ModItem:
		d = r.newDef(DefMod, name, parent)
		d.Mod = NewModule(d, m, m.Crate, false)
		r.define(m, name, TypeNS, defRes(d), it.Name.Span)
		for _, sub := range k.Items {
			r.collectItem(sub, d.Mod, d)
		}
	case *ast.UseItem:
		r.collectUse(k.Tree, nil, false, m)
		return nil
	case *ast.ExternCrateItem:
		bind := name
		if k.Rename != nil {
			bind = k.Rename.Name
		}
		if cr, ok := r.Crates[name]; ok {
			r.define(m, bind, TypeNS, defRes(cr.Def), it.Name.Span)
		} else if name == "std" || name == "core" || name == "alloc" {
			// std may not be loaded yet if we are std itself; resolve lazily via path.
			m.Imports = append(m.Imports, &Import{Module: m, Path: []string{name}, Global: true, Name: bind, Span: it.Span})
		}
		return nil
	case *ast.ExternBlockItem:
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
func (r *Resolver) collectUse(t *ast.UseTree, prefix []string, global bool, m *Module) {
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
		m.Imports = append(m.Imports, &Import{Module: m, Path: path, Global: global, Name: name, Span: t.Span})
	case ast.UseGlob:
		m.Imports = append(m.Imports, &Import{Module: m, Path: segs, Global: global, Glob: true, Span: t.Span})
	case ast.UseNested:
		for _, n := range t.Nested {
			r.collectUse(n, segs, global, m)
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
		if !seen[bm] {
			seen[bm] = true
			out = append(out, bm)
		}
	}
	return out
}

// ResolveImports resolves all `use` declarations to a fixpoint.
func (r *Resolver) ResolveImports() {
	if r.Std != nil {
		if pm := r.Std.Root.Lookup("prelude", TypeNS); pm != nil && pm.Res.Def != nil && pm.Res.Def.Mod != nil {
			r.Prelude = pm.Res.Def.Mod
		}
	}
	mods := r.allModules()
	for {
		progress := false
		pending := 0
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
		if pending == 0 || !progress {
			break
		}
	}
	for _, m := range mods {
		for _, imp := range m.Imports {
			if !imp.Done {
				r.tryImport(imp, true)
			}
		}
	}
	// prelude may be defined via imports
	if r.Std != nil && r.Prelude == nil {
		if pm := r.Std.Root.Lookup("prelude", TypeNS); pm != nil && pm.Res.Def != nil && pm.Res.Def.Mod != nil {
			r.Prelude = pm.Res.Def.Mod
		}
	}
}

func (r *Resolver) tryImport(imp *Import, report bool) bool {
	m := imp.Module
	if imp.Glob {
		res, ok := r.resolveModulePath(m, imp.Path, imp.Global, report, imp.Span)
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
		return true
	}
	// Single import: resolve prefix as module, then look up last name.
	if len(imp.Path) == 1 {
		name := imp.Path[0]
		// `use std;` or `use foo;` (crate) or `use self::x` style single names
		if cr, ok := r.Crates[name]; ok {
			r.define(m, imp.Name, TypeNS, defRes(cr.Def), imp.Span)
			m.Types[imp.Name].Import = true
			return true
		}
		if imp.Global {
			if report {
				r.errorf(imp.Span, "unresolved import `%s`", name)
			}
			return report
		}
		found := false
		for _, ns := range []Namespace{TypeNS, ValueNS} {
			if res, ok := r.lookupLexicalNoPrelude(m.Parent, name, ns); ok && m.Parent != nil {
				r.bindImport(m, imp.Name, ns, res, imp.Span)
				found = true
			}
		}
		if !found && report {
			r.errorf(imp.Span, "unresolved import `%s`", name)
		}
		return found || report
	}
	prefix := imp.Path[:len(imp.Path)-1]
	last := imp.Path[len(imp.Path)-1]
	res, ok := r.resolveModulePath(m, prefix, imp.Global, report, imp.Span)
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
	found := false
	for _, ns := range []Namespace{TypeNS, ValueNS} {
		if b := target.Lookup(last, ns); b != nil {
			r.bindImport(m, imp.Name, ns, b.Res, imp.Span)
			found = true
		}
	}
	if !found {
		if report {
			r.errorf(imp.Span, "unresolved import `%s`: no `%s` in `%s`", strings.Join(imp.Path, "::"), last, strings.Join(prefix, "::"))
			return true
		}
		return false
	}
	return true
}

func (r *Resolver) bindImport(m *Module, name string, ns Namespace, res Res, sp lexer.Span) {
	if name == "_" {
		if res.Def != nil && res.Def.Kind == DefTrait {
			m.AnonTraits = append(m.AnonTraits, res.Def)
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
	if old, ok := tbl[name]; ok && !old.Import {
		return // items shadow imports
	}
	tbl[name] = &NameBinding{Res: res, Import: true, Span: sp}
}

// resolveModulePath resolves a path whose every segment must be a module-like
// (module, crate, enum, trait).
func (r *Resolver) resolveModulePath(m *Module, segs []string, global bool, report bool, sp lexer.Span) (Res, bool) {
	if len(segs) == 0 {
		return defRes(m.Normal.Def), true
	}
	var cur Res
	start := 0
	switch {
	case global:
		cr, ok := r.Crates[segs[0]]
		if !ok {
			if report {
				r.errorf(sp, "unresolved crate `%s`", segs[0])
			}
			return Res{}, false
		}
		cur = defRes(cr.Def)
		start = 1
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
		res, ok := r.LookupLexical(m, segs[0], TypeNS)
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
		b := cur.Def.Mod.Lookup(s, TypeNS)
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
		if cr, ok := r.Crates[name]; ok {
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
}

// FirstLookup lets the caller override lookup of the first segment (for
// generic params, Self, locals). Return ok=false to fall back.
type FirstLookup func(name string, ns Namespace) (Res, bool)

// ResolvePath resolves segments of a path appearing in scope m. ns is the
// namespace of the final segment.
func (r *Resolver) ResolvePath(m *Module, segs []string, global bool, ns Namespace, first FirstLookup) (PathResult, error) {
	if len(segs) == 0 {
		return PathResult{}, fmt.Errorf("empty path")
	}
	var cur Res
	i := 0
	nsFor := func(idx int) Namespace {
		if idx == len(segs)-1 {
			return ns
		}
		return TypeNS
	}
	switch {
	case global:
		cr, ok := r.Crates[segs[0]]
		if !ok {
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
			return PathResult{Res: cur, Consumed: i}, nil
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
					return PathResult{Res: b.Res, Consumed: i + 1}, nil
				}
			}
			if b == nil {
				// Try the type namespace when looking for a non-final value
				return PathResult{}, fmt.Errorf("cannot find `%s` in `%s`", segs[i], d.Name)
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
				return PathResult{Res: cur, Consumed: i}, nil
			}
			cur = b.Res
			i++
		default:
			// structs, traits, aliases, variants, params: the rest is
			// type-relative.
			return PathResult{Res: cur, Consumed: i}, nil
		}
	}
	return PathResult{Res: cur, Consumed: i}, nil
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
