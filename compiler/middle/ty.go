// Package middle holds the compiler's central data structures: types,
// definitions, generics and modules (mirrors rustc_middle).
package middle

import (
	"fmt"
	"strconv"
	"strings"
	"sync"
)

// TyKind enumerates type constructors.
type TyKind uint8

const (
	TBool TyKind = iota
	TChar
	TInt   // Sub = IntTy
	TFloat // Sub = 32 or 64
	TStr
	TNever
	TTuple      // Elems
	TArray      // Elem, Len (a TConst / const TParam / TInfer)
	TSlice      // Elem
	TRef        // Mut, Elem
	TPtr        // Mut, Elem
	TAdt        // Def, Args
	TFnDef      // Def, Args (a zero-sized function item type)
	TFnPtr      // Elems = inputs, Ret
	TClosure    // Def (closure def), Args (enclosing instance args), Index = unique closure id
	TDyn        // Def = principal trait, Args = trait args (without Self), Bindings
	TParam      // Index, Name; Sub = 1 for const params
	TInfer      // Index = var id; Sub: 0 general, 1 integer, 2 float
	TProjection // Def = trait, Args = trait args incl. Self at [0], Name = assoc item
	TOpaque     // Def = fn def, Args = fn args, Index = impl-trait index
	TConst      // Val (const generic value)
	TError
)

// IntTy enumerates integer types.
type IntTy uint8

const (
	I8 IntTy = iota
	I16
	I32
	I64
	I128
	Isize
	U8
	U16
	U32
	U64
	U128
	Usize
)

var intNames = [...]string{"i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize"}

func (i IntTy) String() string { return intNames[i] }
func (i IntTy) Signed() bool   { return i <= Isize }
func (i IntTy) Bits() int {
	switch i {
	case I8, U8:
		return 8
	case I16, U16:
		return 16
	case I32, U32:
		return 32
	case I64, U64, Isize, Usize:
		return 64
	}
	return 128
}

// IntTyByName maps "i32" etc. to IntTy.
func IntTyByName(s string) (IntTy, bool) {
	for i, n := range intNames {
		if n == s {
			return IntTy(i), true
		}
	}
	return 0, false
}

// Binding is an associated-type binding on a dyn type: dyn Iterator<Item = T>.
type Binding struct {
	Name string
	Ty   Ty
}

// Ty is an interned type: pointer equality is structural equality.
type Ty = *TyS

type TyS struct {
	Kind     TyKind
	Sub      uint8
	Mut      bool
	Elem     Ty
	Len      Ty
	Elems    []Ty
	Ret      Ty
	Def      *Def
	Args     []Ty
	Index    int
	Name     string
	Val      uint64
	Bindings []Binding
	key      string
	flags    uint8
}

const (
	flagHasInfer uint8 = 1 << iota
	flagHasParam
	flagHasProj
	flagHasOpaque
	flagHasError
)

var (
	internMu sync.Mutex
	interned = map[string]Ty{}
)

func intern(t *TyS) Ty {
	k := t.computeKey()
	internMu.Lock()
	defer internMu.Unlock()
	if e, ok := interned[k]; ok {
		return e
	}
	t.key = k
	t.flags = t.computeFlags()
	interned[k] = t
	return t
}

func (t *TyS) computeFlags() uint8 {
	var f uint8
	switch t.Kind {
	case TInfer:
		f |= flagHasInfer
	case TParam:
		f |= flagHasParam
	case TProjection:
		f |= flagHasProj
	case TOpaque:
		f |= flagHasOpaque
	case TError:
		f |= flagHasError
	}
	add := func(x Ty) {
		if x != nil {
			f |= x.flags
		}
	}
	add(t.Elem)
	add(t.Len)
	add(t.Ret)
	for _, e := range t.Elems {
		add(e)
	}
	for _, e := range t.Args {
		add(e)
	}
	for _, b := range t.Bindings {
		add(b.Ty)
	}
	return f
}

// Key returns the canonical interning key (useful for maps / mangling).
func (t *TyS) Key() string { return t.key }

func (t *TyS) HasInfer() bool   { return t.flags&flagHasInfer != 0 }
func (t *TyS) HasParam() bool   { return t.flags&flagHasParam != 0 }
func (t *TyS) HasProj() bool    { return t.flags&flagHasProj != 0 }
func (t *TyS) HasOpaque() bool  { return t.flags&flagHasOpaque != 0 }
func (t *TyS) HasError() bool   { return t.flags&flagHasError != 0 }
func (t *TyS) NeedsSubst() bool { return t.flags&(flagHasParam) != 0 }

func (t *TyS) computeKey() string {
	var b strings.Builder
	t.writeKey(&b)
	return b.String()
}

func writeKeyList(b *strings.Builder, ts []Ty) {
	for i, x := range ts {
		if i > 0 {
			b.WriteByte(',')
		}
		b.WriteString(x.key)
	}
}

func (t *TyS) writeKey(b *strings.Builder) {
	switch t.Kind {
	case TBool:
		b.WriteString("bool")
	case TChar:
		b.WriteString("char")
	case TInt:
		b.WriteString(IntTy(t.Sub).String())
	case TFloat:
		if t.Sub == 32 {
			b.WriteString("f32")
		} else {
			b.WriteString("f64")
		}
	case TStr:
		b.WriteString("str")
	case TNever:
		b.WriteString("!")
	case TTuple:
		b.WriteByte('(')
		writeKeyList(b, t.Elems)
		b.WriteByte(')')
	case TArray:
		b.WriteByte('[')
		b.WriteString(t.Elem.key)
		b.WriteByte(';')
		b.WriteString(t.Len.key)
		b.WriteByte(']')
	case TSlice:
		b.WriteString("[")
		b.WriteString(t.Elem.key)
		b.WriteString("]")
	case TRef:
		if t.Mut {
			b.WriteString("&mut ")
		} else {
			b.WriteString("&")
		}
		b.WriteString(t.Elem.key)
	case TPtr:
		if t.Mut {
			b.WriteString("*mut ")
		} else {
			b.WriteString("*const ")
		}
		b.WriteString(t.Elem.key)
	case TAdt, TFnDef, TOpaque:
		switch t.Kind {
		case TFnDef:
			b.WriteString("fn#")
		case TOpaque:
			b.WriteString("opaque#")
			b.WriteString(strconv.Itoa(t.Index))
			b.WriteByte('#')
		}
		b.WriteString(strconv.Itoa(t.Def.ID))
		b.WriteByte('<')
		writeKeyList(b, t.Args)
		b.WriteByte('>')
	case TFnPtr:
		b.WriteString("fn(")
		writeKeyList(b, t.Elems)
		if t.Sub == 1 {
			b.WriteString(",...")
		}
		b.WriteString(")->")
		b.WriteString(t.Ret.key)
	case TClosure:
		b.WriteString("closure#")
		b.WriteString(strconv.Itoa(t.Index))
		b.WriteByte('<')
		writeKeyList(b, t.Args)
		b.WriteByte('>')
	case TDyn:
		b.WriteString("dyn ")
		if t.Def != nil {
			b.WriteString(strconv.Itoa(t.Def.ID))
		}
		b.WriteByte('<')
		writeKeyList(b, t.Args)
		for _, bd := range t.Bindings {
			b.WriteByte(';')
			b.WriteString(bd.Name)
			b.WriteByte('=')
			b.WriteString(bd.Ty.key)
		}
		b.WriteByte('>')
		if t.Sub != 0 {
			b.WriteString("+auto")
			b.WriteString(strconv.Itoa(int(t.Sub)))
		}
	case TParam:
		b.WriteString("P")
		b.WriteString(strconv.Itoa(t.Index))
		b.WriteByte(':')
		b.WriteString(t.Name)
		if t.Sub == 1 {
			b.WriteString(":const")
		}
	case TInfer:
		b.WriteString("?")
		b.WriteString(strconv.Itoa(t.Index))
		if t.Sub == 1 {
			b.WriteString("i")
		} else if t.Sub == 2 {
			b.WriteString("f")
		}
	case TProjection:
		b.WriteString("<")
		b.WriteString(strconv.Itoa(t.Def.ID))
		b.WriteByte('<')
		writeKeyList(b, t.Args)
		b.WriteString(">>::")
		b.WriteString(t.Name)
	case TConst:
		b.WriteString("#")
		b.WriteString(strconv.FormatUint(t.Val, 10))
	case TError:
		b.WriteString("{error}")
	}
}

// ---------------------------------------------------------------------------
// Constructors

var (
	TyBool  = intern(&TyS{Kind: TBool})
	TyChar  = intern(&TyS{Kind: TChar})
	TyStr   = intern(&TyS{Kind: TStr})
	TyNever = intern(&TyS{Kind: TNever})
	TyUnit  = intern(&TyS{Kind: TTuple})
	TyErr   = intern(&TyS{Kind: TError})
	TyF32   = intern(&TyS{Kind: TFloat, Sub: 32})
	TyF64   = intern(&TyS{Kind: TFloat, Sub: 64})
	intTys  = makeIntTys()
)

func makeIntTys() [12]Ty {
	var out [12]Ty
	for i := range out {
		out[i] = intern(&TyS{Kind: TInt, Sub: uint8(i)})
	}
	return out
}

func IntT(i IntTy) Ty { return intTys[i] }

var (
	TyI32   = IntT(I32)
	TyI64   = IntT(I64)
	TyU8    = IntT(U8)
	TyU32   = IntT(U32)
	TyU64   = IntT(U64)
	TyUsize = IntT(Usize)
	TyIsize = IntT(Isize)
)

func Tuple(elems []Ty) Ty {
	if len(elems) == 0 {
		return TyUnit
	}
	return intern(&TyS{Kind: TTuple, Elems: elems})
}
func Array(elem Ty, n Ty) Ty     { return intern(&TyS{Kind: TArray, Elem: elem, Len: n}) }
func ArrayN(elem Ty, n uint64) Ty { return Array(elem, ConstVal(n)) }
func Slice(elem Ty) Ty            { return intern(&TyS{Kind: TSlice, Elem: elem}) }
func Ref(elem Ty, mut bool) Ty    { return intern(&TyS{Kind: TRef, Elem: elem, Mut: mut}) }
func Ptr(elem Ty, mut bool) Ty    { return intern(&TyS{Kind: TPtr, Elem: elem, Mut: mut}) }
func Adt(d *Def, args []Ty) Ty    { return intern(&TyS{Kind: TAdt, Def: d, Args: args}) }
func FnDefTy(d *Def, args []Ty) Ty {
	return intern(&TyS{Kind: TFnDef, Def: d, Args: args})
}
func FnPtr(inputs []Ty, ret Ty, variadic bool) Ty {
	var sub uint8
	if variadic {
		sub = 1
	}
	return intern(&TyS{Kind: TFnPtr, Elems: inputs, Ret: ret, Sub: sub})
}
func Closure(d *Def, id int, args []Ty) Ty {
	return intern(&TyS{Kind: TClosure, Def: d, Index: id, Args: args})
}
func Dyn(trait *Def, args []Ty, bindings []Binding, auto uint8) Ty {
	return intern(&TyS{Kind: TDyn, Def: trait, Args: args, Bindings: bindings, Sub: auto})
}
func Param(idx int, name string) Ty { return intern(&TyS{Kind: TParam, Index: idx, Name: name}) }
func ConstParam(idx int, name string) Ty {
	return intern(&TyS{Kind: TParam, Index: idx, Name: name, Sub: 1})
}
func Projection(trait *Def, args []Ty, name string) Ty {
	return intern(&TyS{Kind: TProjection, Def: trait, Args: args, Name: name})
}
func Opaque(fn *Def, args []Ty, idx int) Ty {
	return intern(&TyS{Kind: TOpaque, Def: fn, Args: args, Index: idx})
}
func ConstVal(v uint64) Ty { return intern(&TyS{Kind: TConst, Val: v}) }

var inferCounter int
var inferMu sync.Mutex

// NewInfer creates a fresh inference variable (kind 0 general, 1 int, 2 float).
func NewInfer(kind uint8) Ty {
	inferMu.Lock()
	inferCounter++
	id := inferCounter
	inferMu.Unlock()
	return intern(&TyS{Kind: TInfer, Index: id, Sub: kind})
}

// ---------------------------------------------------------------------------
// Predicates

func (t *TyS) IsUnit() bool     { return t.Kind == TTuple && len(t.Elems) == 0 }
func (t *TyS) IsInteger() bool  { return t.Kind == TInt || (t.Kind == TInfer && t.Sub == 1) }
func (t *TyS) IsFloat() bool    { return t.Kind == TFloat || (t.Kind == TInfer && t.Sub == 2) }
func (t *TyS) IsNumeric() bool  { return t.IsInteger() || t.IsFloat() }
func (t *TyS) IsSigned() bool   { return t.Kind == TInt && IntTy(t.Sub).Signed() }
func (t *TyS) IsScalar() bool {
	switch t.Kind {
	case TBool, TChar, TInt, TFloat, TPtr, TFnPtr:
		return true
	}
	return false
}
func (t *TyS) IsRefLike() bool { return t.Kind == TRef || t.Kind == TPtr }

// IsUnsized reports whether a type is dynamically sized (str, [T], dyn Trait).
func (t *TyS) IsUnsized() bool {
	return t.Kind == TStr || t.Kind == TSlice || t.Kind == TDyn
}

// IntKind returns the IntTy of an integer type.
func (t *TyS) IntKind() IntTy { return IntTy(t.Sub) }

// ---------------------------------------------------------------------------
// Folding / substitution

// Fold rebuilds a type, applying f bottom-up-first: f is called on each
// node; if f returns non-nil, that replaces the node (and is not recursed into).
func Fold(t Ty, f func(Ty) Ty) Ty {
	if t == nil {
		return nil
	}
	if r := f(t); r != nil {
		return r
	}
	switch t.Kind {
	case TBool, TChar, TInt, TFloat, TStr, TNever, TParam, TInfer, TConst, TError:
		return t
	}
	n := *t
	changed := false
	fold1 := func(x Ty) Ty {
		if x == nil {
			return nil
		}
		y := Fold(x, f)
		if y != x {
			changed = true
		}
		return y
	}
	foldList := func(xs []Ty) []Ty {
		if len(xs) == 0 {
			return xs
		}
		var out []Ty
		for i, x := range xs {
			y := Fold(x, f)
			if y != x && out == nil {
				out = make([]Ty, len(xs))
				copy(out, xs[:i])
			}
			if out != nil {
				out[i] = y
			}
		}
		if out != nil {
			changed = true
			return out
		}
		return xs
	}
	n.Elem = fold1(t.Elem)
	n.Len = fold1(t.Len)
	n.Ret = fold1(t.Ret)
	n.Elems = foldList(t.Elems)
	n.Args = foldList(t.Args)
	if len(t.Bindings) > 0 {
		var nb []Binding
		for i, b := range t.Bindings {
			y := Fold(b.Ty, f)
			if y != b.Ty && nb == nil {
				nb = make([]Binding, len(t.Bindings))
				copy(nb, t.Bindings[:i])
			}
			if nb != nil {
				nb[i] = Binding{b.Name, y}
			}
		}
		if nb != nil {
			changed = true
			n.Bindings = nb
		}
	}
	if !changed {
		return t
	}
	n.key = ""
	n.flags = 0
	return intern(&n)
}

// Walk visits every node of t (pre-order). Returning false stops descent.
func Walk(t Ty, f func(Ty) bool) {
	if t == nil || !f(t) {
		return
	}
	Walk(t.Elem, f)
	Walk(t.Len, f)
	Walk(t.Ret, f)
	for _, x := range t.Elems {
		Walk(x, f)
	}
	for _, x := range t.Args {
		Walk(x, f)
	}
	for _, b := range t.Bindings {
		Walk(b.Ty, f)
	}
}

// Subst replaces TParam(i) with args[i].
func Subst(t Ty, args []Ty) Ty {
	if t == nil || !t.HasParam() {
		return t
	}
	return Fold(t, func(x Ty) Ty {
		if !x.HasParam() {
			return x
		}
		if x.Kind == TParam {
			if x.Index < len(args) && args[x.Index] != nil {
				return args[x.Index]
			}
			return x
		}
		return nil
	})
}

// SubstList applies Subst to each element.
func SubstList(ts []Ty, args []Ty) []Ty {
	out := make([]Ty, len(ts))
	for i, t := range ts {
		out[i] = Subst(t, args)
	}
	return out
}

// ---------------------------------------------------------------------------
// Display

// String renders a type in Rust syntax.
func (t *TyS) String() string {
	var b strings.Builder
	t.write(&b)
	return b.String()
}

func writeList(b *strings.Builder, ts []Ty) {
	for i, x := range ts {
		if i > 0 {
			b.WriteString(", ")
		}
		x.write(b)
	}
}

func (t *TyS) write(b *strings.Builder) {
	switch t.Kind {
	case TTuple:
		b.WriteByte('(')
		writeList(b, t.Elems)
		if len(t.Elems) == 1 {
			b.WriteByte(',')
		}
		b.WriteByte(')')
	case TArray:
		b.WriteByte('[')
		t.Elem.write(b)
		b.WriteString("; ")
		t.Len.write(b)
		b.WriteByte(']')
	case TSlice:
		b.WriteByte('[')
		t.Elem.write(b)
		b.WriteByte(']')
	case TRef:
		b.WriteByte('&')
		if t.Mut {
			b.WriteString("mut ")
		}
		t.Elem.write(b)
	case TPtr:
		if t.Mut {
			b.WriteString("*mut ")
		} else {
			b.WriteString("*const ")
		}
		t.Elem.write(b)
	case TAdt:
		b.WriteString(t.Def.Path())
		if len(t.Args) > 0 {
			b.WriteByte('<')
			writeList(b, t.Args)
			b.WriteByte('>')
		}
	case TFnDef:
		b.WriteString("fn item {")
		b.WriteString(t.Def.Path())
		if len(t.Args) > 0 {
			b.WriteString("::<")
			writeList(b, t.Args)
			b.WriteByte('>')
		}
		b.WriteString("}")
	case TOpaque:
		fmt.Fprintf(b, "impl Trait#%d of %s", t.Index, t.Def.Path())
	case TFnPtr:
		b.WriteString("fn(")
		writeList(b, t.Elems)
		b.WriteByte(')')
		if !t.Ret.IsUnit() {
			b.WriteString(" -> ")
			t.Ret.write(b)
		}
	case TClosure:
		fmt.Fprintf(b, "{closure#%d}", t.Index)
	case TDyn:
		b.WriteString("dyn ")
		if t.Def != nil {
			b.WriteString(t.Def.Name)
		}
		if len(t.Args) > 0 || len(t.Bindings) > 0 {
			b.WriteByte('<')
			writeList(b, t.Args)
			for i, bd := range t.Bindings {
				if i > 0 || len(t.Args) > 0 {
					b.WriteString(", ")
				}
				b.WriteString(bd.Name)
				b.WriteString(" = ")
				bd.Ty.write(b)
			}
			b.WriteByte('>')
		}
	case TParam:
		b.WriteString(t.Name)
	case TInfer:
		switch t.Sub {
		case 1:
			b.WriteString("{integer}")
		case 2:
			b.WriteString("{float}")
		default:
			b.WriteString("_")
		}
	case TProjection:
		b.WriteByte('<')
		t.Args[0].write(b)
		b.WriteString(" as ")
		b.WriteString(t.Def.Name)
		if len(t.Args) > 1 {
			b.WriteByte('<')
			writeList(b, t.Args[1:])
			b.WriteByte('>')
		}
		b.WriteString(">::")
		b.WriteString(t.Name)
	case TConst:
		b.WriteString(strconv.FormatUint(t.Val, 10))
	default:
		b.WriteString(t.key)
	}
}

// TraitRef is a reference to a trait with arguments; Args[0] is Self.
type TraitRef struct {
	Trait *Def
	Args  []Ty
}

func (tr *TraitRef) Self() Ty { return tr.Args[0] }

func (tr *TraitRef) String() string {
	var b strings.Builder
	b.WriteString("<")
	tr.Args[0].write(&b)
	b.WriteString(" as ")
	b.WriteString(tr.Trait.Path())
	if len(tr.Args) > 1 {
		b.WriteByte('<')
		writeList(&b, tr.Args[1:])
		b.WriteByte('>')
	}
	b.WriteString(">")
	return b.String()
}

// Subst substitutes params in the trait ref.
func (tr *TraitRef) Subst(args []Ty) *TraitRef {
	return &TraitRef{Trait: tr.Trait, Args: SubstList(tr.Args, args)}
}
