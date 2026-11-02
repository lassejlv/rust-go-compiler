// Package expand performs macro expansion on the AST (mirrors
// rustc_expand + rustc_builtin_macros): macro_rules! definitions and
// invocations, builtin macros (println!, format!, vec!, assert!, ...),
// #[derive], #[cfg] / #[cfg_attr] stripping and removal of #[test] items.
package expand

import (
	"reflect"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
)

// DefaultRecursionLimit is the maximum macro expansion depth.
const DefaultRecursionLimit = 128

// Expander expands macros crate by crate. Macros defined in a crate stay
// visible to crates expanded later with the same Expander (e.g. std's
// macros are visible in the user crate).
type Expander struct {
	// RecursionLimit bounds nested macro expansion.
	RecursionLimit int
	// Config is the cfg configuration of the crate being expanded (set
	// before each ExpandCrate call).
	Config *CrateConfig
	// ProcMacros are the procedural macros visible to the crate being
	// expanded, by name.
	ProcMacros map[string]*ProcMacro
	// Externs maps the extern crate names of the crate being expanded to
	// crate names as given to ExpandCrate (for path-qualified macro calls
	// such as `dep::m!`); names not listed are taken as crate names.
	Externs map[string]string

	crate      string
	crateOrder []string
	macros     map[string]map[string]*macroDef // crate -> name -> def
	modPath    []string
	// loadMod/modDir load out-of-line modules produced by macro expansion
	// (see ast.Crate.LoadMod); modDir is where the current module's
	// children live.
	loadMod func(it *ast.Item, dir string) (string, error)
	modDir  string
	site    *lexer.Span // outermost user-macro call site, for line!/file!/panic locations
	assoc   bool        // expanding trait/impl items (item macros parse as associated items)
	// aliases maps names introduced by renaming imports (`use m as n;`) to
	// the imported name, per crate; consulted for macro names only.
	aliases map[string]map[string]string
}

// New creates an Expander.
func New() *Expander {
	return &Expander{
		RecursionLimit: DefaultRecursionLimit,
		macros:         map[string]map[string]*macroDef{},
		aliases:        map[string]map[string]string{},
	}
}

// ExpandCrate expands all macros of c in place. crateName is "std" for the
// standard library and the crate's name (e.g. "main") otherwise.
func (e *Expander) ExpandCrate(c *ast.Crate, crateName string) error {
	applyCrateConfig(e.Config)
	e.crate = crateName
	if _, ok := e.macros[crateName]; !ok {
		e.macros[crateName] = map[string]*macroDef{}
		e.crateOrder = append(e.crateOrder, crateName)
	}
	e.modPath = []string{crateName}
	e.site = nil
	e.loadMod, e.modDir = c.LoadMod, c.Dir
	if e.loadMod != nil {
		if err := e.loadModules(c.Items, c.Dir); err != nil {
			return err
		}
	}
	// Pre-pass: collect all macro_rules! definitions so that definition order
	// does not matter.
	e.collectItems(c.Items)
	items, err := e.expandItems(c.Items, 0)
	if err != nil {
		return err
	}
	c.Items = items
	return desugarAsync(c)
}

// ---------------------------------------------------------------------------
// Macro definition pre-pass

// loadModules loads the out-of-line modules whose existence or file
// depends on the crate configuration (`#[cfg(..)] mod m;`,
// `#[cfg_attr(unix, path = "unix.rs")] mod sys;`), which the driver leaves
// to the expander, so that the macro pre-pass sees their items.
func (e *Expander) loadModules(items []*ast.Item, dir string) error {
	for _, it := range items {
		k, ok := it.Kind.(*ast.ModItem)
		if !ok {
			continue
		}
		keep, attrs, err := e.processAttrs(it.Attrs)
		if err != nil {
			return err
		}
		if !keep {
			continue // stripped by expandItemInList
		}
		it.Attrs = attrs
		child, err := e.loadMod(it, dir)
		if err != nil {
			return err
		}
		// the loaded file's inner attributes (`#![cfg(..)]`)
		if keep, attrs, err = e.processAttrs(it.Attrs); err != nil {
			return err
		}
		if !keep {
			continue
		}
		it.Attrs = attrs
		if err := e.loadModules(k.Items, child); err != nil {
			return err
		}
	}
	return nil
}

func (e *Expander) collectItems(items []*ast.Item) {
	for _, it := range items {
		keep, attrs, err := e.processAttrs(it.Attrs)
		if err != nil || !keep {
			continue
		}
		switch k := it.Kind.(type) {
		case *ast.MacroRulesItem:
			if def, err := parseMacroRules(it.Name.Name, k.Body, e.crate, it.Span); err == nil {
				def.exported = hasAttr(attrs, "macro_export")
				e.defineMacro(def)
			}
		case *ast.UseItem:
			e.collectUseAliases(k.Tree)
		case *ast.ModItem:
			e.collectItems(k.Items)
		case *ast.FnItem:
			if k.Body != nil {
				e.collectBlock(k.Body)
			}
		case *ast.ImplItem:
			e.collectItems(k.Items)
		case *ast.TraitItem:
			e.collectItems(k.Items)
		}
	}
}

// collectUseAliases records renaming imports (`use a::m as n;`) so that a
// macro imported under another name can be invoked by that name.
func (e *Expander) collectUseAliases(t *ast.UseTree) {
	if t == nil {
		return
	}
	switch t.Kind {
	case ast.UseSimple:
		if t.Rename == nil || t.Rename.Name == "_" || t.Prefix == nil || len(t.Prefix.Segments) == 0 {
			return
		}
		orig := t.Prefix.Segments[len(t.Prefix.Segments)-1].Ident.Name
		if orig == t.Rename.Name {
			return
		}
		if e.aliases[e.crate] == nil {
			e.aliases[e.crate] = map[string]string{}
		}
		e.aliases[e.crate][t.Rename.Name] = orig
	case ast.UseNested:
		for _, n := range t.Nested {
			e.collectUseAliases(n)
		}
	}
}

func (e *Expander) collectBlock(b *ast.Block) {
	for _, s := range b.Stmts {
		switch s := s.(type) {
		case *ast.ItemStmt:
			e.collectItems([]*ast.Item{s.Item})
		case *ast.ExprStmt:
			if be, ok := s.Expr.(*ast.BlockExpr); ok {
				e.collectBlock(be.Block)
			}
		}
	}
}

func (e *Expander) defineMacro(def *macroDef) {
	e.macros[e.crate][def.name] = def
}

// ---------------------------------------------------------------------------
// Items

func (e *Expander) checkDepth(mac *ast.MacCall, depth int) error {
	if depth > e.RecursionLimit {
		return errAt(mac.Span, "recursion limit reached while expanding `%s!` (limit %d)", macName(mac), e.RecursionLimit)
	}
	return nil
}

// withSite runs f with the expansion call site set (if not already set).
func (e *Expander) withSite(sp lexer.Span, f func() error) error {
	if e.site != nil {
		return f()
	}
	e.site = &sp
	defer func() { e.site = nil }()
	return f()
}

func (e *Expander) callSite(sp lexer.Span) lexer.Span {
	if e.site != nil {
		return *e.site
	}
	return sp
}

func hasAttr(attrs []*ast.Attr, name string) bool {
	for _, a := range attrs {
		if a.Path == name {
			return true
		}
	}
	return false
}

// expandItems expands a list of items (module, impl, trait, extern block).
func (e *Expander) expandItems(items []*ast.Item, depth int) ([]*ast.Item, error) {
	out := make([]*ast.Item, 0, len(items))
	for _, it := range items {
		res, err := e.expandItemInList(it, depth)
		if err != nil {
			return nil, err
		}
		out = append(out, res...)
	}
	return out, nil
}

// expandAssocItems expands the items of a trait or impl.
func (e *Expander) expandAssocItems(items []*ast.Item, depth int) ([]*ast.Item, error) {
	saved := e.assoc
	e.assoc = true
	defer func() { e.assoc = saved }()
	return e.expandItems(items, depth)
}

// expandItemInList expands one item and returns its replacement(s): nothing
// for cfg'd-out items, the expansion for macro calls, the item followed by
// derived impls otherwise.
func (e *Expander) expandItemInList(it *ast.Item, depth int) ([]*ast.Item, error) {
	keep, attrs, err := e.processAttrs(it.Attrs)
	if err != nil {
		return nil, err
	}
	if !keep || hasAttr(attrs, "test") || hasAttr(attrs, "bench") {
		return nil, nil
	}
	it.Attrs = attrs
	switch k := it.Kind.(type) {
	case *ast.MacroRulesItem:
		def, err := parseMacroRules(it.Name.Name, k.Body, e.crate, it.Span)
		if err != nil {
			return nil, err
		}
		def.exported = hasAttr(it.Attrs, "macro_export")
		e.defineMacro(def)
		return nil, nil
	case *ast.MacCallItem:
		if err := e.checkDepth(k.Mac, depth); err != nil {
			return nil, err
		}
		items, user, err := e.expandItemMac(k.Mac)
		if err != nil {
			return nil, err
		}
		var res []*ast.Item
		f := func() error {
			res, err = e.expandItems(items, depth+1)
			return err
		}
		if user {
			err = e.withSite(k.Mac.Span, f)
		} else {
			err = f()
		}
		return res, err
	}
	if res, ok, err := e.procAttr(it, depth); ok || err != nil {
		return res, err
	}
	if err := e.expandItem(it, depth); err != nil {
		return nil, err
	}
	derived, err := e.deriveItem(it)
	if err != nil {
		return nil, err
	}
	return append([]*ast.Item{it}, derived...), nil
}

func (e *Expander) expandItem(it *ast.Item, depth int) error {
	var err error
	switch k := it.Kind.(type) {
	case *ast.FnItem:
		if err = e.expandGenerics(k.Generics, depth); err != nil {
			return err
		}
		if err = e.expandFnDecl(k.Decl, depth); err != nil {
			return err
		}
		if k.Body != nil {
			saved := e.assoc
			e.assoc = false
			defer func() { e.assoc = saved }()
			return e.expandBlock(k.Body, depth)
		}
	case *ast.StructItem:
		if err = e.expandGenerics(k.Generics, depth); err != nil {
			return err
		}
		k.Fields, err = e.expandFields(k.Fields, depth)
		return err
	case *ast.UnionItem:
		if err = e.expandGenerics(k.Generics, depth); err != nil {
			return err
		}
		k.Fields, err = e.expandFields(k.Fields, depth)
		return err
	case *ast.EnumItem:
		if err = e.expandGenerics(k.Generics, depth); err != nil {
			return err
		}
		vs := k.Variants[:0]
		for _, v := range k.Variants {
			keep, attrs, err := e.processAttrs(v.Attrs)
			if err != nil {
				return err
			}
			if !keep {
				continue
			}
			v.Attrs = attrs
			if v.Fields, err = e.expandFields(v.Fields, depth); err != nil {
				return err
			}
			if v.Disc != nil {
				if v.Disc, err = e.expandExpr(v.Disc, depth); err != nil {
					return err
				}
			}
			vs = append(vs, v)
		}
		k.Variants = vs
	case *ast.TraitItem:
		if err = e.expandGenerics(k.Generics, depth); err != nil {
			return err
		}
		if err = e.expandBounds(k.Supertraits, depth); err != nil {
			return err
		}
		k.Items, err = e.expandAssocItems(k.Items, depth)
		return err
	case *ast.ImplItem:
		if err = e.expandGenerics(k.Generics, depth); err != nil {
			return err
		}
		if k.Trait != nil {
			if err = e.expandPath(k.Trait, depth); err != nil {
				return err
			}
		}
		if k.SelfTy, err = e.expandType(k.SelfTy, depth); err != nil {
			return err
		}
		k.Items, err = e.expandAssocItems(k.Items, depth)
		return err
	case *ast.TypeAliasItem:
		if err = e.expandGenerics(k.Generics, depth); err != nil {
			return err
		}
		if err = e.expandBounds(k.Bounds, depth); err != nil {
			return err
		}
		k.Ty, err = e.expandType(k.Ty, depth)
		return err
	case *ast.ConstItem:
		if k.Ty, err = e.expandType(k.Ty, depth); err != nil {
			return err
		}
		k.Expr, err = e.expandExpr(k.Expr, depth)
		return err
	case *ast.StaticItem:
		if k.Ty, err = e.expandType(k.Ty, depth); err != nil {
			return err
		}
		k.Expr, err = e.expandExpr(k.Expr, depth)
		return err
	case *ast.ModItem:
		savedDir := e.modDir
		if e.loadMod != nil {
			// `mod m;` produced by a macro has not been loaded yet.
			loaded := k.Inline || k.Loaded
			if e.modDir, err = e.loadMod(it, e.modDir); err != nil {
				return err
			}
			if !k.Inline && !k.Loaded {
				return errAt(it.Span, "file not found for module `%s`", it.Name.Name)
			}
			if !loaded {
				// the file's inner attributes (`#![cfg(..)]`)
				keep, attrs, err := e.processAttrs(it.Attrs)
				if err != nil {
					return err
				}
				it.Attrs = attrs
				if !keep {
					k.Items = nil
				}
				e.collectItems(k.Items)
			}
		}
		e.modPath = append(e.modPath, it.Name.Name)
		k.Items, err = e.expandItems(k.Items, depth)
		e.modPath = e.modPath[:len(e.modPath)-1]
		e.modDir = savedDir
		return err
	case *ast.ExternBlockItem:
		k.Items, err = e.expandItems(k.Items, depth)
		return err
	}
	return nil
}

func (e *Expander) expandFields(fields []*ast.FieldDef, depth int) ([]*ast.FieldDef, error) {
	out := fields[:0]
	for _, f := range fields {
		keep, attrs, err := e.processAttrs(f.Attrs)
		if err != nil {
			return nil, err
		}
		if !keep {
			continue
		}
		f.Attrs = attrs
		if f.Ty, err = e.expandType(f.Ty, depth); err != nil {
			return nil, err
		}
		out = append(out, f)
	}
	return out, nil
}

func (e *Expander) expandFnDecl(d *ast.FnDecl, depth int) error {
	var err error
	if d == nil {
		return nil
	}
	if d.SelfParam != nil && d.SelfParam.Ty != nil {
		if d.SelfParam.Ty, err = e.expandType(d.SelfParam.Ty, depth); err != nil {
			return err
		}
	}
	ps := d.Params[:0]
	for _, p := range d.Params {
		keep, attrs, err := e.processAttrs(p.Attrs)
		if err != nil {
			return err
		}
		if !keep {
			continue
		}
		p.Attrs = attrs
		if p.Pat, err = e.expandPat(p.Pat, depth); err != nil {
			return err
		}
		if p.Ty, err = e.expandType(p.Ty, depth); err != nil {
			return err
		}
		ps = append(ps, p)
	}
	d.Params = ps
	d.Output, err = e.expandType(d.Output, depth)
	return err
}

func (e *Expander) expandGenerics(g *ast.Generics, depth int) error {
	if g == nil {
		return nil
	}
	var err error
	ps := g.Params[:0]
	for _, p := range g.Params {
		keep, attrs, err := e.processAttrs(p.Attrs)
		if err != nil {
			return err
		}
		if !keep {
			continue
		}
		p.Attrs = attrs
		if err = e.expandBounds(p.Bounds, depth); err != nil {
			return err
		}
		if p.Default, err = e.expandType(p.Default, depth); err != nil {
			return err
		}
		if p.ConstTy, err = e.expandType(p.ConstTy, depth); err != nil {
			return err
		}
		if p.ConstDefault, err = e.expandExpr(p.ConstDefault, depth); err != nil {
			return err
		}
		ps = append(ps, p)
	}
	g.Params = ps
	for _, w := range g.Where {
		if w.Ty, err = e.expandType(w.Ty, depth); err != nil {
			return err
		}
		if err = e.expandBounds(w.Bounds, depth); err != nil {
			return err
		}
	}
	return nil
}

func (e *Expander) expandBounds(bs []*ast.Bound, depth int) error {
	for _, b := range bs {
		if b.Path != nil {
			if err := e.expandPath(b.Path, depth); err != nil {
				return err
			}
		}
	}
	return nil
}

func (e *Expander) expandPath(p *ast.Path, depth int) error {
	if p == nil {
		return nil
	}
	for _, s := range p.Segments {
		if err := e.expandGenericArgs(s.Args, depth); err != nil {
			return err
		}
	}
	return nil
}

func (e *Expander) expandGenericArgs(ga *ast.GenericArgs, depth int) error {
	if ga == nil {
		return nil
	}
	var err error
	for i := range ga.Args {
		a := &ga.Args[i]
		if a.Type != nil {
			if a.Type, err = e.expandType(a.Type, depth); err != nil {
				return err
			}
		}
		if a.Const != nil {
			if a.Const, err = e.expandExpr(a.Const, depth); err != nil {
				return err
			}
		}
	}
	for _, b := range ga.Bindings {
		if b.Ty, err = e.expandType(b.Ty, depth); err != nil {
			return err
		}
		if err = e.expandBounds(b.Bounds, depth); err != nil {
			return err
		}
	}
	for i, t := range ga.Inputs {
		if ga.Inputs[i], err = e.expandType(t, depth); err != nil {
			return err
		}
	}
	ga.Output, err = e.expandType(ga.Output, depth)
	return err
}

func (e *Expander) expandQSelf(q *ast.QSelf, depth int) error {
	if q == nil {
		return nil
	}
	var err error
	q.Ty, err = e.expandType(q.Ty, depth)
	return err
}

// ---------------------------------------------------------------------------
// Types

func (e *Expander) expandType(t ast.Type, depth int) (ast.Type, error) {
	if t == nil {
		return nil, nil
	}
	var err error
	switch t := t.(type) {
	case *ast.PathType:
		if err = e.expandQSelf(t.QSelf, depth); err != nil {
			return nil, err
		}
		err = e.expandPath(t.Path, depth)
	case *ast.RefType:
		t.Elem, err = e.expandType(t.Elem, depth)
	case *ast.PtrType:
		t.Elem, err = e.expandType(t.Elem, depth)
	case *ast.SliceType:
		t.Elem, err = e.expandType(t.Elem, depth)
	case *ast.ArrayType:
		if t.Elem, err = e.expandType(t.Elem, depth); err != nil {
			return nil, err
		}
		t.Len, err = e.expandExpr(t.Len, depth)
	case *ast.TupleType:
		for i, el := range t.Elems {
			if t.Elems[i], err = e.expandType(el, depth); err != nil {
				return nil, err
			}
		}
	case *ast.FnPtrType:
		for i, el := range t.Inputs {
			if t.Inputs[i], err = e.expandType(el, depth); err != nil {
				return nil, err
			}
		}
		t.Output, err = e.expandType(t.Output, depth)
	case *ast.ImplTraitType:
		err = e.expandBounds(t.Bounds, depth)
	case *ast.DynTraitType:
		err = e.expandBounds(t.Bounds, depth)
	case *ast.ParenType:
		t.Elem, err = e.expandType(t.Elem, depth)
	case *ast.MacroType:
		if err := e.checkDepth(t.Mac, depth); err != nil {
			return nil, err
		}
		res, user, err := e.expandTypeMac(t.Mac)
		if err != nil {
			return nil, err
		}
		var out ast.Type
		f := func() error {
			out, err = e.expandType(res, depth+1)
			return err
		}
		if user {
			err = e.withSite(t.Mac.Span, f)
		} else {
			err = f()
		}
		return out, err
	}
	if err != nil {
		return nil, err
	}
	return t, nil
}

// ---------------------------------------------------------------------------
// Patterns

func (e *Expander) expandPats(ps []ast.Pat, depth int) error {
	var err error
	for i, p := range ps {
		if ps[i], err = e.expandPat(p, depth); err != nil {
			return err
		}
	}
	return nil
}

func (e *Expander) expandPat(p ast.Pat, depth int) (ast.Pat, error) {
	if p == nil {
		return nil, nil
	}
	var err error
	switch p := p.(type) {
	case *ast.IdentPat:
		p.Sub, err = e.expandPat(p.Sub, depth)
	case *ast.LitPat:
		p.Expr, err = e.expandExpr(p.Expr, depth)
	case *ast.RangePat:
		if p.Lo, err = e.expandExpr(p.Lo, depth); err != nil {
			return nil, err
		}
		p.Hi, err = e.expandExpr(p.Hi, depth)
	case *ast.TuplePat:
		err = e.expandPats(p.Elems, depth)
	case *ast.TupleStructPat:
		if err = e.expandQSelf(p.QSelf, depth); err != nil {
			return nil, err
		}
		if err = e.expandPath(p.Path, depth); err != nil {
			return nil, err
		}
		err = e.expandPats(p.Elems, depth)
	case *ast.StructPat:
		if err = e.expandQSelf(p.QSelf, depth); err != nil {
			return nil, err
		}
		if err = e.expandPath(p.Path, depth); err != nil {
			return nil, err
		}
		for _, f := range p.Fields {
			if f.Pat, err = e.expandPat(f.Pat, depth); err != nil {
				return nil, err
			}
		}
	case *ast.PathPat:
		if err = e.expandQSelf(p.QSelf, depth); err != nil {
			return nil, err
		}
		err = e.expandPath(p.Path, depth)
	case *ast.RefPat:
		p.Pat, err = e.expandPat(p.Pat, depth)
	case *ast.BoxPat:
		p.Pat, err = e.expandPat(p.Pat, depth)
	case *ast.SlicePat:
		err = e.expandPats(p.Elems, depth)
	case *ast.OrPat:
		err = e.expandPats(p.Alts, depth)
	case *ast.ParenPat:
		p.Pat, err = e.expandPat(p.Pat, depth)
	case *ast.MacPat:
		if err := e.checkDepth(p.Mac, depth); err != nil {
			return nil, err
		}
		res, user, err := e.expandPatMac(p.Mac)
		if err != nil {
			return nil, err
		}
		var out ast.Pat
		f := func() error {
			out, err = e.expandPat(res, depth+1)
			return err
		}
		if user {
			err = e.withSite(p.Mac.Span, f)
		} else {
			err = f()
		}
		return out, err
	}
	if err != nil {
		return nil, err
	}
	return p, nil
}

// ---------------------------------------------------------------------------
// Blocks and statements

func (e *Expander) expandBlock(b *ast.Block, depth int) error {
	if b == nil {
		return nil
	}
	stmts, err := e.expandStmts(b.Stmts, depth)
	if err != nil {
		return err
	}
	b.Stmts = stmts
	return nil
}

func (e *Expander) expandStmts(stmts []ast.Stmt, depth int) ([]ast.Stmt, error) {
	// Register macro_rules! defined in this block up front so that uses
	// earlier in the block (or in nested blocks) work.
	for _, s := range stmts {
		if is, ok := s.(*ast.ItemStmt); ok {
			if mr, ok := is.Item.Kind.(*ast.MacroRulesItem); ok {
				if keep, _, err := e.processAttrs(is.Item.Attrs); err == nil && keep {
					if def, err := parseMacroRules(is.Item.Name.Name, mr.Body, e.crate, is.Item.Span); err == nil {
						e.defineMacro(def)
					}
				}
			}
		}
	}
	out := make([]ast.Stmt, 0, len(stmts))
	emptySemi := -1 // len(out) when a `m!();` expanded to nothing
	for i, s := range stmts {
		last := i == len(stmts)-1
		switch s := s.(type) {
		case *ast.LetStmt:
			keep, attrs, err := e.processAttrs(s.Attrs)
			if err != nil {
				return nil, err
			}
			if !keep {
				continue
			}
			s.Attrs = attrs
			if s.Pat, err = e.expandPat(s.Pat, depth); err != nil {
				return nil, err
			}
			if s.Ty, err = e.expandType(s.Ty, depth); err != nil {
				return nil, err
			}
			if s.Init, err = e.expandExpr(s.Init, depth); err != nil {
				return nil, err
			}
			if err = e.expandBlock(s.Else, depth); err != nil {
				return nil, err
			}
			out = append(out, s)
		case *ast.ExprStmt:
			keep, err := e.exprKept(s.Expr)
			if err != nil {
				return nil, err
			}
			if !keep {
				continue
			}
			if s.Expr, err = e.expandExpr(s.Expr, depth); err != nil {
				return nil, err
			}
			out = append(out, s)
		case *ast.ItemStmt:
			items, err := e.expandItemInList(s.Item, depth)
			if err != nil {
				return nil, err
			}
			for _, it := range items {
				out = append(out, &ast.ItemStmt{Item: it})
			}
		case *ast.MacStmt:
			keep, attrs, err := e.processAttrs(s.Attrs)
			if err != nil {
				return nil, err
			}
			if !keep {
				continue
			}
			s.Attrs = attrs
			res, err := e.expandMacStmt(s, last, depth)
			if err != nil {
				return nil, err
			}
			if len(res) == 0 && s.Semi && !s.ImplicitSemi {
				// `m!();` expanding to nothing still ends the block
				// with a statement
				emptySemi = len(out)
			}
			out = append(out, res...)
		default:
			out = append(out, s)
		}
	}
	// `#[cfg(a)] { x } #[cfg(b)] { y }`: when the statements after a
	// block-like expression were all removed, it is the block's tail
	if len(out) > 0 && len(out) < len(stmts) && emptySemi != len(out) {
		if es, ok := out[len(out)-1].(*ast.ExprStmt); ok && es.ImplicitSemi && out[len(out)-1] != stmts[len(stmts)-1] {
			es.Semi = false
			es.ImplicitSemi = false
		}
	}
	return out, nil
}

func (e *Expander) expandMacStmt(s *ast.MacStmt, last bool, depth int) ([]ast.Stmt, error) {
	if err := e.checkDepth(s.Mac, depth); err != nil {
		return nil, err
	}
	stmts, user, err := e.expandStmtMac(s.Mac, s.Semi)
	if err != nil {
		return nil, err
	}
	var out []ast.Stmt
	f := func() error {
		out, err = e.expandStmts(stmts, depth+1)
		return err
	}
	if user {
		err = e.withSite(s.Mac.Span, f)
	} else {
		err = f()
	}
	if err != nil {
		return nil, err
	}
	if s.Semi && len(out) > 0 {
		if es, ok := out[len(out)-1].(*ast.ExprStmt); ok {
			es.Semi = true
			// a braced `m! { .. }` statement: may still become the tail
			// if what follows it is configured out or expands to nothing
			es.ImplicitSemi = s.ImplicitSemi
		}
	}
	return out, nil
}

// ---------------------------------------------------------------------------
// Expressions

func exprBase(x ast.Expr) *ast.ExprBase {
	v := reflect.ValueOf(x)
	if v.Kind() != reflect.Pointer || v.IsNil() {
		return nil
	}
	f := v.Elem().FieldByName("ExprBase")
	if !f.IsValid() {
		return nil
	}
	return f.Addr().Interface().(*ast.ExprBase)
}

// exprKept evaluates #[cfg] attributes on an expression; it also applies
// #[cfg_attr].
func (e *Expander) exprKept(x ast.Expr) (bool, error) {
	b := exprBase(x)
	if b == nil || len(b.Attrs) == 0 {
		return true, nil
	}
	keep, attrs, err := e.processAttrs(b.Attrs)
	if err != nil {
		return false, err
	}
	b.Attrs = attrs
	return keep, nil
}

// expandExprList expands a list of expressions, dropping cfg'd-out ones.
func (e *Expander) expandExprList(xs []ast.Expr, depth int) ([]ast.Expr, error) {
	out := xs[:0]
	for _, x := range xs {
		keep, err := e.exprKept(x)
		if err != nil {
			return nil, err
		}
		if !keep {
			continue
		}
		if x, err = e.expandExpr(x, depth); err != nil {
			return nil, err
		}
		out = append(out, x)
	}
	return out, nil
}

func (e *Expander) expandExpr(x ast.Expr, depth int) (ast.Expr, error) {
	if x == nil {
		return nil, nil
	}
	var err error
	switch x := x.(type) {
	case *ast.PathExpr:
		if err = e.expandQSelf(x.QSelf, depth); err != nil {
			return nil, err
		}
		err = e.expandPath(x.Path, depth)
	case *ast.UnaryExpr:
		x.X, err = e.expandExpr(x.X, depth)
	case *ast.BinaryExpr:
		if x.L, err = e.expandExpr(x.L, depth); err != nil {
			return nil, err
		}
		x.R, err = e.expandExpr(x.R, depth)
	case *ast.AssignExpr:
		if x.L, err = e.expandExpr(x.L, depth); err != nil {
			return nil, err
		}
		x.R, err = e.expandExpr(x.R, depth)
	case *ast.AssignOpExpr:
		if x.L, err = e.expandExpr(x.L, depth); err != nil {
			return nil, err
		}
		x.R, err = e.expandExpr(x.R, depth)
	case *ast.CallExpr:
		if x.Fn, err = e.expandExpr(x.Fn, depth); err != nil {
			return nil, err
		}
		x.Args, err = e.expandExprList(x.Args, depth)
	case *ast.MethodCallExpr:
		if x.Receiver, err = e.expandExpr(x.Receiver, depth); err != nil {
			return nil, err
		}
		if err = e.expandGenericArgs(x.Generics, depth); err != nil {
			return nil, err
		}
		x.Args, err = e.expandExprList(x.Args, depth)
	case *ast.FieldExpr:
		x.X, err = e.expandExpr(x.X, depth)
	case *ast.IndexExpr:
		if x.X, err = e.expandExpr(x.X, depth); err != nil {
			return nil, err
		}
		x.Index, err = e.expandExpr(x.Index, depth)
	case *ast.RefExpr:
		x.X, err = e.expandExpr(x.X, depth)
	case *ast.AsmExpr:
		for _, op := range x.Operands {
			same := op.Out == op.In
			if op.In != nil {
				if op.In, err = e.expandExpr(op.In, depth); err != nil {
					return nil, err
				}
			}
			if same {
				op.Out = op.In
			} else if op.Out != nil {
				if op.Out, err = e.expandExpr(op.Out, depth); err != nil {
					return nil, err
				}
			}
		}
	case *ast.CastExpr:
		if x.X, err = e.expandExpr(x.X, depth); err != nil {
			return nil, err
		}
		x.Ty, err = e.expandType(x.Ty, depth)
	case *ast.BlockExpr:
		err = e.expandBlock(x.Block, depth)
	case *ast.IfExpr:
		if x.Cond, err = e.expandExpr(x.Cond, depth); err != nil {
			return nil, err
		}
		if err = e.expandBlock(x.Then, depth); err != nil {
			return nil, err
		}
		x.Else, err = e.expandExpr(x.Else, depth)
	case *ast.LetExpr:
		if x.Pat, err = e.expandPat(x.Pat, depth); err != nil {
			return nil, err
		}
		x.Expr, err = e.expandExpr(x.Expr, depth)
	case *ast.WhileExpr:
		if x.Cond, err = e.expandExpr(x.Cond, depth); err != nil {
			return nil, err
		}
		err = e.expandBlock(x.Body, depth)
	case *ast.LoopExpr:
		err = e.expandBlock(x.Body, depth)
	case *ast.ForExpr:
		if x.Pat, err = e.expandPat(x.Pat, depth); err != nil {
			return nil, err
		}
		if x.Iter, err = e.expandExpr(x.Iter, depth); err != nil {
			return nil, err
		}
		err = e.expandBlock(x.Body, depth)
	case *ast.MatchExpr:
		if x.Scrutinee, err = e.expandExpr(x.Scrutinee, depth); err != nil {
			return nil, err
		}
		arms := x.Arms[:0]
		for _, a := range x.Arms {
			keep, attrs, err := e.processAttrs(a.Attrs)
			if err != nil {
				return nil, err
			}
			if !keep {
				continue
			}
			a.Attrs = attrs
			if a.Pat, err = e.expandPat(a.Pat, depth); err != nil {
				return nil, err
			}
			if a.Guard, err = e.expandExpr(a.Guard, depth); err != nil {
				return nil, err
			}
			if a.Body, err = e.expandExpr(a.Body, depth); err != nil {
				return nil, err
			}
			arms = append(arms, a)
		}
		x.Arms = arms
	case *ast.ClosureExpr:
		for _, p := range x.Params {
			if p.Pat, err = e.expandPat(p.Pat, depth); err != nil {
				return nil, err
			}
			if p.Ty, err = e.expandType(p.Ty, depth); err != nil {
				return nil, err
			}
		}
		if x.Ret, err = e.expandType(x.Ret, depth); err != nil {
			return nil, err
		}
		x.Body, err = e.expandExpr(x.Body, depth)
	case *ast.BreakExpr:
		x.Value, err = e.expandExpr(x.Value, depth)
	case *ast.ReturnExpr:
		x.Value, err = e.expandExpr(x.Value, depth)
	case *ast.TupleExpr:
		x.Elems, err = e.expandExprList(x.Elems, depth)
	case *ast.ArrayExpr:
		x.Elems, err = e.expandExprList(x.Elems, depth)
	case *ast.RepeatExpr:
		if x.Elem, err = e.expandExpr(x.Elem, depth); err != nil {
			return nil, err
		}
		x.Count, err = e.expandExpr(x.Count, depth)
	case *ast.StructExpr:
		if err = e.expandQSelf(x.QSelf, depth); err != nil {
			return nil, err
		}
		if err = e.expandPath(x.Path, depth); err != nil {
			return nil, err
		}
		fields := x.Fields[:0]
		for _, f := range x.Fields {
			keep, attrs, err := e.processAttrs(f.Attrs)
			if err != nil {
				return nil, err
			}
			if !keep {
				continue
			}
			f.Attrs = attrs
			if f.Expr, err = e.expandExpr(f.Expr, depth); err != nil {
				return nil, err
			}
			fields = append(fields, f)
		}
		x.Fields = fields
		x.Base, err = e.expandExpr(x.Base, depth)
	case *ast.RangeExpr:
		if x.Start, err = e.expandExpr(x.Start, depth); err != nil {
			return nil, err
		}
		x.End, err = e.expandExpr(x.End, depth)
	case *ast.TryExpr:
		x.X, err = e.expandExpr(x.X, depth)
	case *ast.ParenExpr:
		x.X, err = e.expandExpr(x.X, depth)
	case *ast.AwaitExpr:
		x.X, err = e.expandExpr(x.X, depth)
	case *ast.FormatArgsExpr:
		x.Args, err = e.expandExprList(x.Args, depth)
	case *ast.MacCallExpr:
		if err := e.checkDepth(x.Mac, depth); err != nil {
			return nil, err
		}
		res, user, err := e.expandExprMac(x.Mac)
		if err != nil {
			return nil, err
		}
		var out ast.Expr
		f := func() error {
			out, err = e.expandExpr(res, depth+1)
			return err
		}
		if user {
			err = e.withSite(x.Mac.Span, f)
		} else {
			err = f()
		}
		if err != nil {
			return nil, err
		}
		if len(x.Attrs) > 0 {
			if b := exprBase(out); b != nil {
				b.Attrs = append(x.Attrs, b.Attrs...)
			}
		}
		return out, nil
	}
	if err != nil {
		return nil, err
	}
	return x, nil
}
