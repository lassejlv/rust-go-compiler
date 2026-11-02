// Package ast defines the Rust abstract syntax tree (mirrors rustc_ast).
//
// Every Expr, Pat, Type, Item and binding carries a unique NodeID so that later
// passes (resolve, typeck) can attach information in side tables.
package ast

import (
	"rustgo/compiler/lexer"
)

type Span = lexer.Span
type Token = lexer.Token

// NodeID uniquely identifies an AST node.
type NodeID int

var nextID NodeID = 1

// NewID allocates a fresh NodeID.
func NewID() NodeID { nextID++; return nextID }

// Ident is a name with a span.
type Ident struct {
	Name string
	Span Span
}

// ---------------------------------------------------------------------------
// Token trees (macro input)

// TokenTree is either a single token (Delim == 0) or a delimited group.
type TokenTree struct {
	Tok    Token       // valid when Delim == 0
	Delim  byte        // 0, '(', '[', '{'
	Open   Span        // span of the opening delimiter (groups only)
	Close  Span        // span of the closing delimiter (groups only)
	Inner  []TokenTree // group contents
}

// ---------------------------------------------------------------------------
// Attributes

// Attr is an attribute: #[path(args)] or #![...] (Inner).
// Meta holds the parsed meta structure when it is in the usual form.
type Attr struct {
	Inner  bool
	Path   string      // e.g. "derive", "allow", "test", "doc"
	Tokens []TokenTree // raw tokens after the path (e.g. the (...) group or = value)
	Span   Span
}

// ---------------------------------------------------------------------------
// Paths

// Path is a (possibly global) path like ::std::vec::Vec<T>.
type Path struct {
	Global   bool // leading ::
	Segments []*PathSegment
	Span     Span
	ID       NodeID // resolution side-table key
}

// PathSegment is one segment of a path. Name may be "self", "super",
// "crate", "Self".
type PathSegment struct {
	Ident Ident
	Args  *GenericArgs // nil if none
}

// GenericArgs is <A, B, Item = C> or the parenthesized form (A, B) -> C used
// with Fn traits.
type GenericArgs struct {
	Args          []GenericArg
	Bindings      []*AssocBinding
	Parenthesized bool
	Inputs        []Type // parenthesized form
	Output        Type   // parenthesized form; nil means ()
	Span          Span
}

// GenericArg is a type, a lifetime, or a const expression.
type GenericArg struct {
	Type     Type   // non-nil for type args
	Lifetime string // non-empty for lifetime args
	Const    Expr   // non-nil for const args ({N} or literal)
}

// AssocBinding is Item = Type (or Item: Bound) inside generic args.
type AssocBinding struct {
	Name   Ident
	Ty     Type
	Bounds []*Bound
}

// QSelf is the <T as Trait> part of a qualified path. Position is the number
// of segments of the following Path that belong to the trait.
// For <T>::foo, Position is 0 and the Path has only "foo".
// For <T as a::Trait>::foo, Path = a::Trait::foo, Position = 2.
type QSelf struct {
	Ty       Type
	Position int
}

// ---------------------------------------------------------------------------
// Types

type Type interface {
	typeNode()
	GetSpan() Span
	GetID() NodeID
}

type TypeBase struct {
	Span Span
	ID   NodeID
}

func (t *TypeBase) typeNode()       {}
func (t *TypeBase) GetSpan() Span   { return t.Span }
func (t *TypeBase) GetID() NodeID   { return t.ID }

// PathType: a named type, possibly qualified (<T as Trait>::Assoc). "Self" is
// a PathType with a single segment "Self".
type PathType struct {
	TypeBase
	QSelf *QSelf
	Path  *Path
}

type RefType struct {
	TypeBase
	Lifetime string
	Mut      bool
	Elem     Type
}

type PtrType struct {
	TypeBase
	Mut  bool
	Elem Type
}

type SliceType struct {
	TypeBase
	Elem Type
}

type ArrayType struct {
	TypeBase
	Elem Type
	Len  Expr
}

// TupleType; zero elements is the unit type ().
type TupleType struct {
	TypeBase
	Elems []Type
}

type FnPtrType struct {
	TypeBase
	Unsafe   bool
	Abi      string
	Inputs   []Type
	Output   Type // nil = ()
	Variadic bool
}

type NeverType struct{ TypeBase }

// InferType is `_`.
type InferType struct{ TypeBase }

type ImplTraitType struct {
	TypeBase
	Bounds []*Bound
}

type DynTraitType struct {
	TypeBase
	Bounds []*Bound
}

// ParenType is (T) — kept so the printer can round-trip; semantically T.
type ParenType struct {
	TypeBase
	Elem Type
}

// MacroType is a macro invocation in type position.
type MacroType struct {
	TypeBase
	Mac *MacCall
}

// ---------------------------------------------------------------------------
// Generics

type GenericParamKind int

const (
	GPLifetime GenericParamKind = iota
	GPType
	GPConst
)

type GenericParam struct {
	Kind    GenericParamKind
	Name    Ident
	Bounds  []*Bound // for type params; lifetime bounds as Bound{Lifetime:..}
	Default Type     // type default (or nil)
	ConstTy Type     // for const params
	ConstDefault Expr
	Attrs   []*Attr
	ID      NodeID
}

type Generics struct {
	Params []*GenericParam
	Where  []*WherePredicate
	Span   Span
}

// WherePredicate is `Ty: Bounds` (lifetime predicates are dropped or kept with
// Lifetime set).
type WherePredicate struct {
	ForLifetimes []string
	Ty           Type
	Lifetime     string // 'a: 'b predicates
	Bounds       []*Bound
	Span         Span
}

// Bound is a trait bound (`?Sized`, `for<'a> Fn(&'a T)`, `Trait<X>`) or a
// lifetime bound ('a).
type Bound struct {
	Lifetime     string // non-empty for a lifetime bound
	Path         *Path  // trait path for trait bounds
	Maybe        bool   // ?Trait
	MaybeConst   bool   // ~const
	ForLifetimes []string
	Span         Span
}

// ---------------------------------------------------------------------------
// Items

type Visibility int

const (
	VisPrivate Visibility = iota
	VisPub
	VisCrate      // pub(crate)
	VisRestricted // pub(super), pub(in path)
)

type Item struct {
	Attrs []*Attr
	Vis   Visibility
	Name  Ident // empty for impls, uses, anonymous consts (_), macro calls
	Kind  ItemKind
	Span  Span
	ID    NodeID
	// Tokens are the item's source tokens after its outer attributes
	// (visibility included); used to feed procedural macros.
	Tokens []Token
}

type ItemKind interface{ itemKind() }

type FnDecl struct {
	SelfParam *SelfParam // nil if no self param
	Params    []*Param
	Output    Type // nil = ()
	Variadic  bool
}

type SelfParamKind int

const (
	SelfValue  SelfParamKind = iota // self / mut self
	SelfRef                         // &self / &'a self
	SelfRefMut                      // &mut self
	SelfTyped                       // self: Type
)

type SelfParam struct {
	Kind     SelfParamKind
	Mut      bool // `mut self`
	Lifetime string
	Ty       Type // for SelfTyped
	Span     Span
	ID       NodeID // binding id of `self`
}

type Param struct {
	Attrs []*Attr
	Pat   Pat
	Ty    Type
	Span  Span
}

type FnItem struct {
	Generics *Generics
	Decl     *FnDecl
	Body     *Block // nil for required trait methods / extern fns
	Const    bool
	Unsafe   bool
	Async    bool
	Abi      string // extern "C" fn
}

type FieldDef struct {
	Attrs []*Attr
	Vis   Visibility
	Name  Ident // empty for tuple fields
	Ty    Type
	Span  Span
	ID    NodeID
}

type VariantShape int

const (
	ShapeNamed VariantShape = iota // { a: T }
	ShapeTuple                     // (T, U)
	ShapeUnit                      // nothing
)

type StructItem struct {
	Generics *Generics
	Shape    VariantShape
	Fields   []*FieldDef
}

type UnionItem struct {
	Generics *Generics
	Fields   []*FieldDef
}

type Variant struct {
	Attrs  []*Attr
	Name   Ident
	Shape  VariantShape
	Fields []*FieldDef
	Disc   Expr // explicit discriminant = expr (or nil)
	Span   Span
	ID     NodeID
}

type EnumItem struct {
	Generics *Generics
	Variants []*Variant
}

type TraitItem struct {
	Generics   *Generics
	Supertraits []*Bound
	Items      []*Item // FnItem, TypeAliasItem (assoc types), ConstItem
	Unsafe     bool
	Auto       bool
}

// ImplItem is `impl<G> Trait for Ty { ... }` or `impl<G> Ty { ... }`.
type ImplItem struct {
	Generics *Generics
	Trait    *Path // nil for inherent impls
	Negative bool  // impl !Trait for T
	SelfTy   Type
	Items    []*Item
	Unsafe   bool
}

// TypeAliasItem: `type X<G> = T;` or in traits `type Item: Bounds;`.
type TypeAliasItem struct {
	Generics *Generics
	Bounds   []*Bound
	Ty       Type // nil for assoc type declarations without default
}

type ConstItem struct {
	Ty   Type
	Expr Expr // nil for trait assoc consts without default
}

type StaticItem struct {
	Ty   Type
	Mut  bool
	Expr Expr
}

type ModItem struct {
	Items  []*Item
	Inline bool // mod foo { ... } vs mod foo; (the driver loads the file and fills Items)
	Loaded bool
}

// UseTree: `a::b::{c, d as e, f::*}`.
type UseTree struct {
	Prefix *Path     // may have zero segments for `use {a, b}` / `use ::*`
	Kind   UseKind
	Rename *Ident    // for UseSimple: `as name` (name "_" allowed)
	Nested []*UseTree
	Span   Span
	ID     NodeID
}

type UseKind int

const (
	UseSimple UseKind = iota // use a::b (as c)
	UseGlob                  // use a::b::*
	UseNested                // use a::b::{...}
)

type UseItem struct {
	Tree *UseTree
}

type ExternCrateItem struct {
	Rename *Ident
}

type ExternBlockItem struct {
	Abi   string
	Items []*Item // FnItem with Body == nil, StaticItem
}

// MacroRulesItem: macro_rules! name { (matcher) => { transcriber }; ... }
type MacroRulesItem struct {
	Body []TokenTree // contents of the outer braces
}

// MacCallItem: a macro invoked in item position (e.g. `thread_local!{...}` or a
// user macro producing items).
type MacCallItem struct {
	Mac *MacCall
}

func (*FnItem) itemKind()          {}
func (*StructItem) itemKind()      {}
func (*UnionItem) itemKind()       {}
func (*EnumItem) itemKind()        {}
func (*TraitItem) itemKind()       {}
func (*ImplItem) itemKind()        {}
func (*TypeAliasItem) itemKind()   {}
func (*ConstItem) itemKind()       {}
func (*StaticItem) itemKind()      {}
func (*ModItem) itemKind()         {}
func (*UseItem) itemKind()         {}
func (*ExternCrateItem) itemKind() {}
func (*ExternBlockItem) itemKind() {}
func (*MacroRulesItem) itemKind()  {}
func (*MacCallItem) itemKind()     {}

// Crate is a parsed crate root.
type Crate struct {
	Attrs []*Attr
	Items []*Item
	Span  Span
}

// ---------------------------------------------------------------------------
// Macro invocations

// MacCall is `path!(tokens)` / `path![tokens]` / `path!{tokens}`.
type MacCall struct {
	Path  *Path
	Delim byte // '(', '[', '{'
	Args  []TokenTree
	Span  Span
}

// ---------------------------------------------------------------------------
// Statements and blocks

type Block struct {
	Stmts  []Stmt
	Unsafe bool
	Span   Span
	ID     NodeID
}

type Stmt interface{ stmtNode() }

// LetStmt: let pat: ty = init else { ... };
type LetStmt struct {
	Attrs []*Attr
	Pat   Pat
	Ty    Type   // may be nil
	Init  Expr   // may be nil
	Else  *Block // let-else, may be nil
	Span  Span
	ID    NodeID
}

// ExprStmt: an expression statement. Semi is false for the trailing
// expression of a block and for block-like expressions (if/match/loop/...)
// used as statements without a semicolon. The block's value is the final
// ExprStmt when Semi == false.
type ExprStmt struct {
	Expr Expr
	Semi bool
}

type ItemStmt struct {
	Item *Item
}

// MacStmt: a macro call in statement position (`println!(...);`). Semi
// records the trailing semicolon. Style is the delimiter.
type MacStmt struct {
	Attrs []*Attr
	Mac   *MacCall
	Semi  bool
}

type EmptyStmt struct{}

func (*LetStmt) stmtNode()   {}
func (*ExprStmt) stmtNode()  {}
func (*ItemStmt) stmtNode()  {}
func (*MacStmt) stmtNode()   {}
func (*EmptyStmt) stmtNode() {}

// ---------------------------------------------------------------------------
// Expressions

type Expr interface {
	exprNode()
	GetSpan() Span
	GetID() NodeID
}

type ExprBase struct {
	Span  Span
	ID    NodeID
	Attrs []*Attr
}

func (e *ExprBase) exprNode()      {}
func (e *ExprBase) GetSpan() Span  { return e.Span }
func (e *ExprBase) GetID() NodeID  { return e.ID }

type LitKind int

const (
	LitInt LitKind = iota
	LitFloat
	LitStr
	LitByteStr
	LitCStr
	LitChar
	LitByte
	LitBool
)

// LitExpr. Value holds: for Int the digits text with base prefix stripped and
// underscores removed is NOT done — use IntVal; for Float the cleaned text;
// for Str/ByteStr the decoded contents; for Char the decoded char as a string;
// for Bool "true"/"false".
type LitExpr struct {
	ExprBase
	Kind   LitKind
	Value  string
	IntVal uint64 // decoded integer value (Int, Char codepoint, Byte)
	Big    string // for integers that overflow uint64 (u128 literals): decimal text
	Suffix string
}

type PathExpr struct {
	ExprBase
	QSelf *QSelf
	Path  *Path
}

type UnOp int

const (
	UnNeg UnOp = iota
	UnNot
	UnDeref
)

type UnaryExpr struct {
	ExprBase
	Op UnOp
	X  Expr
}

type BinOp int

const (
	BinAdd BinOp = iota
	BinSub
	BinMul
	BinDiv
	BinRem
	BinAnd // &&
	BinOr  // ||
	BinBitXor
	BinBitAnd
	BinBitOr
	BinShl
	BinShr
	BinEq
	BinLt
	BinLe
	BinNe
	BinGe
	BinGt
)

var BinOpStr = [...]string{"+", "-", "*", "/", "%", "&&", "||", "^", "&", "|", "<<", ">>", "==", "<", "<=", "!=", ">=", ">"}

func (b BinOp) String() string { return BinOpStr[b] }

type BinaryExpr struct {
	ExprBase
	Op   BinOp
	L, R Expr
}

type AssignExpr struct {
	ExprBase
	L, R Expr
}

// AssignOpExpr: L op= R (Op is the arithmetic/bit op, e.g. BinAdd for +=).
type AssignOpExpr struct {
	ExprBase
	Op   BinOp
	L, R Expr
}

type CallExpr struct {
	ExprBase
	Fn   Expr
	Args []Expr
}

type MethodCallExpr struct {
	ExprBase
	Receiver Expr
	Method   Ident
	Generics *GenericArgs // turbofish ::<...>, may be nil
	Args     []Expr
}

// FieldExpr: X.Name where Name may be a tuple index ("0").
type FieldExpr struct {
	ExprBase
	X    Expr
	Name Ident
}

type IndexExpr struct {
	ExprBase
	X, Index Expr
}

// RefExpr: &X, &mut X, &raw const X, &raw mut X.
type RefExpr struct {
	ExprBase
	Mut bool
	Raw bool
	X   Expr
}

type CastExpr struct {
	ExprBase
	X  Expr
	Ty Type
}

type BlockExpr struct {
	ExprBase
	Block *Block
	Label string // 'label: { ... }
	Const bool   // const { }
	Async bool   // async { } / async move { }
	Move  bool   // async move { }
}

// IfExpr: Cond may contain LetExpr (if let / let chains joined by &&).
// Else is nil, a *BlockExpr, or another *IfExpr.
type IfExpr struct {
	ExprBase
	Cond Expr
	Then *Block
	Else Expr
}

// LetExpr: `let Pat = Expr` (only valid in if/while conditions, possibly
// combined with &&).
type LetExpr struct {
	ExprBase
	Pat  Pat
	Expr Expr
}

type WhileExpr struct {
	ExprBase
	Cond  Expr
	Body  *Block
	Label string
}

type LoopExpr struct {
	ExprBase
	Body  *Block
	Label string
}

type ForExpr struct {
	ExprBase
	Pat   Pat
	Iter  Expr
	Body  *Block
	Label string
}

type Arm struct {
	Attrs []*Attr
	Pat   Pat
	Guard Expr // may be nil
	Body  Expr
	Span  Span
	ID    NodeID
}

type MatchExpr struct {
	ExprBase
	Scrutinee Expr
	Arms      []*Arm
}

type ClosureParam struct {
	Pat Pat
	Ty  Type // may be nil
}

type ClosureExpr struct {
	ExprBase
	Move   bool
	Async  bool
	Params []*ClosureParam
	Ret    Type // may be nil
	Body   Expr
	// MoveParamsOf marks the future-producing closure of a non-move async
	// closure: it moves only that closure's parameters and captures other
	// variables like a non-move closure.
	MoveParamsOf *ClosureExpr
}

type BreakExpr struct {
	ExprBase
	Label string
	Value Expr
}

type ContinueExpr struct {
	ExprBase
	Label string
}

type ReturnExpr struct {
	ExprBase
	Value Expr
}

type TupleExpr struct {
	ExprBase
	Elems []Expr
}

type ArrayExpr struct {
	ExprBase
	Elems []Expr
}

// RepeatExpr: [Elem; Count]
type RepeatExpr struct {
	ExprBase
	Elem  Expr
	Count Expr
}

type FieldInit struct {
	Name      Ident // may be a tuple index like "0"
	Expr      Expr
	Shorthand bool
	Span      Span
}

// StructExpr: Path { a: x, b, ..base }
type StructExpr struct {
	ExprBase
	QSelf  *QSelf
	Path   *Path
	Fields []*FieldInit
	Base   Expr // ..base, may be nil
	Rest   bool // `..` with no base (struct patterns in destructuring assignment)
}

// RangeExpr: Start..End, Start..=End, either may be nil.
type RangeExpr struct {
	ExprBase
	Start, End Expr
	Inclusive  bool
}

// TryExpr: X?
type TryExpr struct {
	ExprBase
	X Expr
}

type ParenExpr struct {
	ExprBase
	X Expr
}

type MacCallExpr struct {
	ExprBase
	Mac *MacCall
}

// AwaitExpr: X.await (parsed, unsupported later).
type AwaitExpr struct {
	ExprBase
	X Expr
}

// UnderscoreExpr: `_` in destructuring assignment.
type UnderscoreExpr struct{ ExprBase }

// ---------------------------------------------------------------------------
// Expressions synthesized by macro expansion (never produced by the parser).

// FormatSpec mirrors a parsed `{arg:spec}` placeholder.
type FormatSpec struct {
	ArgIndex  int    // index into FormatArgsExpr.Args
	Trait     string // "Display", "Debug", "LowerHex", "UpperHex", "Octal", "Binary", "LowerExp", "UpperExp", "Pointer"
	Fill      rune
	Align     byte // 0, '<', '>', '^'
	Sign      byte // 0, '+', '-'
	Alternate bool // #
	ZeroPad   bool // 0
	Width     int  // -1 if none
	WidthArg  int  // index of arg supplying width (-1 if none)
	Prec      int  // -1 if none
	PrecArg   int  // index of arg supplying precision (-1 if none)
}

// FormatArgsExpr is the expansion of format_args!. Pieces has len(Specs)+1
// literal string fragments interleaved with the specs.
type FormatArgsExpr struct {
	ExprBase
	Pieces []string
	Specs  []*FormatSpec
	Args   []Expr
}

func (*FormatArgsExpr) isSynth() {}

// ---------------------------------------------------------------------------
// Patterns

type Pat interface {
	patNode()
	GetSpan() Span
	GetID() NodeID
}

type PatBase struct {
	Span Span
	ID   NodeID
}

func (p *PatBase) patNode()      {}
func (p *PatBase) GetSpan() Span { return p.Span }
func (p *PatBase) GetID() NodeID { return p.ID }

type WildPat struct{ PatBase }

// IdentPat: [ref] [mut] name [@ sub]. Whether `name` is a binding or refers
// to a unit struct/variant/const is decided by name resolution.
type IdentPat struct {
	PatBase
	ByRef bool
	Mut   bool
	Name  Ident
	Sub   Pat
}

// LitPat: a literal (possibly negated numeric literal), or a const path.
type LitPat struct {
	PatBase
	Expr Expr
}

// RangePat: lo..=hi, lo..hi, lo.., ..=hi. Lo/Hi are LitExpr, negated
// LitExpr, or PathExpr.
type RangePat struct {
	PatBase
	Lo, Hi    Expr
	Inclusive bool
}

type TuplePat struct {
	PatBase
	Elems []Pat // may include *RestPat
}

type TupleStructPat struct {
	PatBase
	QSelf *QSelf
	Path  *Path
	Elems []Pat
}

type FieldPat struct {
	Name      Ident
	Pat       Pat
	Shorthand bool
	Span      Span
}

type StructPat struct {
	PatBase
	QSelf  *QSelf
	Path   *Path
	Fields []*FieldPat
	Rest   bool
}

// PathPat: a path pattern with more than one segment or qualified
// (Foo::Bar, <T>::X). Single identifiers are IdentPat.
type PathPat struct {
	PatBase
	QSelf *QSelf
	Path  *Path
}

type RefPat struct {
	PatBase
	Mut bool
	Pat Pat
}

type BoxPat struct {
	PatBase
	Pat Pat
}

type SlicePat struct {
	PatBase
	Elems []Pat
}

type OrPat struct {
	PatBase
	Alts []Pat
}

// RestPat is `..` inside tuple/slice patterns (or `name @ ..` in slices,
// represented as IdentPat with Sub = RestPat).
type RestPat struct{ PatBase }

type ParenPat struct {
	PatBase
	Pat Pat
}

type MacPat struct {
	PatBase
	Mac *MacCall
}
