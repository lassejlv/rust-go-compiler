package middle

import (
	"strconv"
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
)

// DefKind classifies definitions.
type DefKind int

const (
	DefMod DefKind = iota
	DefStruct
	DefEnum
	DefUnion
	DefVariant
	DefTrait
	DefTyAlias
	DefFn
	DefConst
	DefStatic
	DefImpl
	DefAssocTy
	DefTyParam
	DefConstParam
	DefClosure
	DefCrate
)

var defKindNames = [...]string{"module", "struct", "enum", "union", "variant", "trait", "type alias", "function", "constant", "static", "impl", "associated type", "type parameter", "const parameter", "closure", "crate"}

func (k DefKind) String() string { return defKindNames[k] }

// Def is any named (or anonymous) definition.
type Def struct {
	ID     int
	Kind   DefKind
	Name   string
	Parent *Def // enclosing module, trait, impl, or enum
	Crate  *Crate
	Item   *ast.Item
	Span   lexer.Span
	Attrs  []*ast.Attr

	// Scope is the module in which names used by this definition's
	// signature/body are resolved (the containing module or block module).
	Scope *Module
	// Mod is the namespace this def introduces (modules, enums, traits).
	Mod *Module

	Adt     *AdtDef     // struct/enum/union
	Variant *VariantDef // for DefVariant (and struct defs point to their single variant via Adt)
	Trait   *TraitInfo
	Impl    *ImplInfo
	Fn      *FnInfo
	Const   *ConstInfo // const / static
	AssocTy *AssocTyInfo
	Alias   *AliasInfo

	Generics *Generics // filled by collect (nil until then)

	// State for lazy collection.
	Collecting bool
	Collected  bool

	LangItem  string // #[lang = "..."]
	Intrinsic string // name of compiler intrinsic, for extern "rust-intrinsic" fns
}

var defCounter int

// NewDef allocates a fresh definition.
func NewDef(kind DefKind, name string, parent *Def) *Def {
	defCounter++
	d := &Def{ID: defCounter, Kind: kind, Name: name, Parent: parent}
	if parent != nil {
		d.Crate = parent.Crate
	}
	return d
}

// Path returns a human-readable path for diagnostics.
func (d *Def) Path() string {
	var parts []string
	for x := d; x != nil; x = x.Parent {
		if x.Kind == DefCrate {
			if x.Name != "main" {
				parts = append(parts, x.Name)
			}
			break
		}
		if x.Kind == DefImpl {
			continue
		}
		if x.Name == "" {
			continue
		}
		parts = append(parts, x.Name)
	}
	for i, j := 0, len(parts)-1; i < j; i, j = i+1, j-1 {
		parts[i], parts[j] = parts[j], parts[i]
	}
	if len(parts) == 0 {
		return d.Name
	}
	// Shorten std paths for readability: std::option::Option -> Option.
	if len(parts) > 1 && parts[0] == "std" {
		return parts[len(parts)-1]
	}
	return strings.Join(parts, "::")
}

func (d *Def) String() string { return d.Path() }

// HasAttr reports whether the def has #[name].
func (d *Def) HasAttr(name string) bool {
	for _, a := range d.Attrs {
		if a.Path == name {
			return true
		}
	}
	return false
}

// IsAssoc reports whether d is an associated item of a trait or impl.
func (d *Def) IsAssoc() bool {
	return d.Parent != nil && (d.Parent.Kind == DefTrait || d.Parent.Kind == DefImpl)
}

// InTrait reports whether d is declared inside a trait.
func (d *Def) InTrait() bool { return d.Parent != nil && d.Parent.Kind == DefTrait }

// InImpl reports whether d is declared inside an impl.
func (d *Def) InImpl() bool { return d.Parent != nil && d.Parent.Kind == DefImpl }

// ---------------------------------------------------------------------------
// ADTs

type AdtKind int

const (
	AdtStruct AdtKind = iota
	AdtEnum
	AdtUnion
)

type AdtDef struct {
	Kind     AdtKind
	Variants []*VariantDef
	Repr     string // #[repr(...)] integer type for enums, if any
	// TailParam: for structs whose last field may be unsized, the index of
	// the generic param determining it; -2 if the tail is always unsized;
	// -1 if the struct is always sized. Valid when TailKnown.
	TailParam int
	TailKnown bool
}

func (a *AdtDef) IsEnum() bool { return a.Kind == AdtEnum }

type VariantDef struct {
	Def    *Def // the variant def (enums) or the struct def
	Name   string
	Index  int
	Shape  ast.VariantShape
	Fields []*FieldDef
	Disc   int64 // discriminant value
	Ast    *ast.Variant
}

// FieldIndex returns the index of a named field or -1.
func (v *VariantDef) FieldIndex(name string) int {
	for i, f := range v.Fields {
		if f.Name == name {
			return i
		}
	}
	return -1
}

type FieldDef struct {
	Name string // "0", "1", ... for tuple fields
	Ty   Ty     // in terms of the ADT's generic params
	Vis  ast.Visibility
	Ast  *ast.FieldDef
}

// ---------------------------------------------------------------------------
// Traits and impls

type TraitInfo struct {
	Items       map[string]*Def
	Order       []*Def
	Supertraits []*TraitRef // Self = Param(0)
	Impls       []*Def
	Auto        bool
	Marker      bool // no items
	// SuperPreds are where-clause predicates on Self (where Self: Trait).
}

type ImplInfo struct {
	TraitRef *TraitRef // nil for inherent impls (in terms of impl generics)
	SelfTy   Ty
	Items    map[string]*Def
	Order    []*Def
	Negative bool
	Ast      *ast.ImplItem
}

type FnInfo struct {
	Ast     *ast.FnItem
	Sig     *FnSig // lowered lazily
	HasSelf bool
	Extern  bool // extern "C" foreign fn
	// Opaque types (impl Trait in return position) declared by the signature:
	// OpaqueBounds[i] are the bounds of Opaque(def, args, i).
	OpaqueBounds [][]*Predicate
	// AstOpaques[i] is the ast node of the i-th impl Trait.
	AstOpaques []*ast.ImplTraitType
}

// FnSig is a lowered function signature (in terms of the fn's generics).
type FnSig struct {
	Inputs   []Ty
	Output   Ty
	Variadic bool
}

type ConstInfo struct {
	Ty     Ty // lowered lazily
	Expr   ast.Expr
	TyAst  ast.Type
	Mut    bool // static mut
	Static bool
}

type AssocTyInfo struct {
	Bounds  []*Predicate // bounds on the assoc type (Self::Name: Bound) — trait decls
	Default ast.Type
	Ty      Ty // for impl assoc types: the lowered type (in impl generics)
	TyAst   ast.Type
}

type AliasInfo struct {
	Ty    Ty
	TyAst ast.Type
}

// ---------------------------------------------------------------------------
// Generics

type GenericParamDef struct {
	Name      string
	Index     int
	Const     bool
	ConstTy   Ty
	Default   ast.Type
	Def       *Def // the TyParam/ConstParam def
	Owner     *Def
	Ast       *ast.GenericParam
	Synthetic bool // impl Trait in argument position
	// MaybeUnsized: the param has a `?Sized` bound (otherwise it is
	// implicitly Sized).
	MaybeUnsized bool
}

// Predicate is a bound `Trait.Args[0]: Trait<Args[1:]>` optionally with
// associated-type equality constraints (Iterator<Item = X>), or a pure
// projection equality <T as Tr>::Name == Ty.
type Predicate struct {
	Trait *TraitRef
	// Projections: assoc type bindings attached to this trait bound.
	Bindings []Binding
	Maybe    bool // ?Sized
}

func (p *Predicate) Subst(args []Ty) *Predicate {
	q := &Predicate{Trait: p.Trait.Subst(args), Maybe: p.Maybe}
	for _, b := range p.Bindings {
		nb := Binding{Name: b.Name, Ty: Subst(b.Ty, args)}
		for _, a := range b.Args {
			nb.Args = append(nb.Args, Subst(a, args))
		}
		q.Bindings = append(q.Bindings, nb)
	}
	return q
}

// Generics lists all generic params visible in an item: the parent's params
// first (impl/trait), then the item's own. For traits, param 0 is Self.
type Generics struct {
	Parent     *Generics
	Params     []*GenericParamDef // all params incl. parent's
	OwnStart   int                // index of the first own param
	HasSelf    bool
	Predicates []*Predicate // all predicates incl. parent's
}

// Count returns the total number of params.
func (g *Generics) Count() int {
	if g == nil {
		return 0
	}
	return len(g.Params)
}

// Own returns the item's own params.
func (g *Generics) Own() []*GenericParamDef {
	if g == nil {
		return nil
	}
	return g.Params[g.OwnStart:]
}

// Identity returns args mapping each param to itself.
func (g *Generics) Identity() []Ty {
	if g == nil {
		return nil
	}
	out := make([]Ty, len(g.Params))
	for i, p := range g.Params {
		if p.Const {
			out[i] = ConstParam(i, p.Name)
		} else {
			out[i] = Param(i, p.Name)
		}
	}
	return out
}

// ---------------------------------------------------------------------------
// Crates, modules and resolutions

type Crate struct {
	Name string
	Root *Module
	Def  *Def
	Ast  *ast.Crate
	// Edition is "2015", "2018", "2021" or "2024" ("" = 2021).
	Edition string
	// Externs is the extern prelude: dependency crates by name. nil means
	// "every loaded crate" (single-file compilation).
	Externs map[string]*Crate
}

// Is2015 reports whether the crate uses Rust 2015 path semantics.
func (c *Crate) Is2015() bool { return c != nil && c.Edition == "2015" }

type Namespace int

const (
	TypeNS Namespace = iota
	ValueNS
)

type ResKind int

const (
	ResErr      ResKind = iota
	ResDef              // Def
	ResPrim             // Prim (a primitive type name)
	ResSelfTy           // Def = impl or trait providing Self
	ResLocal            // Local (typeck)
	ResSelfCtor         // Def = impl (Self(..) / Self in value ns for tuple/unit structs)
	ResCtor             // Def = struct or variant used as constructor in value NS
)

type Res struct {
	Kind ResKind
	Def  *Def
	Prim string
}

func (r Res) IsErr() bool { return r.Kind == ResErr }

// NameBinding is an entry in a module namespace.
type NameBinding struct {
	Res    Res
	Import bool
	Span   lexer.Span
	// Vis is the visibility of the item or `use` declaration that made the
	// binding (relative to the module holding it).
	Vis ast.Visibility
}

// Module is a namespace: a `mod`, the crate root, an enum (for its
// variants), a trait (for its items), or an anonymous block scope.
type Module struct {
	Def    *Def
	Parent *Module // lexical parent (for block modules: enclosing module or block)
	Crate  *Crate
	Types  map[string]*NameBinding
	Values map[string]*NameBinding
	// Globs are glob imports (`use foo::*`) whose names are visible here.
	Globs []*Module
	// Normal is the nearest enclosing non-block module (self for normal mods).
	Normal *Module
	Block  bool
	Items  []*Def // defs declared directly here (incl impls)
	// Pending imports to resolve.
	Imports []*Import
	// AnonTraits are traits imported with `use Trait as _`.
	AnonTraits    []*Def
	traitsInScope map[*Def]bool
}

// TraitsInScope returns the traits usable for method calls from module m
// (items and imports of the enclosing block/module chain, globs, plus the
// given prelude).
func (m *Module) TraitsInScope(prelude *Module) map[*Def]bool {
	if m.traitsInScope != nil {
		return m.traitsInScope
	}
	set := map[*Def]bool{}
	var addMod func(x *Module, seen map[*Module]bool)
	addMod = func(x *Module, seen map[*Module]bool) {
		if x == nil || seen[x] {
			return
		}
		seen[x] = true
		for _, b := range x.Types {
			if b.Res.Def != nil && b.Res.Def.Kind == DefTrait {
				set[b.Res.Def] = true
			}
		}
		for _, t := range x.AnonTraits {
			set[t] = true
		}
		for _, g := range x.Globs {
			addMod(g, seen)
		}
	}
	for x := m; x != nil; x = x.Parent {
		addMod(x, map[*Module]bool{})
		if !x.Block {
			break
		}
	}
	addMod(prelude, map[*Module]bool{})
	m.traitsInScope = set
	return set
}

// Import is a single resolved-or-pending `use` binding.
type Import struct {
	Module *Module  // importing module
	Path   []string // full path segments (may start with "crate", "self", "super", "::")
	Global bool
	Name   string // binding name ("" for glob)
	Glob   bool
	Span   lexer.Span
	Done   bool
	Vis    ast.Visibility
}

func NewModule(def *Def, parent *Module, crate *Crate, block bool) *Module {
	m := &Module{Def: def, Parent: parent, Crate: crate, Types: map[string]*NameBinding{}, Values: map[string]*NameBinding{}, Block: block}
	if block && parent != nil {
		m.Normal = parent.Normal
	} else {
		m.Normal = m
	}
	return m
}

// Lookup returns the binding for name in ns, consulting glob imports.
func (m *Module) Lookup(name string, ns Namespace) *NameBinding {
	return m.lookup(name, ns, map[*Module]bool{})
}

func (m *Module) lookup(name string, ns Namespace, seen map[*Module]bool) *NameBinding {
	if seen[m] {
		return nil
	}
	seen[m] = true
	tbl := m.Types
	if ns == ValueNS {
		tbl = m.Values
	}
	if b, ok := tbl[name]; ok {
		return b
	}
	for _, g := range m.Globs {
		if b := g.lookup(name, ns, seen); b != nil {
			return b
		}
	}
	return nil
}

// Define adds a binding (later definitions do not override earlier ones
// unless the earlier one is an import).
func (m *Module) Define(name string, ns Namespace, b *NameBinding) {
	tbl := m.Types
	if ns == ValueNS {
		tbl = m.Values
	}
	if old, ok := tbl[name]; ok && !old.Import {
		return
	}
	tbl[name] = b
}

// ParentModule returns the module that contains this one (for `super`).
func (m *Module) ParentModule() *Module {
	n := m.Normal
	if n.Parent == nil {
		return n
	}
	return n.Parent.Normal
}

// ReprAlign returns N for #[repr(align(N))] (0 if absent).
func (d *Def) ReprAlign() uint64 {
	for _, a := range d.Attrs {
		if a.Path != "repr" || len(a.Tokens) == 0 || a.Tokens[0].Delim != '(' {
			continue
		}
		in := a.Tokens[0].Inner
		for i := 0; i+1 < len(in); i++ {
			if in[i].Delim == 0 && in[i].Tok.Text == "align" && in[i+1].Delim == '(' && len(in[i+1].Inner) == 1 {
				if n, err := strconv.ParseUint(strings.ReplaceAll(in[i+1].Inner[0].Tok.Text, "_", ""), 0, 64); err == nil {
					return n
				}
			}
		}
	}
	return 0
}
