package typeck

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
)

// AdjustKind enumerates expression adjustments (implicit coercions).
type AdjustKind int

const (
	AdjDeref           AdjustKind = iota // builtin deref of &T / &mut T / Box<T>
	AdjDerefOverloaded                   // Deref::deref / DerefMut::deref_mut (Callee set); result is the place *callee(&x)
	AdjBorrow                            // &x / &mut x
	AdjUnsize                            // unsizing coercion to Target (refs, raw ptrs, Box)
	AdjNeverToAny                        // ! -> Target
	AdjReifyFnPtr                        // fn item -> fn pointer
	AdjClosureFnPtr                      // non-capturing closure -> fn pointer
	AdjMutToConst                        // &mut T -> &T, *mut T -> *const T
	AdjBorrowRaw                         // &mut T -> *mut T, &T -> *const T
)

// Adjustment is one step applied to an expression's value. Target is the
// resulting type.
type Adjustment struct {
	Kind   AdjustKind
	Mut    bool
	Target Ty
	Callee *Callee
}

// CalleeKind classifies resolved calls.
type CalleeKind int

const (
	CallFn      CalleeKind = iota // static call to Def with Args
	CallVirtual                   // trait-object method: Def = trait method, Args[0] = dyn type
	CallCtor                      // tuple struct / variant constructor: Def = struct or variant
	CallFnPtr                     // indirect call through a fn pointer value
	CallClosure                   // direct call of a closure value; Ty = closure type
	CallDynFn                     // call of a dyn Fn* trait object; Ty = dyn type
	CallBuiltin                   // builtin operation (e.g. Clone of a Copy type, Fn impls) — see Def/Args
)

// Callee describes the target of a call, method call, or overloaded operator.
type Callee struct {
	Kind CalleeKind
	Def  *Def
	Args []Ty
	Ty   Ty // for CallClosure / CallDynFn / CallFnPtr: the callee type
}

// ValueResKind classifies value paths.
type ValueResKind int

const (
	VLocal      ValueResKind = iota
	VFn                      // function item value (Def, Args) — includes ctor fns
	VConst                   // const item (Def, Args)
	VStatic                  // static item
	VUnitCtor                // unit struct / variant value (Def = struct/variant, Args = adt args)
	VConstParam              // const generic value (Value)
)

type ValueRes struct {
	Kind  ValueResKind
	Local *Local
	Def   *Def
	Args  []Ty
	Value uint64
}

// Local is a local variable (binding).
type Local struct {
	ID     int
	Name   string
	Ty     Ty
	Mut    bool
	ByRef  bool // `ref x` / default-binding-mode by reference
	RefMut bool
	Span   lexer.Span
	Node   ast.NodeID // binding pattern node (IdentPat) or SelfParam id
	// Closure: if this local belongs to a closure body (param or let), the
	// closure node; 0 for fn-level locals.
	Owner ast.NodeID
}

// FieldRes identifies a field access.
type FieldRes struct {
	Index int
	// Union is set for union field accesses.
	Union bool
}

// Capture describes a variable captured by a closure.
type Capture struct {
	Local *Local
	ByRef bool
	Mut   bool
	Ty    Ty // the captured local's type
}

// ClosureInfo describes a closure expression in a particular instance.
type ClosureInfo struct {
	Ty       Ty
	Expr     *ast.ClosureExpr
	Inputs   []Ty
	Output   Ty
	Kind     int // 0 Fn, 1 FnMut, 2 FnOnce
	Captures []*Capture
	Results  *Results // owning body's results
	Params   []*Local
	// Sealed: the owning body is fully checked (signature resolved, kind
	// and captures final), so trait answers about the closure can be
	// cached globally.
	Sealed bool
}

// ForInfo records how a for loop iterates.
type ForInfo struct {
	IntoIter *Callee // IntoIterator::into_iter
	IterTy   Ty      // type of the iterator
	Next     *Callee // Iterator::next
	ItemTy   Ty
}

// TryInfo records how `?` is desugared.
type TryInfo struct {
	IsOption bool
	// From converts the error (Result only), nil if the error types match.
	From     *Callee
	ErrTy    Ty // source error type
	OutErrTy Ty // target error type
	OkTy     Ty
	OutTy    Ty // the enclosing fn/closure return type
}

// Results holds the type-checking results of one body instance.
type Results struct {
	Inst      Instance
	NodeTypes map[ast.NodeID]Ty
	Adjust    map[ast.NodeID][]Adjustment
	// PatDerefs[pat] = number of implicit derefs (match ergonomics) applied
	// to the scrutinee before matching pat; the types peeled are recorded.
	PatDerefs map[ast.NodeID][]Ty
	Callees   map[ast.NodeID]*Callee
	Paths     map[ast.NodeID]*ValueRes
	Fields    map[ast.NodeID]FieldRes
	Locals    map[ast.NodeID]*Local // binding pattern node -> local
	Variants  map[ast.NodeID]*VariantDef
	Closures  map[ast.NodeID]*ClosureInfo
	Fors      map[ast.NodeID]*ForInfo
	Tries     map[ast.NodeID]*TryInfo
	// OpOverloaded marks binary/unary/compound-assign exprs implemented via
	// a trait call (Callee in Callees).
	// IndexBuiltin marks index exprs handled natively (arrays/slices).
	IndexBuiltin map[ast.NodeID]bool
	// MutPlace marks overloaded deref/index expressions that must use the
	// mutable variant (deref_mut / index_mut).
	MutPlace map[ast.NodeID]bool
	// Params: locals for fn params (by position), SelfLocal for `self`.
	Params    []*Local
	SelfLocal *Local
	RetTy     Ty
	ParamTys  []Ty
	Body      *ast.Block
	ConstExpr ast.Expr // for const/static bodies
	// Labels for loops/blocks.
	LocalCount int
}

func newResults(inst Instance) *Results {
	return &Results{
		Inst:         inst,
		NodeTypes:    map[ast.NodeID]Ty{},
		Adjust:       map[ast.NodeID][]Adjustment{},
		PatDerefs:    map[ast.NodeID][]Ty{},
		Callees:      map[ast.NodeID]*Callee{},
		Paths:        map[ast.NodeID]*ValueRes{},
		Fields:       map[ast.NodeID]FieldRes{},
		Locals:       map[ast.NodeID]*Local{},
		Variants:     map[ast.NodeID]*VariantDef{},
		Closures:     map[ast.NodeID]*ClosureInfo{},
		Fors:         map[ast.NodeID]*ForInfo{},
		Tries:        map[ast.NodeID]*TryInfo{},
		IndexBuiltin: map[ast.NodeID]bool{},
		MutPlace:     map[ast.NodeID]bool{},
	}
}

// TypeOf returns the (unadjusted) type of an expression or pattern.
func (r *Results) TypeOf(id ast.NodeID) Ty { return r.NodeTypes[id] }

// AdjustedType returns the type of an expression after adjustments.
func (r *Results) AdjustedType(e ast.Expr) Ty {
	if adj := r.Adjust[e.GetID()]; len(adj) > 0 {
		return adj[len(adj)-1].Target
	}
	return r.NodeTypes[e.GetID()]
}
