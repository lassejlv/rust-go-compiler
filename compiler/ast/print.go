package ast

import (
	"fmt"
	"strconv"
	"strings"

	"rustgo/compiler/lexer"
)

// This file pretty-prints AST fragments back to Rust source text. Printed
// types, paths, generics and bounds reparse to an equivalent AST; expressions
// and patterns are printed on a best-effort basis (with parentheses inserted
// wherever precedence requires them).

// TypeString prints a type.
func TypeString(t Type) string {
	var p printer
	p.typ(t)
	return p.String()
}

// PathString prints a path (generic args included, type style: `Vec<T>`).
func PathString(path *Path) string {
	var p printer
	p.path(path, false)
	return p.String()
}

// ExprPathString prints a path in expression style (`Vec::<T>::new`).
func ExprPathString(path *Path) string {
	var p printer
	p.path(path, true)
	return p.String()
}

// GenericArgsString prints `<A, B, Item = C>` or `(A, B) -> C`.
func GenericArgsString(ga *GenericArgs) string {
	var p printer
	p.genericArgs(ga)
	return p.String()
}

// GenericsParamsString prints the generic parameter list with bounds, e.g.
// `<'a, T: Clone + 'a, const N: usize>`. Defaults are omitted (so the result
// can be used on impl blocks). Returns "" if there are no parameters.
func GenericsParamsString(g *Generics) string {
	if g == nil || len(g.Params) == 0 {
		return ""
	}
	var p printer
	p.s("<")
	for i, gp := range g.Params {
		if i > 0 {
			p.s(", ")
		}
		p.genericParam(gp)
	}
	p.s(">")
	return p.String()
}

// GenericsArgsString prints the generic parameters as arguments, e.g.
// `<'a, T, N>`. Returns "" if there are no parameters.
func GenericsArgsString(g *Generics) string {
	if g == nil || len(g.Params) == 0 {
		return ""
	}
	var parts []string
	for _, gp := range g.Params {
		parts = append(parts, gp.Name.Name)
	}
	return "<" + strings.Join(parts, ", ") + ">"
}

// WhereString prints the where clause as `where A: B, 'a: 'b` (no leading
// or trailing space). Returns "" if there are no predicates.
func WhereString(g *Generics) string {
	if g == nil || len(g.Where) == 0 {
		return ""
	}
	var parts []string
	for _, w := range g.Where {
		parts = append(parts, WherePredicateString(w))
	}
	return "where " + strings.Join(parts, ", ")
}

// WherePredicateString prints a single where predicate (`T: Clone`).
func WherePredicateString(w *WherePredicate) string {
	var p printer
	if len(w.ForLifetimes) > 0 {
		p.s("for<" + strings.Join(w.ForLifetimes, ", ") + "> ")
	}
	if w.Lifetime != "" {
		p.s(w.Lifetime)
	} else {
		p.typ(w.Ty)
	}
	p.s(": ")
	p.bounds(w.Bounds)
	return p.String()
}

// BoundString prints a single bound.
func BoundString(b *Bound) string {
	var p printer
	p.bound(b)
	return p.String()
}

// BoundsString prints `A + B + 'a`.
func BoundsString(bs []*Bound) string {
	var p printer
	p.bounds(bs)
	return p.String()
}

// ExprString prints an expression.
func ExprString(e Expr) string {
	var p printer
	p.expr(e, 0)
	return p.String()
}

// PatString prints a pattern.
func PatString(pt Pat) string {
	var p printer
	p.pat(pt)
	return p.String()
}

// BlockString prints a block.
func BlockString(b *Block) string {
	var p printer
	p.block(b)
	return p.String()
}

// TokenTreesString prints token trees as source text.
func TokenTreesString(tts []TokenTree) string {
	var p printer
	p.tokenTrees(tts)
	return p.String()
}

// TokenString prints a single token as Rust source.
func TokenString(t lexer.Token) string {
	switch t.Kind {
	case lexer.EOF:
		return ""
	case lexer.Ident:
		if t.Raw {
			return "r#" + t.Text
		}
		return t.Text
	case lexer.Str:
		return QuoteStr(t.Text) + t.Suffix
	case lexer.ByteStr:
		return "b" + quoteBytes(t.Text, '"') + t.Suffix
	case lexer.CStr:
		return "c" + quoteBytes(t.Text, '"') + t.Suffix
	case lexer.Char:
		return QuoteChar(t.Text) + t.Suffix
	case lexer.Byte:
		return "b" + quoteBytes(t.Text, '\'') + t.Suffix
	case lexer.Int, lexer.Float:
		return t.Text + t.Suffix
	case lexer.DocComment:
		if t.Inner {
			return "#![doc = " + QuoteStr(t.Text) + "]"
		}
		return "#[doc = " + QuoteStr(t.Text) + "]"
	}
	return t.Text
}

// QuoteStr returns s as a Rust string literal.
func QuoteStr(s string) string {
	var b strings.Builder
	b.WriteByte('"')
	for _, r := range s {
		writeEscapedRune(&b, r, '"')
	}
	b.WriteByte('"')
	return b.String()
}

// QuoteChar returns s (a single char) as a Rust char literal.
func QuoteChar(s string) string {
	var b strings.Builder
	b.WriteByte('\'')
	for _, r := range s {
		writeEscapedRune(&b, r, '\'')
	}
	b.WriteByte('\'')
	return b.String()
}

func writeEscapedRune(b *strings.Builder, r rune, quote rune) {
	switch r {
	case '\n':
		b.WriteString(`\n`)
	case '\r':
		b.WriteString(`\r`)
	case '\t':
		b.WriteString(`\t`)
	case '\\':
		b.WriteString(`\\`)
	case 0:
		b.WriteString(`\0`)
	case quote:
		b.WriteByte('\\')
		b.WriteRune(r)
	default:
		if r < 0x20 || r == 0x7f {
			fmt.Fprintf(b, `\x%02x`, r)
		} else if r >= 0x80 && !strconv.IsPrint(r) {
			fmt.Fprintf(b, `\u{%x}`, r)
		} else {
			b.WriteRune(r)
		}
	}
}

func quoteBytes(s string, quote byte) string {
	var b strings.Builder
	b.WriteByte(quote)
	for i := 0; i < len(s); i++ {
		c := s[i]
		switch c {
		case '\n':
			b.WriteString(`\n`)
		case '\r':
			b.WriteString(`\r`)
		case '\t':
			b.WriteString(`\t`)
		case '\\':
			b.WriteString(`\\`)
		case 0:
			b.WriteString(`\0`)
		case quote:
			b.WriteByte('\\')
			b.WriteByte(c)
		default:
			if c < 0x20 || c >= 0x7f {
				fmt.Fprintf(&b, `\x%02x`, c)
			} else {
				b.WriteByte(c)
			}
		}
	}
	b.WriteByte(quote)
	return b.String()
}

// ---------------------------------------------------------------------------

type printer struct {
	b        strings.Builder
	noStruct bool // parenthesize struct literals (if/while/match heads)
}

func (p *printer) String() string { return p.b.String() }
func (p *printer) s(x string)     { p.b.WriteString(x) }

func (p *printer) path(path *Path, exprStyle bool) {
	if path == nil {
		return
	}
	if path.Global {
		p.s("::")
	}
	p.segments(path.Segments, exprStyle)
}

func (p *printer) segments(segs []*PathSegment, exprStyle bool) {
	for i, seg := range segs {
		if i > 0 {
			p.s("::")
		}
		p.s(identStr(seg.Ident.Name))
		if seg.Args != nil {
			if exprStyle && !seg.Args.Parenthesized {
				p.s("::")
			}
			p.genericArgs(seg.Args)
		}
	}
}

// qpath prints `<T as Trait>::rest`.
func (p *printer) qpath(q *QSelf, path *Path, exprStyle bool) {
	p.s("<")
	p.typ(q.Ty)
	if q.Position > 0 {
		p.s(" as ")
		if path.Global {
			p.s("::")
		}
		p.segments(path.Segments[:q.Position], false)
	}
	p.s(">")
	if q.Position < len(path.Segments) {
		p.s("::")
		p.segments(path.Segments[q.Position:], exprStyle)
	}
}

func (p *printer) genericArgs(ga *GenericArgs) {
	if ga.Parenthesized {
		p.s("(")
		for i, t := range ga.Inputs {
			if i > 0 {
				p.s(", ")
			}
			p.typ(t)
		}
		p.s(")")
		if ga.Output != nil {
			p.s(" -> ")
			p.typNoPlus(ga.Output)
		}
		return
	}
	p.s("<")
	n := 0
	sep := func() {
		if n > 0 {
			p.s(", ")
		}
		n++
	}
	for _, a := range ga.Args {
		sep()
		switch {
		case a.Lifetime != "":
			p.s(a.Lifetime)
		case a.Type != nil:
			p.typ(a.Type)
		case a.Const != nil:
			p.constArg(a.Const)
		}
	}
	for _, b := range ga.Bindings {
		sep()
		p.s(identStr(b.Name.Name))
		if b.Ty != nil {
			p.s(" = ")
			p.typ(b.Ty)
		} else {
			p.s(": ")
			p.bounds(b.Bounds)
		}
	}
	p.s(">")
}

func (p *printer) constArg(e Expr) {
	switch x := e.(type) {
	case *LitExpr, *BlockExpr:
		p.expr(e, 0)
	case *UnaryExpr:
		if _, ok := x.X.(*LitExpr); ok && x.Op == UnNeg {
			p.expr(e, 0)
			return
		}
		p.s("{ ")
		p.expr(e, 0)
		p.s(" }")
	case *PathExpr:
		if x.QSelf == nil && len(x.Path.Segments) == 1 && x.Path.Segments[0].Args == nil {
			p.s(identStr(x.Path.Segments[0].Ident.Name))
			return
		}
		p.s("{ ")
		p.expr(e, 0)
		p.s(" }")
	default:
		p.s("{ ")
		p.expr(e, 0)
		p.s(" }")
	}
}

func (p *printer) bounds(bs []*Bound) {
	for i, b := range bs {
		if i > 0 {
			p.s(" + ")
		}
		p.bound(b)
	}
}

func (p *printer) bound(b *Bound) {
	if b.Lifetime != "" {
		p.s(b.Lifetime)
		return
	}
	if len(b.ForLifetimes) > 0 {
		p.s("for<" + strings.Join(b.ForLifetimes, ", ") + "> ")
	}
	if b.Maybe {
		p.s("?")
	}
	if b.MaybeConst {
		p.s("~const ")
	}
	p.path(b.Path, false)
}

func (p *printer) genericParam(gp *GenericParam) {
	switch gp.Kind {
	case GPLifetime:
		p.s(gp.Name.Name)
		if len(gp.Bounds) > 0 {
			p.s(": ")
			p.bounds(gp.Bounds)
		}
	case GPType:
		p.s(gp.Name.Name)
		if len(gp.Bounds) > 0 {
			p.s(": ")
			p.bounds(gp.Bounds)
		}
	case GPConst:
		p.s("const " + gp.Name.Name + ": ")
		p.typ(gp.ConstTy)
	}
}

// typNoPlus prints a type in a position where a `+` would be ambiguous.
func (p *printer) typNoPlus(t Type) {
	switch x := t.(type) {
	case *DynTraitType:
		if len(x.Bounds) > 1 {
			p.s("(")
			p.typ(t)
			p.s(")")
			return
		}
	case *ImplTraitType:
		if len(x.Bounds) > 1 {
			p.s("(")
			p.typ(t)
			p.s(")")
			return
		}
	}
	p.typ(t)
}

func (p *printer) typ(t Type) {
	switch x := t.(type) {
	case nil:
		p.s("()")
	case *PathType:
		if x.QSelf != nil {
			p.qpath(x.QSelf, x.Path, false)
		} else {
			p.path(x.Path, false)
		}
	case *RefType:
		p.s("&")
		if x.Lifetime != "" {
			p.s(x.Lifetime + " ")
		}
		if x.Mut {
			p.s("mut ")
		}
		p.typNoPlus(x.Elem)
	case *PtrType:
		if x.Mut {
			p.s("*mut ")
		} else {
			p.s("*const ")
		}
		p.typNoPlus(x.Elem)
	case *SliceType:
		p.s("[")
		p.typ(x.Elem)
		p.s("]")
	case *ArrayType:
		p.s("[")
		p.typ(x.Elem)
		p.s("; ")
		p.expr(x.Len, 0)
		p.s("]")
	case *TupleType:
		p.s("(")
		for i, e := range x.Elems {
			if i > 0 {
				p.s(", ")
			}
			p.typ(e)
		}
		if len(x.Elems) == 1 {
			p.s(",")
		}
		p.s(")")
	case *FnPtrType:
		if x.Unsafe {
			p.s("unsafe ")
		}
		if x.Abi != "" {
			p.s("extern " + QuoteStr(x.Abi) + " ")
		}
		p.s("fn(")
		for i, in := range x.Inputs {
			if i > 0 {
				p.s(", ")
			}
			p.typ(in)
		}
		if x.Variadic {
			if len(x.Inputs) > 0 {
				p.s(", ")
			}
			p.s("...")
		}
		p.s(")")
		if x.Output != nil {
			p.s(" -> ")
			p.typNoPlus(x.Output)
		}
	case *NeverType:
		p.s("!")
	case *InferType:
		p.s("_")
	case *ImplTraitType:
		p.s("impl ")
		p.bounds(x.Bounds)
	case *DynTraitType:
		p.s("dyn ")
		p.bounds(x.Bounds)
	case *ParenType:
		p.s("(")
		p.typ(x.Elem)
		p.s(")")
	case *MacroType:
		p.macCall(x.Mac)
	default:
		p.s(fmt.Sprintf("/* unknown type %T */", t))
	}
}

// ---------------------------------------------------------------------------
// Expressions

// Expression precedences used by the printer (see parser).
const (
	ppJump    = 1 // return/break/closure: lowest, prefix-ish
	ppAssign  = 2
	ppRange   = 3
	ppOr      = 4
	ppAnd     = 5
	ppCmp     = 6
	ppBitOr   = 7
	ppBitXor  = 8
	ppBitAnd  = 9
	ppShift   = 10
	ppAdd     = 11
	ppMul     = 12
	ppCast    = 13
	ppUnary   = 14
	ppPostfix = 15
	ppPrimary = 16
)

func binPrec(op BinOp) int {
	switch op {
	case BinMul, BinDiv, BinRem:
		return ppMul
	case BinAdd, BinSub:
		return ppAdd
	case BinShl, BinShr:
		return ppShift
	case BinBitAnd:
		return ppBitAnd
	case BinBitXor:
		return ppBitXor
	case BinBitOr:
		return ppBitOr
	case BinAnd:
		return ppAnd
	case BinOr:
		return ppOr
	}
	return ppCmp
}

func exprPrec(e Expr) int {
	switch x := e.(type) {
	case *BinaryExpr:
		return binPrec(x.Op)
	case *AssignExpr, *AssignOpExpr:
		return ppAssign
	case *RangeExpr:
		return ppRange
	case *CastExpr:
		return ppCast
	case *UnaryExpr, *RefExpr:
		return ppUnary
	case *CallExpr, *MethodCallExpr, *FieldExpr, *IndexExpr, *TryExpr, *AwaitExpr:
		return ppPostfix
	case *LetExpr:
		// `let` scrutinees bind tighter than `&&`, so let-chains print
		// without parentheses (which would be invalid).
		return ppCmp
	case *ReturnExpr, *BreakExpr, *ClosureExpr:
		return ppJump
	}
	return ppPrimary
}

// expr prints e, wrapping it in parentheses if its precedence is below min.
func (p *printer) expr(e Expr, min int) {
	if e == nil {
		return
	}
	if exprPrec(e) < min {
		p.s("(")
		old := p.noStruct
		p.noStruct = false
		p.expr(e, 0)
		p.noStruct = old
		p.s(")")
		return
	}
	switch x := e.(type) {
	case *LitExpr:
		p.lit(x)
	case *PathExpr:
		if x.QSelf != nil {
			p.qpath(x.QSelf, x.Path, true)
		} else {
			p.path(x.Path, true)
		}
	case *UnaryExpr:
		switch x.Op {
		case UnNeg:
			p.s("-")
		case UnNot:
			p.s("!")
		case UnDeref:
			p.s("*")
		}
		// avoid `--x` lexing issues: `- -x`
		if u, ok := x.X.(*UnaryExpr); ok && u.Op == UnNeg && x.Op == UnNeg {
			p.s(" ")
		}
		if l, ok := x.X.(*LitExpr); ok && x.Op == UnNeg && (l.Kind == LitInt || l.Kind == LitFloat) {
			p.lit(l)
			return
		}
		p.expr(x.X, ppUnary)
	case *RefExpr:
		p.s("&")
		if x.Raw {
			if x.Mut {
				p.s("raw mut ")
			} else {
				p.s("raw const ")
			}
		} else if x.Mut {
			p.s("mut ")
		}
		p.expr(x.X, ppUnary)
	case *BinaryExpr:
		pr := binPrec(x.Op)
		lmin, rmin := pr, pr+1
		if pr == ppCmp {
			lmin = pr + 1
		}
		if _, isCast := x.L.(*CastExpr); isCast && (x.Op == BinLt || x.Op == BinShl) {
			// `x as T < y` would start generic args after the type.
			lmin = ppCast + 1
		}
		p.expr(x.L, lmin)
		p.s(" " + x.Op.String() + " ")
		p.expr(x.R, rmin)
	case *AssignExpr:
		p.expr(x.L, ppAssign+1)
		p.s(" = ")
		p.expr(x.R, ppAssign)
	case *AssignOpExpr:
		p.expr(x.L, ppAssign+1)
		p.s(" " + x.Op.String() + "= ")
		p.expr(x.R, ppAssign)
	case *CastExpr:
		p.expr(x.X, ppCast)
		p.s(" as ")
		p.typNoPlus(x.Ty)
	case *CallExpr:
		p.expr(x.Fn, ppPostfix)
		p.args(x.Args)
	case *MethodCallExpr:
		p.receiver(x.Receiver)
		p.s("." + identStr(x.Method.Name))
		if x.Generics != nil {
			p.s("::")
			p.genericArgs(x.Generics)
		}
		p.args(x.Args)
	case *FieldExpr:
		p.receiver(x.X)
		p.s("." + identStr(x.Name.Name))
	case *IndexExpr:
		p.expr(x.X, ppPostfix)
		p.s("[")
		p.withStruct(func() { p.expr(x.Index, 0) })
		p.s("]")
	case *TryExpr:
		p.expr(x.X, ppPostfix)
		p.s("?")
	case *AwaitExpr:
		p.receiver(x.X)
		p.s(".await")
	case *ParenExpr:
		p.s("(")
		p.withStruct(func() { p.expr(x.X, 0) })
		p.s(")")
	case *TupleExpr:
		p.s("(")
		p.withStruct(func() {
			for i, el := range x.Elems {
				if i > 0 {
					p.s(", ")
				}
				p.expr(el, 0)
			}
		})
		if len(x.Elems) == 1 {
			p.s(",")
		}
		p.s(")")
	case *ArrayExpr:
		p.s("[")
		p.withStruct(func() {
			for i, el := range x.Elems {
				if i > 0 {
					p.s(", ")
				}
				p.expr(el, 0)
			}
		})
		p.s("]")
	case *RepeatExpr:
		p.s("[")
		p.withStruct(func() {
			p.expr(x.Elem, 0)
			p.s("; ")
			p.expr(x.Count, 0)
		})
		p.s("]")
	case *BlockExpr:
		if x.Label != "" {
			p.s(x.Label + ": ")
		}
		if x.Const {
			p.s("const ")
		}
		if x.Async {
			p.s("async move ")
		}
		p.block(x.Block)
	case *IfExpr:
		p.s("if ")
		p.cond(x.Cond)
		p.s(" ")
		p.block(x.Then)
		if x.Else != nil {
			p.s(" else ")
			p.expr(x.Else, 0)
		}
	case *LetExpr:
		p.s("let ")
		p.pat(x.Pat)
		p.s(" = ")
		p.expr(x.Expr, ppCmp)
	case *WhileExpr:
		if x.Label != "" {
			p.s(x.Label + ": ")
		}
		p.s("while ")
		p.cond(x.Cond)
		p.s(" ")
		p.block(x.Body)
	case *LoopExpr:
		if x.Label != "" {
			p.s(x.Label + ": ")
		}
		p.s("loop ")
		p.block(x.Body)
	case *ForExpr:
		if x.Label != "" {
			p.s(x.Label + ": ")
		}
		p.s("for ")
		p.pat(x.Pat)
		p.s(" in ")
		p.cond(x.Iter)
		p.s(" ")
		p.block(x.Body)
	case *MatchExpr:
		p.s("match ")
		p.cond(x.Scrutinee)
		p.s(" {")
		for _, a := range x.Arms {
			p.s(" ")
			p.pat(a.Pat)
			if a.Guard != nil {
				p.s(" if ")
				p.expr(a.Guard, 0)
			}
			p.s(" => ")
			p.withStruct(func() { p.expr(a.Body, 0) })
			p.s(",")
		}
		p.s(" }")
	case *ClosureExpr:
		if x.Async {
			p.s("async ")
		}
		if x.Move {
			p.s("move ")
		}
		p.s("|")
		for i, cp := range x.Params {
			if i > 0 {
				p.s(", ")
			}
			p.pat(cp.Pat)
			if cp.Ty != nil {
				p.s(": ")
				p.typNoPlus(cp.Ty)
			}
		}
		p.s("| ")
		if x.Ret != nil {
			p.s("-> ")
			p.typNoPlus(x.Ret)
			p.s(" ")
			if _, ok := x.Body.(*BlockExpr); !ok {
				p.s("{ ")
				p.expr(x.Body, 0)
				p.s(" }")
				return
			}
		}
		p.expr(x.Body, 0)
	case *BreakExpr:
		p.s("break")
		if x.Label != "" {
			p.s(" " + x.Label)
		}
		if x.Value != nil {
			p.s(" ")
			p.expr(x.Value, 0)
		}
	case *ContinueExpr:
		p.s("continue")
		if x.Label != "" {
			p.s(" " + x.Label)
		}
	case *ReturnExpr:
		p.s("return")
		if x.Value != nil {
			p.s(" ")
			p.expr(x.Value, 0)
		}
	case *StructExpr:
		if p.noStruct {
			p.s("(")
			p.noStruct = false
			p.structExpr(x)
			p.noStruct = true
			p.s(")")
			return
		}
		p.structExpr(x)
	case *RangeExpr:
		if x.Start != nil {
			p.expr(x.Start, ppRange+1)
		}
		if x.Inclusive {
			p.s("..=")
		} else {
			p.s("..")
		}
		if x.End != nil {
			p.expr(x.End, ppRange+1)
		}
	case *MacCallExpr:
		p.macCall(x.Mac)
	case *UnderscoreExpr:
		p.s("_")
	case *FormatArgsExpr:
		p.s("format_args!(")
		p.s(QuoteStr(strings.Join(x.Pieces, "{}")))
		for _, a := range x.Args {
			p.s(", ")
			p.expr(a, 0)
		}
		p.s(")")
	default:
		p.s(fmt.Sprintf("/* unknown expr %T */", e))
	}
}

func (p *printer) withStruct(f func()) {
	old := p.noStruct
	p.noStruct = false
	f()
	p.noStruct = old
}

func (p *printer) cond(e Expr) {
	old := p.noStruct
	p.noStruct = true
	p.expr(e, 0)
	p.noStruct = old
}

// receiver prints the receiver of a method call / field access. Integer
// literals need parentheses (`1.foo()` is fine but `1.0` would be a float).
func (p *printer) receiver(e Expr) {
	if l, ok := e.(*LitExpr); ok && (l.Kind == LitInt || l.Kind == LitFloat) && l.Suffix == "" {
		p.s("(")
		p.lit(l)
		p.s(")")
		return
	}
	p.expr(e, ppPostfix)
}

func (p *printer) args(args []Expr) {
	p.s("(")
	p.withStruct(func() {
		for i, a := range args {
			if i > 0 {
				p.s(", ")
			}
			p.expr(a, 0)
		}
	})
	p.s(")")
}

func (p *printer) structExpr(x *StructExpr) {
	if x.QSelf != nil {
		p.qpath(x.QSelf, x.Path, true)
	} else {
		p.path(x.Path, true)
	}
	p.s(" {")
	for i, f := range x.Fields {
		if i > 0 {
			p.s(",")
		}
		p.s(" ")
		if f.Shorthand {
			p.s(identStr(f.Name.Name))
			continue
		}
		p.s(identStr(f.Name.Name) + ": ")
		p.expr(f.Expr, 0)
	}
	if x.Base != nil || x.Rest {
		if len(x.Fields) > 0 {
			p.s(",")
		}
		p.s(" ..")
		if x.Base != nil {
			p.expr(x.Base, 0)
		}
	}
	p.s(" }")
}

func (p *printer) lit(l *LitExpr) {
	switch l.Kind {
	case LitInt:
		p.s(l.Value + l.Suffix)
	case LitFloat:
		v := l.Value
		if l.Suffix == "" && !strings.ContainsAny(v, ".eE") {
			v += ".0"
		}
		p.s(v + l.Suffix)
	case LitStr:
		p.s(QuoteStr(l.Value))
	case LitByteStr:
		p.s("b" + quoteBytes(l.Value, '"'))
	case LitCStr:
		p.s("c" + quoteBytes(l.Value, '"'))
	case LitChar:
		p.s(QuoteChar(l.Value))
	case LitByte:
		p.s("b" + quoteBytes(l.Value, '\''))
	case LitBool:
		p.s(l.Value)
	}
}

func (p *printer) macCall(m *MacCall) {
	p.path(m.Path, true)
	p.s("!")
	open, close := "(", ")"
	switch m.Delim {
	case '[':
		open, close = "[", "]"
	case '{':
		open, close = "{ ", " }"
	}
	p.s(open)
	p.tokenTrees(m.Args)
	p.s(close)
}

func (p *printer) tokenTrees(tts []TokenTree) {
	for i, tt := range tts {
		if i > 0 {
			prev := tts[i-1]
			if prev.Delim != 0 || !prev.Tok.JointNext {
				p.s(" ")
			}
		}
		if tt.Delim == 0 {
			p.s(TokenString(tt.Tok))
			continue
		}
		var close string
		switch tt.Delim {
		case '(':
			close = ")"
		case '[':
			close = "]"
		case '{':
			close = "}"
		}
		p.s(string(tt.Delim))
		p.tokenTrees(tt.Inner)
		p.s(close)
	}
}

func (p *printer) block(b *Block) {
	if b == nil {
		p.s("{}")
		return
	}
	if b.Unsafe {
		p.s("unsafe ")
	}
	old := p.noStruct
	p.noStruct = false
	defer func() { p.noStruct = old }()
	if len(b.Stmts) == 0 {
		p.s("{}")
		return
	}
	p.s("{")
	for _, st := range b.Stmts {
		p.s(" ")
		p.stmt(st)
	}
	p.s(" }")
}

func (p *printer) stmt(st Stmt) {
	switch x := st.(type) {
	case *LetStmt:
		p.s("let ")
		p.pat(x.Pat)
		if x.Ty != nil {
			p.s(": ")
			p.typ(x.Ty)
		}
		if x.Init != nil {
			p.s(" = ")
			p.expr(x.Init, 0)
		}
		if x.Else != nil {
			p.s(" else ")
			p.block(x.Else)
		}
		p.s(";")
	case *ExprStmt:
		// Parenthesize statement-position expressions that would otherwise
		// be parsed differently (e.g. a struct literal or a binary expr
		// starting with a block-like expression).
		if !isBlockLikeExpr(x.Expr) && startsWithBlockLike(x.Expr) {
			p.s("(")
			p.expr(x.Expr, 0)
			p.s(")")
		} else {
			p.expr(x.Expr, 0)
		}
		if x.Semi {
			p.s(";")
		}
	case *MacStmt:
		p.macCall(x.Mac)
		if x.Semi && x.Mac.Delim != '{' {
			p.s(";")
		}
	case *ItemStmt:
		if x.Item != nil {
			p.s("/* item " + x.Item.Name.Name + " */")
		}
	case *EmptyStmt:
		p.s(";")
	}
}

// ---------------------------------------------------------------------------
// Patterns

func (p *printer) pat(pt Pat) {
	switch x := pt.(type) {
	case nil:
	case *WildPat:
		p.s("_")
	case *IdentPat:
		if x.ByRef {
			p.s("ref ")
		}
		if x.Mut {
			p.s("mut ")
		}
		p.s(identStr(x.Name.Name))
		if x.Sub != nil {
			p.s(" @ ")
			p.pat(x.Sub)
		}
	case *LitPat:
		p.expr(x.Expr, 0)
	case *RangePat:
		if x.Lo != nil {
			p.expr(x.Lo, 0)
		}
		if x.Inclusive {
			p.s("..=")
		} else {
			p.s("..")
		}
		if x.Hi != nil {
			p.expr(x.Hi, 0)
		}
	case *TuplePat:
		p.s("(")
		for i, e := range x.Elems {
			if i > 0 {
				p.s(", ")
			}
			p.pat(e)
		}
		if len(x.Elems) == 1 {
			if _, ok := x.Elems[0].(*RestPat); !ok {
				p.s(",")
			}
		}
		p.s(")")
	case *TupleStructPat:
		if x.QSelf != nil {
			p.qpath(x.QSelf, x.Path, true)
		} else {
			p.path(x.Path, true)
		}
		p.s("(")
		for i, e := range x.Elems {
			if i > 0 {
				p.s(", ")
			}
			p.pat(e)
		}
		p.s(")")
	case *StructPat:
		if x.QSelf != nil {
			p.qpath(x.QSelf, x.Path, true)
		} else {
			p.path(x.Path, true)
		}
		p.s(" {")
		for i, f := range x.Fields {
			if i > 0 {
				p.s(",")
			}
			p.s(" ")
			if f.Shorthand {
				p.pat(f.Pat)
				continue
			}
			p.s(identStr(f.Name.Name) + ": ")
			p.pat(f.Pat)
		}
		if x.Rest {
			if len(x.Fields) > 0 {
				p.s(",")
			}
			p.s(" ..")
		}
		p.s(" }")
	case *PathPat:
		if x.QSelf != nil {
			p.qpath(x.QSelf, x.Path, true)
		} else {
			p.path(x.Path, true)
		}
	case *RefPat:
		p.s("&")
		if x.Mut {
			p.s("mut ")
		}
		if _, ok := x.Pat.(*RefPat); ok {
			p.s(" ")
		}
		p.pat(x.Pat)
	case *BoxPat:
		p.s("box ")
		p.pat(x.Pat)
	case *SlicePat:
		p.s("[")
		for i, e := range x.Elems {
			if i > 0 {
				p.s(", ")
			}
			p.pat(e)
		}
		p.s("]")
	case *OrPat:
		for i, a := range x.Alts {
			if i > 0 {
				p.s(" | ")
			}
			p.pat(a)
		}
	case *RestPat:
		p.s("..")
	case *ParenPat:
		p.s("(")
		p.pat(x.Pat)
		p.s(")")
	case *MacPat:
		p.macCall(x.Mac)
	default:
		p.s(fmt.Sprintf("/* unknown pat %T */", pt))
	}
}

func isBlockLikeExpr(e Expr) bool {
	switch e.(type) {
	case *BlockExpr, *IfExpr, *MatchExpr, *LoopExpr, *WhileExpr, *ForExpr:
		return true
	}
	return false
}

// startsWithBlockLike reports whether the leftmost operand of e is a
// block-like expression (which would end an expression statement early).
func startsWithBlockLike(e Expr) bool {
	for {
		if isBlockLikeExpr(e) {
			return true
		}
		switch x := e.(type) {
		case *BinaryExpr:
			e = x.L
		case *AssignExpr:
			e = x.L
		case *AssignOpExpr:
			e = x.L
		case *CastExpr:
			e = x.X
		case *RangeExpr:
			if x.Start == nil {
				return false
			}
			e = x.Start
		case *CallExpr:
			e = x.Fn
		case *IndexExpr:
			e = x.X
		case *StructExpr:
			return false
		default:
			return false
		}
	}
}

// identStr prints an identifier, using raw-identifier syntax for keywords
// that cannot appear as plain identifiers.
func identStr(name string) string {
	switch name {
	case "self", "Self", "super", "crate", "_", "$crate":
		return name
	}
	if lexer.Keywords[name] {
		return "r#" + name
	}
	return name
}
