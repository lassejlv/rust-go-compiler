package codegen

import (
	"fmt"
	"math"
	"math/big"
	"rustgo/compiler/lexer"
	"strconv"
	"strings"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
	"rustgo/compiler/typeck"
)

// operand is the result of lowering an expression: either a C value or a
// C lvalue (place). Unsized places (str, [T], dyn) are represented by their
// fat pointer value with unsized=true.
type operand struct {
	s       string
	place   bool
	unsized bool
	flag    string // drop flag of the root (cleared when moved out)
	movable bool   // place can be moved out of (rooted in a local/temp, no ref derefs)
	ty      Ty
	// tlocal/tpath: the place is a tracked field path of a split local
	// (flag is pathFlags(tlocal, tpath))
	tlocal *typeck.Local
	tpath  []int
}

// value lowers e to a C rvalue (applying adjustments, moving out of places).
func (fc *fnGen) value(e ast.Expr) string {
	op := fc.adjusted(e)
	return fc.toValue(op)
}

// coerceValue lowers e as a value of type t (coercions are adjustments
// recorded by typeck; this only handles diverging expressions).
func (fc *fnGen) coerceValue(e ast.Expr, t Ty) string {
	v := fc.value(e)
	if t != nil && fc.aty(e).Kind == TNever && !isZST(t) {
		return fc.dummy(t)
	}
	return v
}

func (fc *fnGen) toValue(op operand) string {
	if op.unsized {
		fc.g.fatalf("codegen: cannot use unsized value of type %s by value", op.ty)
	}
	if op.place && op.flag != "" && op.movable && !fc.g.tcx.IsCopy(op.ty) {
		// moving out: copy first, then clear the root drop flag
		if !isSimple(op.s) || true {
			v := fc.spill(op.s, op.ty)
			fc.line("%s = 0;", op.flag)
			return v
		}
	}
	return op.s
}

// placeOp lowers e as a place (materializing rvalues into temporaries).
func (fc *fnGen) placeOp(e ast.Expr) operand {
	op := fc.adjusted(e)
	return fc.asPlace(op)
}

func (fc *fnGen) asPlace(op operand) operand {
	if op.place || op.unsized {
		return op
	}
	if isZST(op.ty) {
		n := fc.newName("t")
		fc.line("%s %s = %s;", fc.ctype(op.ty), n, op.s)
		return operand{s: n, place: true, ty: op.ty}
	}
	p, f := fc.materialize(op.s, op.ty)
	return operand{s: p, place: true, flag: f, movable: true, ty: op.ty}
}

// addrOf returns a pointer (or fat pointer for unsized places) to e.
func (fc *fnGen) addrOf(op operand) string {
	op = fc.asPlace(op)
	if op.unsized {
		return op.s
	}
	return "(&" + op.s + ")"
}

// adjusted lowers e and applies its adjustments.
func (fc *fnGen) adjusted(e ast.Expr) operand {
	op := fc.operand(e)
	for _, a := range fc.r.Adjust[e.GetID()] {
		op = fc.applyAdjust(op, a)
	}
	return op
}

func (fc *fnGen) applyAdjust(op operand, a typeck.Adjustment) operand {
	cur := op.ty
	switch a.Kind {
	case typeck.AdjDeref:
		switch cur.Kind {
		case TRef, TPtr:
			v := fc.toValue(op)
			if cur.Elem.IsUnsized() {
				return operand{s: v, unsized: true, ty: a.Target}
			}
			return operand{s: "(*" + v + ")", place: true, ty: a.Target}
		case TAdt: // Box
			op = fc.asPlace(op)
			if cur.Args[0].IsUnsized() {
				return operand{s: op.s + ".f0", unsized: true, ty: a.Target}
			}
			return operand{s: "(*" + op.s + ".f0)", place: true, flag: op.flag, movable: op.movable, ty: a.Target}
		}
		fc.g.fatalf("bad deref adjustment on %s", cur)
	case typeck.AdjDerefOverloaded:
		recv := fc.addrOf(op)
		rt := Ref(a.Target, a.Mut)
		v := fc.callResolved(a.Callee.Def, a.Callee.Args, []string{recv}, rt)
		v = fc.spill(v, rt)
		if a.Target.IsUnsized() {
			return operand{s: v, unsized: true, ty: a.Target}
		}
		return operand{s: "(*" + v + ")", place: true, ty: a.Target}
	case typeck.AdjBorrow, typeck.AdjBorrowRaw:
		return operand{s: fc.addrOf(op), ty: a.Target}
	case typeck.AdjUnsize:
		return operand{s: fc.unsize(fc.toValue(op), cur, a.Target), ty: a.Target}
	case typeck.AdjNeverToAny:
		if !op.place {
			if op.s != fc.unit() && !isSimple(op.s) {
				fc.line("(void)%s;", op.s)
			}
		}
		return operand{s: fc.dummy(a.Target), ty: a.Target}
	case typeck.AdjReifyFnPtr:
		return operand{s: fmt.Sprintf("((%s)%s)", fc.ctype(a.Target), fc.g.fnValue(cur)), ty: a.Target}
	case typeck.AdjClosureFnPtr:
		return operand{s: fmt.Sprintf("((%s)%s)", fc.ctype(a.Target), fc.g.closurePtrThunk(cur)), ty: a.Target}
	case typeck.AdjMutToConst:
		return operand{s: fc.toValue(op), ty: a.Target}
	}
	fc.g.fatalf("unsupported adjustment")
	return op
}

// unsize converts a pointer-like value from type `from` to type `to`.
func (fc *fnGen) unsize(v string, from, to Ty) string {
	switch to.Kind {
	case TRef, TPtr:
		return fc.unsizePtr(v, from.Elem, to.Elem)
	case TAdt:
		// Box<T> -> Box<U> (or other smart pointer with one pointer field)
		src := fc.spill(v, from)
		inner := fc.g.fieldTypes(from, 0)[0]
		outer := fc.g.fieldTypes(to, 0)[0]
		conv := fc.unsize(src+".f0", inner, outer)
		fc.g.defineByValue(to)
		return fmt.Sprintf("((%s){ .f0 = %s })", fc.ctype(to), conv)
	}
	fc.g.fatalf("unsupported unsizing %s -> %s", from, to)
	return ""
}

func (fc *fnGen) unsizePtr(v string, src, dst Ty) string {
	if src.IsUnsized() {
		// already a fat pointer: only the metadata changes (upcasts,
		// struct tails)
		f := fc.spillIfNeeded(v, Ptr(src, false))
		return fmt.Sprintf("((Fat){ %s.ptr, %s })", f, fc.g.unsizeMeta(src, dst, f+".meta"))
	}
	if dst.Kind == TAdt {
		return fmt.Sprintf("((Fat){ (void*)(%s), %s })", v, fc.g.unsizeMeta(src, dst, ""))
	}
	switch dst.Kind {
	case TSlice:
		if src.Kind == TArray {
			return fmt.Sprintf("((Fat){ (void*)(%s), %d })", v, fc.g.arrayLen(src))
		}
	case TDyn:
		if src.Kind == TDyn {
			return v
		}
		return fmt.Sprintf("((Fat){ (void*)(%s), (uint64_t)(uintptr_t)%s })", v, fc.g.vtable(src, dst))
	}
	fc.g.fatalf("unsupported pointer unsizing %s -> %s", src, dst)
	return ""
}

// ---------------------------------------------------------------------------
// Unadjusted operands

func (fc *fnGen) operand(e ast.Expr) operand {
	t := fc.ty(e)
	val := func(s string) operand { return operand{s: s, ty: t} }
	if fc.closure != nil && fc.closure.CapExprs != nil {
		if i, ok := fc.closure.CapExprs[e.GetID()]; ok {
			return fc.upvarOperand(i, t)
		}
	}
	switch x := e.(type) {
	case *ast.LitExpr:
		return val(fc.literal(x, t))
	case *ast.ParenExpr:
		op := fc.adjusted(x.X)
		return op
	case *ast.PathExpr:
		return fc.pathOperand(x, t)
	case *ast.UnaryExpr:
		return fc.unaryOperand(x, t)
	case *ast.BinaryExpr:
		return val(fc.binary(x, t))
	case *ast.AssignExpr:
		fc.assign(x)
		return val(fc.unit())
	case *ast.AssignOpExpr:
		fc.assignOp(x)
		return val(fc.unit())
	case *ast.CallExpr:
		return val(fc.callExpr(x, t))
	case *ast.MethodCallExpr:
		return val(fc.methodCall(x, t))
	case *ast.FieldExpr:
		base := fc.asPlace(fc.adjusted(x.X))
		fr := fc.r.Fields[x.ID]
		if base.unsized {
			// field of an unsized-tail struct behind a fat pointer
			fat := base.s
			if !isSimple(fat) {
				fat = fc.spill(fat, Ptr(base.ty, false))
			}
			fc.g.defineType(base.ty)
			sn := fc.g.ctype(base.ty)
			if t.IsUnsized() {
				if !staticTail(t) {
					return operand{s: fmt.Sprintf("((Fat){ (void*)((uint8_t*)%s.ptr + %s), %s.meta })", fat, fc.g.tailOffset(base.ty, fat+".meta"), fat), unsized: true, ty: t}
				}
				return operand{s: fmt.Sprintf("((Fat){ (void*)((%s*)%s.ptr)->f%d, %s.meta })", sn, fat, fr.Index, fat), unsized: true, ty: t}
			}
			return operand{s: fmt.Sprintf("((%s*)%s.ptr)->f%d", sn, fat, fr.Index), place: true, ty: t}
		}
		flag := base.flag
		var tl *typeck.Local
		var tp []int
		if l, path := fc.fieldPath(x); l != nil && fc.tracked(l, path) {
			if _, captured := fc.capIndex[l]; (!captured && !fc.pathCaps[l]) || fc.ownedUpvars[l] {
				flag, tl, tp = fc.pathFlags(l, path), l, path
			}
		}
		return operand{s: fmt.Sprintf("%s.f%d", base.s, fr.Index), place: true, flag: flag, movable: base.movable, ty: t, tlocal: tl, tpath: tp}
	case *ast.IndexExpr:
		return fc.indexOperand(x, t)
	case *ast.RefExpr:
		if !x.Mut && fc.promotable(x.X) && len(fc.r.Adjust[x.X.GetID()]) == 0 {
			it := fc.ty(x.X)
			v := fc.constC(x.X)
			name := fc.newName("prom")
			fc.g.defineByValue(it)
			fmt.Fprintf(&fc.g.statics, "static %s %s = %s;\n", fc.g.ctype(it), name, v)
			return val("(&" + name + ")")
		}
		if it := fc.ty(x.X); !x.Mut && len(fc.r.Adjust[x.X.GetID()]) == 0 && !it.IsUnsized() && !fc.g.needsDrop(it) &&
			((fc.inConst && !isPlaceExpr(x.X)) || fc.promotableDyn(x.X)) {
			// constant borrow of a value computed at run time (const
			// paths, const-fn results): static storage, (re)initialized
			// with the same value on each evaluation
			name := fc.newName("prom")
			fc.g.defineByValue(it)
			fmt.Fprintf(&fc.g.statics, "static %s %s;\n", fc.g.ctype(it), name)
			v := fc.value(x.X)
			if !isZST(it) {
				fc.line("%s = %s;", name, v)
			}
			return val("(&" + name + ")")
		}
		inner := fc.adjusted(x.X)
		if x.Mut {
			// ensure place
		}
		return val(fc.addrOf(inner))
	case *ast.AsmExpr:
		fc.asm(x)
		if t != nil && t.Kind == TNever {
			return val(fc.dummy(t))
		}
		return val(fc.unit())
	case *ast.CastExpr:
		return val(fc.cast(x, t))
	case *ast.BlockExpr:
		if x.Label != "" {
			return val(fc.labeledBlock(x, t))
		}
		return val(fc.blockValue(x.Block, t))
	case *ast.IfExpr:
		return val(fc.ifExpr(x, t))
	case *ast.MatchExpr:
		return val(fc.matchExpr(x, t))
	case *ast.LoopExpr:
		return val(fc.loopExpr(x, t))
	case *ast.WhileExpr:
		return val(fc.whileExpr(x))
	case *ast.ForExpr:
		return val(fc.forExpr(x))
	case *ast.BreakExpr:
		fc.breakExpr(x)
		return val(fc.unit())
	case *ast.ContinueExpr:
		fc.continueExpr(x)
		return val(fc.unit())
	case *ast.ReturnExpr:
		v := fc.unit()
		if x.Value != nil {
			v = fc.coerceValue(x.Value, fc.retTy)
		}
		fc.emitReturn(v)
		return val(fc.unit())
	case *ast.TupleExpr:
		if len(x.Elems) == 0 {
			return val(fc.unit())
		}
		var vs []string
		for i, v := range fc.evalOperands(x.Elems) {
			vs = append(vs, fmt.Sprintf(".f%d = %s", i, v))
		}
		return val(fmt.Sprintf("((%s){ %s })", fc.ctype(t), strings.Join(vs, ", ")))
	case *ast.ArrayExpr:
		vs := fc.evalOperands(x.Elems)
		return val(fmt.Sprintf("((%s){ { %s } })", fc.ctype(t), strings.Join(vs, ", ")))
	case *ast.RepeatExpr:
		v := fc.spill(fc.value(x.Elem), fc.aty(x.Elem))
		n := fc.newName("t")
		fc.line("%s %s;", fc.ctype(t), n)
		cnt := fc.g.arrayLen(t)
		if cnt > 0 {
			if fc.g.tcx.IsCopy(t.Elem) {
				fc.line("for (uint64_t i = 0; i < %d; i++) %s.a[i] = %s;", cnt, n, v)
			} else {
				// non-Copy: only valid for consts; clone bitwise
				fc.line("for (uint64_t i = 0; i < %d; i++) %s.a[i] = %s;", cnt, n, v)
			}
		}
		return val(n)
	case *ast.StructExpr:
		return val(fc.structLit(x, t))
	case *ast.RangeExpr:
		return val(fc.rangeLit(x, t))
	case *ast.TryExpr:
		return val(fc.tryExpr(x, t))
	case *ast.ClosureExpr:
		return val(fc.closureLit(x, t))
	case *ast.LetExpr:
		fc.g.fatalf("%s: `let` expression in unsupported position", x.Span.Lo)
	}
	fc.g.fatalf("%s: codegen: unsupported expression %T", e.GetSpan().Lo, e)
	return operand{}
}

func (fc *fnGen) spillIfNeeded(v string, t Ty) string {
	if isSimple(v) || strings.HasPrefix(v, "((") {
		return v
	}
	return fc.spill(v, t)
}

// ---------------------------------------------------------------------------
// Literals

func (fc *fnGen) literal(x *ast.LitExpr, t Ty) string {
	switch x.Kind {
	case ast.LitInt, ast.LitFloat:
		if t.Kind == TFloat {
			var f float64
			if x.Kind == ast.LitInt {
				f = float64(x.IntVal)
			} else {
				s := strings.ReplaceAll(x.Value, "_", "")
				var err error
				f, err = strconv.ParseFloat(s, 64)
				if err != nil {
					fc.g.fatalf("%s: invalid float literal", x.Span.Lo)
				}
			}
			if t.Sub == 32 {
				return fmt.Sprintf("((float)%s)", cFloat(float64(float32(f))))
			}
			return cFloat(f)
		}
		return intLit(x, t, fc.ctype(t))
	case ast.LitBool:
		if x.Value == "true" {
			return "1"
		}
		return "0"
	case ast.LitChar:
		return fmt.Sprintf("((uint32_t)%d)", x.IntVal)
	case ast.LitByte:
		return fmt.Sprintf("((uint8_t)%d)", x.IntVal)
	case ast.LitStr:
		return fmt.Sprintf("((Fat){ (void*)%s, %d })", cString(x.Value), len(x.Value))
	case ast.LitByteStr:
		// &[u8; N]: pointer to a static array
		name := fc.newName("bs")
		at := t.Elem
		fc.g.defineByValue(at)
		if len(x.Value) >= 64 {
			// a string literal (of exactly N chars, no NUL) parses much
			// faster than N integer initializers (include_bytes! data)
			fmt.Fprintf(&fc.g.statics, "static %s %s = { { %s } };\n", fc.g.ctype(at), name, cString(x.Value))
			return "(&" + name + ")"
		}
		var vs []string
		for i := 0; i < len(x.Value); i++ {
			vs = append(vs, strconv.Itoa(int(x.Value[i])))
		}
		fmt.Fprintf(&fc.g.statics, "static %s %s = { { %s } };\n", fc.g.ctype(at), name, strings.Join(vs, ", "))
		return "(&" + name + ")"
	}
	fc.g.fatalf("unsupported literal")
	return ""
}

func cFloat(f float64) string {
	switch {
	case math.IsInf(f, 1):
		return "(1.0/0.0)"
	case math.IsInf(f, -1):
		return "(-1.0/0.0)"
	case math.IsNaN(f):
		return "(0.0/0.0)"
	}
	return fmt.Sprintf("(%s)", strconv.FormatFloat(f, 'x', -1, 64))
}

// u128Const renders a decimal 128-bit literal as a C constant expression.
func u128Const(dec string) string {
	b, ok := new(big.Int).SetString(dec, 10)
	if !ok {
		return fmt.Sprintf("rg_u128_from_str(%q)", dec)
	}
	mask := new(big.Int).SetUint64(^uint64(0))
	lo := new(big.Int).And(b, mask).Uint64()
	hi := new(big.Int).Rsh(b, 64).Uint64()
	return fmt.Sprintf("(((unsigned __int128)%dULL << 64) | (unsigned __int128)%dULL)", hi, lo)
}

func intLit(x *ast.LitExpr, t Ty, ct string) string {
	if x.Big != "" {
		// 128-bit literal from decimal text
		return fmt.Sprintf("((%s)%s)", ct, u128Const(x.Big))
	}
	v := x.IntVal
	switch t.IntKind() {
	case I128, U128:
		return fmt.Sprintf("((%s)%dULL)", ct, v)
	}
	if t.IsSigned() {
		return fmt.Sprintf("((%s)%dLL)", ct, int64(v))
	}
	return fmt.Sprintf("((%s)%dULL)", ct, v)
}

// ---------------------------------------------------------------------------
// Paths

func (fc *fnGen) pathOperand(x *ast.PathExpr, t Ty) operand {
	vr := fc.r.Paths[x.ID]
	if vr == nil {
		fc.g.fatalf("%s: unresolved path", x.Span.Lo)
	}
	switch vr.Kind {
	case typeck.VLocal:
		p, flag := fc.localPlace(vr.Local)
		return operand{s: p, place: true, flag: flag, movable: true, ty: t}
	case typeck.VFn:
		return operand{s: fc.unit(), ty: t}
	case typeck.VConst:
		d, args := vr.Def, vr.Args
		if d.InTrait() {
			d, args = fc.resolveAssocConst(d, args)
		}
		if lit, ok := fc.g.scalarConstLit(d, args, t); ok {
			return operand{s: lit, ty: t}
		}
		if fc.g.constCached(d, t) && !fc.inConst {
			// a place in the const's static cache: `&CONST[..]` and
			// `&CONST.field` borrow 'static data (rvalue promotion)
			return operand{s: "(*" + fc.g.fnName(d, args) + "_p())", place: true, ty: t}
		}
		return operand{s: fc.g.fnName(d, args) + "()", ty: t}
	case typeck.VStatic:
		return operand{s: fc.g.staticVar(vr.Def), place: true, ty: t}
	case typeck.VUnitCtor:
		return operand{s: fc.g.adtLiteral(t, vr.Def, nil), ty: t}
	case typeck.VConstParam:
		return operand{s: fmt.Sprintf("((%s)%dULL)", fc.ctype(t), vr.Value), ty: t}
	}
	fc.g.fatalf("bad path")
	return operand{}
}

// resolveAssocConst maps a trait assoc const to the impl's const (or the
// trait default).
func (fc *fnGen) resolveAssocConst(d *Def, args []Ty) (*Def, []Ty) {
	trait := d.Parent
	tr := &TraitRef{Trait: trait, Args: args}
	impl, iargs := fc.g.tcx.SelectImplSimple(tr)
	if impl != nil {
		if c, ok := impl.Impl.Items[d.Name]; ok {
			return c, iargs
		}
	}
	return d, args
}

// ---------------------------------------------------------------------------
// Unary / binary operators

func (fc *fnGen) unaryOperand(x *ast.UnaryExpr, t Ty) operand {
	if c, ok := fc.r.Callees[x.ID]; ok {
		if x.Op == ast.UnDeref {
			inner := fc.adjusted(x.X)
			recv := fc.addrOf(inner)
			mut := c.Def.Name == "deref_mut"
			rt := Ref(t, mut)
			v := fc.spill(fc.callResolved(c.Def, c.Args, []string{recv}, rt), rt)
			if t.IsUnsized() {
				return operand{s: v, unsized: true, ty: t}
			}
			return operand{s: "(*" + v + ")", place: true, ty: t}
		}
		v := fc.value(x.X)
		return operand{s: fc.spill(fc.callResolved(c.Def, c.Args, []string{v}, t), t), ty: t}
	}
	switch x.Op {
	case ast.UnDeref:
		it := fc.aty(x.X)
		switch it.Kind {
		case TRef, TPtr:
			v := fc.value(x.X)
			if it.Elem.IsUnsized() {
				return operand{s: v, unsized: true, ty: t}
			}
			return operand{s: "(*" + v + ")", place: true, ty: t}
		case TAdt: // Box
			inner := fc.asPlace(fc.adjusted(x.X))
			if it.Args[0].IsUnsized() {
				return operand{s: inner.s + ".f0", unsized: true, ty: t}
			}
			return operand{s: "(*" + inner.s + ".f0)", place: true, flag: inner.flag, movable: inner.movable, ty: t}
		}
	case ast.UnNeg:
		if lit, ok := unparen(x.X).(*ast.LitExpr); ok && lit.Kind == ast.LitInt && t.Kind == TInt && len(fc.r.Adjust[x.X.GetID()]) == 0 {
			if lit.Big != "" {
				return operand{s: fmt.Sprintf("((%s)(0 - %s))", fc.ctype(t), u128Const(lit.Big)), ty: t}
			}
			return operand{s: fmt.Sprintf("((%s)(0 - (unsigned __int128)%dULL))", fc.ctype(t), lit.IntVal), ty: t}
		}
		v := fc.value(x.X)
		if t.Kind == TInt && t.IsSigned() {
			return operand{s: fmt.Sprintf("RG_NEG(%s, %s, %s)", fc.ctype(t), v, fc.loc(x.Span)), ty: t}
		}
		return operand{s: "(-" + v + ")", ty: t}
	case ast.UnNot:
		v := fc.value(x.X)
		if t.Kind == TBool {
			return operand{s: "(!" + v + ")", ty: t}
		}
		return operand{s: fmt.Sprintf("((%s)~%s)", fc.ctype(t), v), ty: t}
	}
	fc.g.fatalf("bad unary")
	return operand{}
}

func (fc *fnGen) loc(sp ast.Span) string {
	if fc.trackCaller {
		// implicit panics in a #[track_caller] fn report its caller
		f := cString(fileNameOf(sp))
		return fmt.Sprintf("(__loc ? (const char*)__loc->file.ptr : %s), (__loc ? __loc->line : %d), (__loc ? __loc->col : %d)", f, sp.Lo.Line, sp.Lo.Col)
	}
	return fmt.Sprintf("%s, %d, %d", cString(fileNameOf(sp)), sp.Lo.Line, sp.Lo.Col)
}

var arithMacro = map[ast.BinOp]string{
	ast.BinAdd: "RG_ADD", ast.BinSub: "RG_SUB", ast.BinMul: "RG_MUL", ast.BinDiv: "RG_DIV", ast.BinRem: "RG_REM",
	ast.BinShl: "RG_SHL", ast.BinShr: "RG_SHR",
}

var cOp = map[ast.BinOp]string{
	ast.BinBitAnd: "&", ast.BinBitOr: "|", ast.BinBitXor: "^",
	ast.BinEq: "==", ast.BinNe: "!=", ast.BinLt: "<", ast.BinLe: "<=", ast.BinGt: ">", ast.BinGe: ">=",
	ast.BinAdd: "+", ast.BinSub: "-", ast.BinMul: "*", ast.BinDiv: "/",
}

func (fc *fnGen) binary(x *ast.BinaryExpr, t Ty) string {
	if c, ok := fc.r.Callees[x.ID]; ok {
		if x.Op >= ast.BinEq {
			l := fc.addrOf(fc.adjusted(x.L))
			r := fc.addrOf(fc.adjusted(x.R))
			v := fc.callResolvedSpan(c.Def, c.Args, []string{l, r}, TyBool, x.Span)
			return fc.spill(v, TyBool)
		}
		l := fc.spillIfNeeded(fc.value(x.L), fc.aty(x.L))
		r := fc.value(x.R)
		return fc.spill(fc.callResolvedSpan(c.Def, c.Args, []string{l, r}, t, x.Span), t)
	}
	switch x.Op {
	case ast.BinAnd, ast.BinOr:
		l := fc.value(x.L)
		res := fc.newName("t")
		fc.line("uint8_t %s = %s;", res, l)
		if x.Op == ast.BinAnd {
			fc.line("if (%s) {", res)
		} else {
			fc.line("if (!%s) {", res)
		}
		fc.ind++
		fc.pushScope()
		r := fc.value(x.R)
		fc.line("%s = %s;", res, r)
		fc.popScope()
		fc.ind--
		fc.line("}")
		return res
	}
	lt := fc.aty(x.L)
	l := fc.value(x.L)
	if !isSimple(l) {
		l = fc.spill(l, lt)
	}
	r := fc.value(x.R)
	if x.Op >= ast.BinEq {
		if lt.Kind == TFnPtr || lt.Kind == TPtr {
			if lt.Kind == TPtr && lt.Elem.IsUnsized() {
				return fmt.Sprintf("((%s).ptr %s (%s).ptr)", l, cOp[x.Op], r)
			}
		}
		return fmt.Sprintf("(%s %s %s)", l, cOp[x.Op], r)
	}
	ct := fc.ctype(t)
	if t.Kind == TFloat {
		if x.Op == ast.BinRem {
			if t.Sub == 32 {
				return fmt.Sprintf("fmodf(%s, %s)", l, r)
			}
			return fmt.Sprintf("fmod(%s, %s)", l, r)
		}
		return fmt.Sprintf("((%s)(%s %s %s))", ct, l, cOp[x.Op], r)
	}
	if t.Kind == TBool {
		return fmt.Sprintf("((uint8_t)(%s %s %s))", l, cOp[x.Op], r)
	}
	if m, ok := arithMacro[x.Op]; ok {
		if x.Op == ast.BinShl || x.Op == ast.BinShr {
			return fmt.Sprintf("%s(%s, %d, %s, %s, %s)", m, ct, t.IntKind().Bits(), l, r, fc.loc(x.Span))
		}
		return fmt.Sprintf("%s(%s, %s, %s, %s)", m, ct, l, r, fc.loc(x.Span))
	}
	return fmt.Sprintf("((%s)(%s %s %s))", ct, l, cOp[x.Op], r)
}

func (fc *fnGen) assign(x *ast.AssignExpr) {
	switch l := x.L.(type) {
	case *ast.UnderscoreExpr:
		v := fc.value(x.R)
		t := fc.aty(x.R)
		if fc.g.needsDrop(t) {
			fc.pushScope()
			fc.materialize(v, t)
			fc.popScope()
		} else if v != fc.unit() && !isSimple(v) {
			fc.line("(void)%s;", v)
		}
		return
	case *ast.TupleExpr:
		v := fc.spill(fc.value(x.R), fc.aty(x.R))
		fc.destructure(l, v)
		return
	}
	v := fc.value(x.R)
	fc.storeTo(x.L, v)
}

func (fc *fnGen) destructure(l ast.Expr, v string) {
	switch x := l.(type) {
	case *ast.UnderscoreExpr:
	case *ast.ParenExpr:
		fc.destructure(x.X, v)
	case *ast.TupleExpr:
		for i, el := range x.Elems {
			fc.destructure(el, fmt.Sprintf("%s.f%d", v, i))
		}
	default:
		fc.storeTo(l, v)
	}
}

// storeTo assigns value v to the place l, dropping the old value.
func (fc *fnGen) storeTo(l ast.Expr, v string) {
	t := fc.aty(l)
	if isZST(t) {
		if v != fc.unit() && !isSimple(v) {
			fc.line("(void)%s;", v)
		}
		// still evaluate place for side effects
		return
	}
	// whole-local assignment
	if pe, ok := unparen(l).(*ast.PathExpr); ok && len(fc.r.Adjust[l.GetID()]) == 0 {
		if vr := fc.r.Paths[pe.ID]; vr != nil && vr.Kind == typeck.VLocal {
			p, flag := fc.localPlace(vr.Local)
			if flag != "" {
				tmp := fc.spill(v, t)
				if _, captured := fc.capIndex[vr.Local]; !captured {
					fc.line("%s", fc.dropCode(dropEntry{place: p, flag: fc.flagName(vr.Local), ty: t, local: vr.Local}))
				}
				fc.line("%s = %s; %s = 1;", p, tmp, flag)
				return
			}
			fc.line("%s = %s;", p, v)
			return
		}
	}
	if fc.g.needsDrop(t) {
		tmp := fc.spill(v, t)
		op := fc.placeOp(l)
		if op.tlocal != nil && fc.trackedBelow(op.tlocal, op.tpath) {
			// re-initializing a field of a partially moved local whose
			// parts have their own flags
			var parts []string
			f := fc.pathFlag(op.tlocal, op.tpath)
			fc.dropParts(&parts, op.tlocal, op.s, t, op.tpath, f)
			fc.line("%s", strings.Join(parts, " "))
			fc.line("%s = %s; %s = 1;", op.s, tmp, op.flag)
			return
		}
		if strings.Contains(op.flag, "_") && !strings.Contains(op.flag, "=") {
			// re-initializing a field of a partially moved local
			fc.line("if (%s) %s(&%s);", op.flag, fc.g.dropFn(t), op.s)
			fc.line("%s = %s; %s = 1;", op.s, tmp, op.flag)
			return
		}
		ptr := fc.spill("&"+op.s, Ptr(t, true))
		fc.line("%s(%s);", fc.g.dropFn(t), ptr)
		fc.line("*%s = %s;", ptr, tmp)
		return
	}
	if !isSimple(v) {
		v = fc.spill(v, t)
	}
	op := fc.placeOp(l)
	fc.line("%s = %s;", op.s, v)
}

func unparen(e ast.Expr) ast.Expr {
	for {
		p, ok := e.(*ast.ParenExpr)
		if !ok {
			return e
		}
		e = p.X
	}
}

func (fc *fnGen) assignOp(x *ast.AssignOpExpr) {
	if c, ok := fc.r.Callees[x.ID]; ok {
		r := fc.value(x.R)
		r = fc.spillIfNeeded(r, fc.aty(x.R))
		l := fc.addrOf(fc.adjusted(x.L))
		fc.line("%s;", fc.callResolvedSpan(c.Def, c.Args, []string{l, r}, TyUnit, x.Span))
		return
	}
	t := fc.aty(x.L)
	r := fc.value(x.R)
	if !isSimple(r) {
		r = fc.spill(r, fc.aty(x.R))
	}
	op := fc.placeOp(x.L)
	ptr := op.s
	if !isSimple(op.s) {
		ptr = "(*" + fc.spill("&"+op.s, Ptr(t, true)) + ")"
	}
	ct := fc.ctype(t)
	if t.Kind == TFloat {
		if x.Op == ast.BinRem {
			fn := "fmod"
			if t.Sub == 32 {
				fn = "fmodf"
			}
			fc.line("%s = %s(%s, %s);", ptr, fn, ptr, r)
			return
		}
		fc.line("%s = %s %s %s;", ptr, ptr, cOp[x.Op], r)
		return
	}
	if t.Kind == TBool {
		fc.line("%s = (uint8_t)(%s %s %s);", ptr, ptr, cOp[x.Op], r)
		return
	}
	if m, ok := arithMacro[x.Op]; ok {
		if x.Op == ast.BinShl || x.Op == ast.BinShr {
			fc.line("%s = %s(%s, %d, %s, %s, %s);", ptr, m, ct, t.IntKind().Bits(), ptr, r, fc.loc(x.Span))
		} else {
			fc.line("%s = %s(%s, %s, %s, %s);", ptr, m, ct, ptr, r, fc.loc(x.Span))
		}
		return
	}
	fc.line("%s = (%s)(%s %s %s);", ptr, ct, ptr, cOp[x.Op], r)
}

// ---------------------------------------------------------------------------
// Indexing

func (fc *fnGen) indexOperand(x *ast.IndexExpr, t Ty) operand {
	if fc.r.IndexBuiltin[x.ID] {
		base := fc.asPlace(fc.adjusted(x.X))
		bt := fc.aty(x.X)
		idx := fc.spill(fc.value(x.Index), TyUsize)
		switch bt.Kind {
		case TArray:
			n := fc.g.arrayLen(bt)
			fc.line("if (%s >= %d) rg_panic_bounds(%s, %d, %s);", idx, n, idx, n, fc.loc(x.Span))
			return operand{s: fmt.Sprintf("%s.a[%s]", base.s, idx), place: true, flag: base.flag, movable: false, ty: t}
		case TSlice:
			fs := base.s
			if !isSimple(fs) {
				fs = fc.spill(fs, Ref(bt, false))
			}
			fc.line("if (%s >= %s.meta) rg_panic_bounds(%s, %s.meta, %s);", idx, fs, idx, fs, fc.loc(x.Span))
			return operand{s: fmt.Sprintf("((%s*)%s.ptr)[%s]", fc.ctype(t), fs, idx), place: true, ty: t}
		}
		fc.g.fatalf("bad builtin index on %s", bt)
	}
	c := fc.r.Callees[x.ID]
	base := fc.adjusted(x.X)
	recv := fc.addrOf(base)
	if !isSimple(recv) {
		recv = fc.spill(recv, Ref(fc.aty(x.X), true))
	}
	idx := fc.value(x.Index)
	mut := c.Def.Name == "index_mut"
	rt := Ref(t, mut)
	v := fc.spill(fc.callResolvedSpan(c.Def, c.Args, []string{recv, idx}, rt, bracketSpan(x)), rt)
	if t.IsUnsized() {
		return operand{s: v, unsized: true, ty: t}
	}
	return operand{s: "(*" + v + ")", place: true, ty: t}
}

// ---------------------------------------------------------------------------
// Casts

func (fc *fnGen) cast(x *ast.CastExpr, t Ty) string {
	from := fc.aty(x.X)
	v := fc.value(x.X)
	ct := fc.ctype(t)
	if from == t {
		return v
	}
	switch {
	case from.Kind == TFloat && t.Kind == TInt:
		return fmt.Sprintf("RG_F2I(%s, %s, %s, %s)", ct, v, intMinStr(t), intMaxStr(t))
	case from.Kind == TAdt && from.Def.Adt.Kind == AdtEnum:
		return fmt.Sprintf("((%s)%s)", ct, fc.g.enumTag(from, fc.spillIfNeeded(v, from)))
	case (from.Kind == TRef || from.Kind == TPtr) && (t.Kind == TPtr || t.Kind == TRef):
		if from.Elem.IsUnsized() && !t.Elem.IsUnsized() {
			return fmt.Sprintf("((%s)(%s).ptr)", ct, v)
		}
		if from.Elem.IsUnsized() && t.Elem.IsUnsized() {
			return v
		}
		return fmt.Sprintf("((%s)(%s))", ct, v)
	case (from.Kind == TRef || from.Kind == TPtr) && t.Kind == TInt:
		if from.Elem.IsUnsized() {
			return fmt.Sprintf("((%s)(uintptr_t)(%s).ptr)", ct, v)
		}
		return fmt.Sprintf("((%s)(uintptr_t)(%s))", ct, v)
	case from.Kind == TInt && t.Kind == TPtr:
		return fmt.Sprintf("((%s)(uintptr_t)(%s))", ct, v)
	case from.Kind == TFnDef && (t.Kind == TFnPtr || t.Kind == TInt || t.Kind == TPtr):
		return fmt.Sprintf("((%s)%s)", ct, fc.g.fnValue(from))
	case from.Kind == TFnPtr && t.Kind == TInt:
		return fmt.Sprintf("((%s)(uintptr_t)(%s))", ct, v)
	case t.Kind == TChar && from.Kind == TInt:
		return fmt.Sprintf("((uint32_t)(uint8_t)(%s))", v)
	case t.Kind == TBool:
		return fmt.Sprintf("((uint8_t)(%s))", v)
	}
	return fmt.Sprintf("((%s)(%s))", ct, v)
}

func intMinStr(t Ty) string {
	it := t.IntKind()
	switch it {
	case I128:
		return "RG_I128_MIN"
	case U128, U8, U16, U32, U64, Usize:
		return "0"
	}
	return fmt.Sprintf("(%dLL)", int64(-1)<<(it.Bits()-1))
}

func intMaxStr(t Ty) string {
	it := t.IntKind()
	switch it {
	case I128:
		return "RG_I128_MAX"
	case U128:
		return "RG_U128_MAX"
	case U64, Usize:
		return "UINT64_MAX"
	case I64, Isize:
		return "INT64_MAX"
	}
	if it.Signed() {
		return fmt.Sprintf("(%dLL)", int64(1)<<(it.Bits()-1)-1)
	}
	return fmt.Sprintf("(%dULL)", uint64(1)<<it.Bits()-1)
}

// ---------------------------------------------------------------------------
// Aggregates

func (fc *fnGen) structLit(x *ast.StructExpr, t Ty) string {
	v := fc.r.Variants[x.ID]
	vals := make([]string, len(v.Fields))
	fes := make([]ast.Expr, len(x.Fields))
	for k, f := range x.Fields {
		fes[k] = f.Expr
	}
	for k, fv := range fc.evalOperands(fes) {
		vals[v.FieldIndex(x.Fields[k].Name.Name)] = fv
	}
	if l := fc.structBaseLocal(x); l != nil && len(fc.split[l]) > 0 {
		// `..l` moves only the fields not given; the rest stay in l
		place, _ := fc.localPlace(l)
		fts := fc.g.fieldTypes(t, 0)
		for i := range vals {
			if vals[i] == "" {
				vals[i] = fmt.Sprintf("%s.f%d", place, i)
				if fc.g.needsDrop(fts[i]) {
					fc.line("%s = 0;", fc.pathFlags(l, []int{i}))
				}
			}
		}
	} else if x.Base != nil {
		base := fc.spill(fc.value(x.Base), t)
		// fields not given come from base; the remaining base fields are
		// moved into the new value
		for i := range vals {
			if vals[i] == "" {
				vals[i] = fmt.Sprintf("%s.f%d", base, i)
			}
		}
		// drop base fields that were overridden
		fts := fc.g.fieldTypes(t, 0)
		for _, f := range x.Fields {
			i := v.FieldIndex(f.Name.Name)
			if fc.g.needsDrop(fts[i]) {
				fc.line("%s(&%s.f%d);", fc.g.dropFn(fts[i]), base, i)
			}
		}
	}
	if t.Def.Adt.Kind == AdtUnion {
		for i, val := range vals {
			if val != "" {
				return fmt.Sprintf("((%s){ .f%d = %s })", fc.ctype(t), i, val)
			}
		}
	}
	return fc.g.adtLiteral(t, v.Def, vals)
}

func (fc *fnGen) rangeLit(x *ast.RangeExpr, t Ty) string {
	var vals []string
	if x.Start != nil {
		vals = append(vals, fc.spillIfNeeded(fc.value(x.Start), fc.aty(x.Start)))
	}
	if x.End != nil {
		vals = append(vals, fc.spillIfNeeded(fc.value(x.End), fc.aty(x.End)))
	}
	if t.Def.Name == "RangeInclusive" {
		vals = append(vals, "0")
	}
	return fc.g.adtLiteral(t, t.Def, vals)
}

func (fc *fnGen) closureLit(x *ast.ClosureExpr, t Ty) string {
	ci := fc.g.tcx.Closures[t]
	fc.g.closureFn(t)
	var fs []string
	for i, c := range ci.Captures {
		p, flag := fc.capturePlace(c)
		if c.ByRef {
			fs = append(fs, fmt.Sprintf(".c%d = &%s", i, p))
		} else {
			if flag != "" && !fc.g.tcx.IsCopy(c.Ty) {
				v := fc.spill(p, c.Ty)
				fc.line("%s = 0;", flag)
				p = v
			}
			fs = append(fs, fmt.Sprintf(".c%d = %s", i, p))
		}
	}
	fc.g.defineByValue(t)
	return fmt.Sprintf("((%s){ %s })", fc.ctype(t), strings.Join(fs, ", "))
}

// ---------------------------------------------------------------------------
// `?`

func (fc *fnGen) tryExpr(x *ast.TryExpr, t Ty) string {
	info := fc.r.Tries[x.ID]
	st := fc.aty(x.X)
	v := fc.spill(fc.value(x.X), st)
	tcx := fc.g.tcx
	if info.IsOption {
		some := tcx.Lang("Some").Variant
		fc.line("if (!%s) {", fc.g.enumIs(st, v, some))
		fc.ind++
		none := tcx.Lang("None")
		fc.emitReturn(fc.g.adtLiteral(info.OutTy, none, nil))
		fc.ind--
		fc.line("}")
		if isZST(t) {
			return fc.unit()
		}
		return fmt.Sprintf("%s.f0", fc.g.enumPayload(st, v, some.Index))
	}
	ok := tcx.Lang("Ok").Variant
	errD := tcx.Lang("Err")
	g := fc.g
	// res: the Result inside the operand; opt: the Option around it
	res, resTy := v, st
	var out string // the value of the `?` expression
	if info.Src == typeck.TryPoll || info.Src == typeck.TryPollOpt {
		ready := tcx.Lang("Ready")
		out = fc.newName("t")
		fc.line("%s %s;", fc.ctype(t), out)
		fc.line("if (%s) {", g.enumIs(st, v, ready.Variant))
		fc.ind++
		inner := st.Args[0]
		iv := fc.spill(fmt.Sprintf("%s.f0", g.enumPayload(st, v, ready.Variant.Index)), inner)
		res, resTy = iv, inner
		if info.Src == typeck.TryPollOpt {
			some := tcx.Lang("Some")
			fc.line("if (%s) {", g.enumIs(inner, iv, some.Variant))
			fc.ind++
			resTy = inner.Args[0]
			res = fc.spill(fmt.Sprintf("%s.f0", g.enumPayload(inner, iv, some.Variant.Index)), resTy)
		}
	}
	fc.line("if (!%s) {", g.enumIs(resTy, res, ok))
	fc.ind++
	e := fmt.Sprintf("%s.f0", g.enumPayload(resTy, res, errD.Variant.Index))
	if info.From != nil {
		e = fc.spill(fc.callResolved(info.From.Def, info.From.Args, []string{e}, info.OutErrTy), info.OutErrTy)
	}
	fc.emitReturn(fc.tryResidual(info, e))
	fc.ind--
	fc.line("}")
	okv := fc.unit()
	okTy := resTy.Args[0]
	if !isZST(okTy) {
		okv = fmt.Sprintf("%s.f0", g.enumPayload(resTy, res, ok.Index))
	}
	if out == "" {
		if isZST(t) {
			return fc.unit()
		}
		return okv
	}
	ready := tcx.Lang("Ready")
	pending := tcx.Lang("Pending")
	if info.Src == typeck.TryPollOpt {
		some := tcx.Lang("Some")
		ot := t.Args[0]
		fc.line("%s = %s;", out, g.adtLiteral(t, ready, []string{g.adtLiteral(ot, some, []string{okv})}))
		fc.ind--
		fc.line("} else {")
		fc.ind++
		fc.line("%s = %s;", out, g.adtLiteral(t, ready, []string{g.adtLiteral(ot, tcx.Lang("None"), nil)}))
		fc.ind--
		fc.line("}")
	} else {
		fc.line("%s = %s;", out, g.adtLiteral(t, ready, []string{okv}))
	}
	fc.ind--
	fc.line("} else {")
	fc.line("  %s = %s;", out, g.adtLiteral(t, pending, nil))
	fc.line("}")
	return out
}

// tryResidual builds the return value of a failed `?` from the (converted)
// error e: Err(e), Poll::Ready(Err(e)) or Poll::Ready(Some(Err(e))).
func (fc *fnGen) tryResidual(info *typeck.TryInfo, e string) string {
	tcx := fc.g.tcx
	errD := tcx.Lang("Err")
	switch info.Out {
	case typeck.TryPoll:
		rt := info.OutTy.Args[0]
		return fc.g.adtLiteral(info.OutTy, tcx.Lang("Ready"), []string{fc.g.adtLiteral(rt, errD, []string{e})})
	case typeck.TryPollOpt:
		ot := info.OutTy.Args[0]
		rt := ot.Args[0]
		return fc.g.adtLiteral(info.OutTy, tcx.Lang("Ready"), []string{fc.g.adtLiteral(ot, tcx.Lang("Some"), []string{fc.g.adtLiteral(rt, errD, []string{e})})})
	}
	return fc.g.adtLiteral(info.OutTy, errD, []string{e})
}

// ---------------------------------------------------------------------------
// Calls

// argValues lowers call arguments left to right.
func (fc *fnGen) argValues(args []ast.Expr) []string {
	return fc.evalOperands(args)
}

// evalOperands evaluates operands left to right (call arguments, aggregate
// fields). An owned operand that is followed by an operand that may panic is
// kept in a drop-flagged temporary on the cleanup stack, so unwinding out of
// a later operand drops it; once all operands are evaluated, ownership passes
// to the aggregate or callee and the flags are cleared.
func (fc *fnGen) evalOperands(es []ast.Expr) []string {
	out := make([]string, len(es))
	var flags []string
	for i, e := range es {
		v := fc.value(e)
		t := fc.aty(e)
		if t != nil && !isZST(t) && fc.g.needsDrop(t) && anyMayPanic(es[i+1:]) {
			n, fl := fc.materialize(v, t)
			if fl != "" {
				flags = append(flags, fl)
			}
			v = n
		} else if !isSimple(v) && !strings.HasPrefix(v, "((") {
			v = fc.spill(v, t)
		}
		out[i] = v
	}
	for _, fl := range flags {
		fc.line("%s = 0;", fl)
	}
	return out
}

// anyMayPanic reports whether evaluating any of es might panic (conservative:
// only literals, paths, closures and borrows of paths are known not to).
func anyMayPanic(es []ast.Expr) bool {
	for _, e := range es {
		if mayPanic(e) {
			return true
		}
	}
	return false
}

func mayPanic(e ast.Expr) bool {
	switch x := e.(type) {
	case *ast.LitExpr, *ast.PathExpr, *ast.ClosureExpr:
		return false
	case *ast.ParenExpr:
		return mayPanic(x.X)
	case *ast.RefExpr:
		return mayPanic(x.X)
	case *ast.FieldExpr:
		// plain field projections of paths (autoderef through Deref impls
		// would be a call, but those don't panic in practice)
		return mayPanic(x.X)
	}
	return true
}

func (fc *fnGen) callExpr(x *ast.CallExpr, t Ty) string {
	c := fc.r.Callees[x.ID]
	if c == nil {
		fc.g.fatalf("%s: unresolved call", x.Span.Lo)
	}
	switch c.Kind {
	case typeck.CallFn, typeck.CallCtor, typeck.CallVirtual:
		args := fc.argValues(x.Args)
		return fc.finishCall(fc.callResolvedSpan(c.Def, c.Args, args, t, x.Span), t)
	case typeck.CallFnPtr:
		f := fc.spillIfNeeded(fc.value(x.Fn), fc.aty(x.Fn))
		args := fc.argValues(x.Args)
		return fc.finishCall(fmt.Sprintf("%s(%s)", f, strings.Join(args, ", ")), t)
	case typeck.CallClosure:
		op := fc.asPlace(fc.adjusted(x.Fn))
		args := fc.argValues(x.Args)
		ci := fc.g.tcx.Closures[c.Ty]
		if ci == nil {
			fc.g.fatalf("%s: internal error: call of `%s` resolved to non-closure type", x.Span.Lo, c.Ty)
		}
		name := fc.g.closureFn(c.Ty)
		all := append([]string{"&" + op.s}, args...)
		res := fc.finishCall(fmt.Sprintf("%s(%s)", name, strings.Join(all, ", ")), t)
		if ci.Kind == 2 && op.flag != "" && op.movable {
			fc.line("%s = 0;", op.flag)
		}
		return res
	case typeck.CallDynFn:
		op := fc.adjusted(x.Fn)
		fat := op.s
		if !isSimple(fat) {
			fat = fc.spill(fat, Ref(c.Ty, false))
		}
		args := fc.argValues(x.Args)
		var pts []string
		pts = append(pts, "void*")
		for _, a := range x.Args {
			pts = append(pts, fc.ctype(fc.aty(a)))
		}
		fp := fmt.Sprintf("((%s(*)(%s))((void**)(%s).meta)[3])", fc.ctype(t), strings.Join(pts, ", "), fat)
		all := append([]string{fat + ".ptr"}, args...)
		res := fc.finishCall(fmt.Sprintf("%s(%s)", fp, strings.Join(all, ", ")), t)
		if c.Ty.Def.LangItem == "fn_once" {
			// the boxed FnOnce consumed its environment; free the box
			// without dropping its contents
			if pe, ok := unparen(x.Fn).(*ast.PathExpr); ok {
				if vr := fc.r.Paths[pe.ID]; vr != nil && vr.Kind == typeck.VLocal {
					if _, flag := fc.localPlace(vr.Local); flag != "" {
						fc.line("%s = 0;", flag)
						// the environment was consumed by the call; free
						// the box's allocation (size/align from the vtable)
						if fc.isBoxTy(fc.ty(pe)) {
							fc.line("{ uint64_t _bsz = (uint64_t)((void**)(%s).meta)[1]; if (_bsz) rg_dealloc((uint8_t*)(%s).ptr, _bsz, (uint64_t)((void**)(%s).meta)[2]); }", fat, fat, fat)
						}
					}
				}
			}
		}
		return res
	}
	fc.g.fatalf("unsupported call kind")
	return ""
}

// finishCall spills the call result (or emits it as a statement for ZSTs).
func (fc *fnGen) finishCall(call string, t Ty) string {
	if isZST(t) {
		fc.line("%s;", call)
		if t.Kind == TNever {
			fc.line("rg_unreachable();")
		}
		return fc.unit()
	}
	return fc.spill(call, t)
}

func (fc *fnGen) methodCall(x *ast.MethodCallExpr, t Ty) string {
	c := fc.r.Callees[x.ID]
	ops := fc.evalOperands(append([]ast.Expr{x.Receiver}, x.Args...))
	recv, args := ops[0], ops[1:]
	if c.Kind == typeck.CallVirtual {
		dyn := c.Args[0]
		idx := fc.g.vtableIndex(dyn, c.Def)
		var pts []string
		pts = append(pts, "void*")
		for _, a := range x.Args {
			pts = append(pts, fc.ctype(fc.aty(a)))
		}
		fat := fc.dynFat(recv, fc.aty(x.Receiver))
		fp := fmt.Sprintf("((%s(*)(%s))((void**)(%s).meta)[%d])", fc.ctype(t), strings.Join(pts, ", "), fat, idx)
		all := append([]string{fat + ".ptr"}, args...)
		return fc.finishCall(fmt.Sprintf("%s(%s)", fp, strings.Join(all, ", ")), t)
	}
	all := append([]string{recv}, args...)
	return fc.finishCall(fc.callResolvedSpan(c.Def, c.Args, all, t, x.Method.Span), t)
}

// callResolved builds a call expression to a (possibly trait) function.
func (fc *fnGen) callResolved(d *Def, args []Ty, vals []string, ret Ty) string {
	return fc.callResolvedSpan(d, args, vals, ret, d.Span)
}

func (fc *fnGen) callResolvedSpan(d *Def, args []Ty, vals []string, ret Ty, sp ast.Span) string {
	if d.InTrait() {
		if len(args) > 0 && args[0].Kind == TDyn && fc.g.tcx.IsObjectMethod(d, args[0]) {
			// virtual call through a generic path
			idx := fc.g.vtableIndex(args[0], d)
			recv := vals[0]
			if !isSimple(recv) {
				recv = fc.spill(recv, Ref(args[0], false))
			}
			var pts []string
			pts = append(pts, "void*")
			ins, _ := fc.g.tcx.InstantiateFnSig(d, args)
			for _, in := range ins[1:] {
				pts = append(pts, fc.ctype(in))
			}
			fat := fc.dynFat(recv, ins[0])
			fp := fmt.Sprintf("((%s(*)(%s))((void**)(%s).meta)[%d])", fc.ctype(ret), strings.Join(pts, ", "), fat, idx)
			return fmt.Sprintf("%s(%s)", fp, strings.Join(append([]string{fat + ".ptr"}, vals[1:]...), ", "))
		}
		rd, rargs, ok := fc.g.tcx.ResolveAssocFn(d, args)
		if !ok || rd == nil {
			fc.g.fatalf("%s: error: no implementation of `%s` for `%s` (args %v)", sp.Lo, d.Path(), args[0], args)
		}
		if rd == d && d.Fn.Ast.Body == nil {
			// builtin impl (Clone for closures, Fn traits, ...)
			return fc.builtinTraitCall(d, args, vals, ret)
		}
		d, args = rd, rargs
	}
	if d.Intrinsic != "" {
		return fc.intrinsic(d, args, vals, ret, sp)
	}
	name := fc.g.fnName(d, args)
	if hasTrackCaller(d) {
		// pass our location (or the one we were given) after evaluating
		// the arguments, which may make tracked calls themselves
		var b strings.Builder
		b.WriteString("({ ")
		as := make([]string, len(vals))
		for i, v := range vals {
			as[i] = fc.newName("tca")
			fmt.Fprintf(&b, "__auto_type %s = %s; ", as[i], v)
		}
		if fc.trackCaller {
			b.WriteString("rg_caller_loc = __loc; rg_caller_tracked = 1; ")
		} else {
			fmt.Fprintf(&b, "rg_caller_loc = &%s; rg_caller_tracked = 0; ", fc.g.locStatic(sp))
		}
		fmt.Fprintf(&b, "%s(%s); })", name, strings.Join(as, ", "))
		return b.String()
	}
	return fmt.Sprintf("%s(%s)", name, strings.Join(vals, ", "))
}

// builtinTraitCall implements trait methods with compiler-provided impls.
func (fc *fnGen) builtinTraitCall(d *Def, args []Ty, vals []string, ret Ty) string {
	switch d.Parent.LangItem {
	case "clone":
		if args[0].Kind == TClosure && !fc.g.tcx.IsCopy(args[0]) {
			return fmt.Sprintf("%s(%s)", fc.g.closureCloneFn(args[0]), vals[0])
		}
		// bitwise copy of *self
		return "(*" + vals[0] + ")"
	case "fn", "fn_mut", "fn_once":
		self := args[0]
		tupleTy := args[1]
		tup := fc.spillIfNeeded(vals[1], tupleTy)
		var as []string
		for i := range tupleTy.Elems {
			as = append(as, fmt.Sprintf("%s.f%d", tup, i))
		}
		recv := vals[0]
		st := self
		if d.Parent.LangItem == "fn_once" {
			// self by value
			p, _ := fc.materializeNoDrop(recv, self)
			recv = "&" + p
		}
		for st.Kind == TRef {
			if d.Parent.LangItem != "fn_once" || st != self {
				recv = "(*" + recv + ")"
			}
			st = st.Elem
			recv = "&" + recv
		}
		switch st.Kind {
		case TClosure:
			return fmt.Sprintf("%s(%s)", fc.g.closureFn(st), strings.Join(append([]string{recv}, as...), ", "))
		case TFnDef:
			return fmt.Sprintf("%s(%s)", fc.g.fnValue(st), strings.Join(as, ", "))
		case TFnPtr:
			return fmt.Sprintf("(*%s)(%s)", recv, strings.Join(as, ", "))
		}
	}
	fc.g.fatalf("unsupported builtin trait method `%s` for `%s`", d.Path(), args[0])
	return ""
}

// closureCloneFn returns a function cloning a closure capture by capture
// (a closure is Clone when its by-value captures are).
func (g *Gen) closureCloneFn(t Ty) string {
	key := "cclone|" + t.Key()
	if n, ok := g.vtables[key]; ok {
		return n
	}
	g.fnN++
	name := fmt.Sprintf("cclone%d", g.fnN)
	g.vtables[key] = name
	g.defineByValue(t)
	ct := g.ctype(t)
	ci := g.tcx.Closures[t]
	var b strings.Builder
	fmt.Fprintf(&b, "static %s %s(%s* self) { %s r;", ct, name, ct, ct)
	for i, c := range ci.Captures {
		if c.ByRef {
			fmt.Fprintf(&b, " r.c%d = self->c%d;", i, i)
			continue
		}
		fmt.Fprintf(&b, " r.c%d = %s;", i, g.cloneValue(c.Ty, fmt.Sprintf("&self->c%d", i)))
	}
	b.WriteString(" return r; }\n")
	fmt.Fprintf(&g.protos, "static %s %s(%s* self);\n", ct, name, ct)
	g.fns.WriteString(b.String())
	return name
}

// cloneValue is a C expression cloning the value of type ty at ptr.
func (g *Gen) cloneValue(ty Ty, ptr string) string {
	if g.tcx.IsCopy(ty) {
		return "(*" + ptr + ")"
	}
	if ty.Kind == TClosure {
		return fmt.Sprintf("%s(%s)", g.closureCloneFn(ty), ptr)
	}
	cl := g.tcx.Lang("clone").Trait.Items["clone"]
	rd, rargs, ok := g.tcx.ResolveAssocFn(cl, []Ty{ty})
	if !ok || rd == nil || rd.Fn == nil || rd.Fn.Ast == nil || rd.Fn.Ast.Body == nil {
		g.fatalf("error: no implementation of `Clone::clone` for `%s`", ty)
	}
	return fmt.Sprintf("%s(%s)", g.fnName(rd, rargs), ptr)
}

// closurePtrThunk returns a plain function pointer for a non-capturing closure.
func (g *Gen) closurePtrThunk(t Ty) string {
	key := "cthunk|" + t.Key()
	if n, ok := g.vtables[key]; ok {
		return n
	}
	g.fnN++
	name := fmt.Sprintf("cthunk%d", g.fnN)
	g.vtables[key] = name
	ci := g.tcx.Closures[t]
	var ps, as []string
	for i, in := range ci.Inputs {
		ps = append(ps, fmt.Sprintf("%s a%d", g.ctype(in), i))
		as = append(as, fmt.Sprintf("a%d", i))
	}
	cf := g.closureFn(t)
	g.defineByValue(t)
	fmt.Fprintf(&g.protos, "static %s %s(%s);\n", g.ctype(ci.Output), name, joinParams(ps))
	fmt.Fprintf(&g.fns, "static %s %s(%s) { %s env = {}; return %s(%s); }\n", g.ctype(ci.Output), name, joinParams(ps), g.ctype(t), cf, strings.Join(append([]string{"&env"}, as...), ", "))
	return name
}

// promotable reports whether e is a constant expression that rustc would
// promote to a 'static allocation when borrowed.
func (fc *fnGen) promotable(e ast.Expr) bool {
	switch x := e.(type) {
	case *ast.LitExpr:
		return x.Kind != ast.LitByteStr
	case *ast.ParenExpr:
		return fc.promotable(x.X)
	case *ast.UnaryExpr:
		if x.Op == ast.UnNeg {
			if _, ok := unparen(x.X).(*ast.LitExpr); ok {
				return fc.r.Callees[x.ID] == nil
			}
		}
	case *ast.TupleExpr:
		for _, el := range x.Elems {
			if !fc.promotable(el) || !fc.constAdjOK(el) {
				return false
			}
		}
		return true
	case *ast.ArrayExpr:
		for _, el := range x.Elems {
			if !fc.promotable(el) || !fc.constAdjOK(el) {
				return false
			}
		}
		return true
	case *ast.RefExpr:
		// nested borrows in constant tables: `&[(c, &['a', 'b'])]`
		if x.Mut || len(fc.r.Adjust[x.X.GetID()]) > 0 {
			return false
		}
		if _, isArr := unparen(x.X).(*ast.ArrayExpr); !isArr {
			if _, isTup := unparen(x.X).(*ast.TupleExpr); !isTup {
				return false
			}
		}
		return fc.promotable(x.X)
	case *ast.PathExpr:
		vr := fc.r.Paths[x.ID]
		return vr != nil && vr.Kind == typeck.VUnitCtor
	}
	return false
}

// constAdjOK reports whether the adjustments of an element of a constant
// aggregate can be applied in a C static initializer: none, or a borrowed
// array unsized to a slice reference.
func (fc *fnGen) constAdjOK(e ast.Expr) bool {
	adj := fc.r.Adjust[e.GetID()]
	if len(adj) == 0 {
		return true
	}
	if _, isRef := unparen(e).(*ast.RefExpr); !isRef {
		return false
	}
	last := adj[len(adj)-1]
	if last.Kind != typeck.AdjUnsize || last.Target == nil || last.Target.Kind != TRef || last.Target.Elem.Kind != TSlice {
		return false
	}
	for _, a := range adj {
		switch a.Kind {
		case typeck.AdjDeref, typeck.AdjBorrow, typeck.AdjUnsize:
		default:
			return false
		}
	}
	return true
}

// promotableDyn reports whether e is a constant expression involving const
// items (which rustc promotes, but whose value we compute at run time).
func (fc *fnGen) promotableDyn(e ast.Expr) bool {
	hasConst := false
	var ok func(e ast.Expr) bool
	ok = func(e ast.Expr) bool {
		if fc.promotable(e) {
			return true
		}
		switch x := e.(type) {
		case *ast.ParenExpr:
			return ok(x.X)
		case *ast.PathExpr:
			vr := fc.r.Paths[x.ID]
			if vr != nil && (vr.Kind == typeck.VConst || vr.Kind == typeck.VFn) {
				hasConst = true
				return true
			}
		case *ast.FieldExpr:
			// `&CONST.field`
			return ok(x.X) && hasConst
		case *ast.StructExpr:
			if x.Base != nil || x.Rest {
				return false
			}
			for _, f := range x.Fields {
				if !ok(f.Expr) {
					return false
				}
			}
			hasConst = true
			return true
		case *ast.BinaryExpr:
			// arithmetic on constants (`&(1 << 40)`)
			if fc.r.Callees[x.ID] != nil || x.Op == ast.BinAnd || x.Op == ast.BinOr {
				return false
			}
			hasConst = true
			return ok(x.L) && ok(x.R)
		case *ast.UnaryExpr:
			if x.Op == ast.UnDeref || fc.r.Callees[x.ID] != nil {
				return false
			}
			hasConst = true
			return ok(x.X)
		case *ast.CastExpr:
			hasConst = true
			return ok(x.X)
		case *ast.CallExpr:
			pe, isPath := unparen(x.Fn).(*ast.PathExpr)
			if !isPath {
				return false
			}
			vr := fc.r.Paths[pe.ID]
			// constructors and #[rustc_promotable] const fns
			// (RawWakerVTable::new)
			if vr == nil || vr.Kind != typeck.VFn || (vr.Def.Kind != DefStruct && vr.Def.Kind != DefVariant && !vr.Def.HasAttr("rustc_promotable")) {
				return false
			}
			for _, a := range x.Args {
				if !ok(a) {
					return false
				}
			}
			hasConst = true
			return true
		case *ast.TupleExpr:
			for _, el := range x.Elems {
				if !ok(el) {
					return false
				}
			}
			return true
		case *ast.ArrayExpr:
			for _, el := range x.Elems {
				if !ok(el) {
					return false
				}
			}
			return true
		}
		return false
	}
	return ok(e) && hasConst
}

// constC renders a promotable constant expression as a C constant.
func (fc *fnGen) constC(e ast.Expr) string {
	t := fc.ty(e)
	switch x := e.(type) {
	case *ast.ParenExpr:
		return fc.constC(x.X)
	case *ast.TupleExpr:
		if len(x.Elems) == 0 {
			return fc.unit()
		}
		var vs []string
		for i, el := range x.Elems {
			vs = append(vs, fmt.Sprintf(".f%d = %s", i, fc.constC(el)))
		}
		fc.g.defineByValue(t)
		return fmt.Sprintf("((%s){ %s })", fc.g.ctype(t), strings.Join(vs, ", "))
	case *ast.ArrayExpr:
		var vs []string
		for _, el := range x.Elems {
			vs = append(vs, fc.constC(el))
		}
		fc.g.defineByValue(t)
		return fmt.Sprintf("((%s){ { %s } })", fc.g.ctype(t), strings.Join(vs, ", "))
	case *ast.RefExpr:
		// static storage for the borrowed constant; an array borrowed as
		// a slice becomes a constant fat pointer
		it := fc.ty(x.X)
		inner := fc.constC(x.X)
		name := fc.newName("prom")
		fc.g.defineByValue(it)
		fmt.Fprintf(&fc.g.statics, "static %s %s = %s;\n", fc.g.ctype(it), name, inner)
		if adj := fc.r.Adjust[x.ID]; len(adj) > 0 && it.Kind == TArray {
			target := adj[len(adj)-1].Target
			fc.g.defineByValue(target)
			return fmt.Sprintf("((%s){ (void*)%s.a, %d })", fc.ctype(target), name, fc.g.arrayLen(it))
		}
		return "(&" + name + ")"
	}
	return fc.value(e)
}

// dynFat finds the fat pointer inside a trait-object receiver value: the
// receiver may be &dyn T, Box<dyn T>, Pin<&mut dyn T>, Pin<Box<dyn T>>, ...
// (single-pointer wrappers are ABI-identical to the data pointer, so the
// implementation method can be called with the data pointer).
func (fc *fnGen) dynFat(v string, t Ty) string {
	for t != nil && t.Kind == TAdt {
		fts := fc.g.fieldTypes(t, 0)
		if len(fts) == 0 {
			break
		}
		v = "(" + v + ").f0"
		t = fts[0]
	}
	return v
}

// bracketSpan is the span rustc reports for an overloaded index operation:
// starting at the `[` after the indexed expression.
func bracketSpan(x *ast.IndexExpr) ast.Span {
	sp := x.Span
	hi := x.X.GetSpan().Hi
	f := hi.File
	if f < 0 || int(f) >= len(lexer.Files) {
		return sp
	}
	src := lexer.Files[f].Src
	off, col, line := hi.Off, hi.Col, hi.Line
	for int(off) < len(src) && src[off] != '[' {
		if src[off] == '\n' {
			line++
			col = 1
		} else {
			col++
		}
		off++
	}
	if int(off) >= len(src) {
		return sp
	}
	sp.Lo = lexer.Pos{File: f, Off: off, Line: line, Col: col}
	return sp
}

// scalarConstLit renders an integer/bool/char const item as a C literal
// when its value is known at compile time (no accessor call per use).
func (g *Gen) scalarConstLit(d *Def, args []Ty, t Ty) (string, bool) {
	switch t.Kind {
	case TInt, TBool, TChar:
	default:
		return "", false
	}
	key := typeck.InstKey(d, args)
	if lit, ok := g.scalarConsts[key]; ok {
		return lit, lit != ""
	}
	g.scalarConsts[key] = ""
	n, ok := g.tcx.ScalarConst(d, args)
	if !ok {
		return "", false
	}
	var lit string
	switch {
	case t.Kind == TBool:
		lit = "((uint8_t)" + n.String() + ")"
	case t.Kind == TChar:
		lit = "((uint32_t)" + n.String() + "U)"
	case t.Kind == TInt && (t.IntKind() == I128 || t.IntKind() == U128):
		u := new(big.Int).Set(n)
		if u.Sign() < 0 {
			u.Add(u, new(big.Int).Lsh(big.NewInt(1), 128))
		}
		lit = fmt.Sprintf("((%s)%s)", g.ctype(t), u128Const(u.String()))
	default:
		if n.Sign() < 0 {
			lit = fmt.Sprintf("((%s)(%sLL))", g.ctype(t), n.String())
		} else {
			lit = fmt.Sprintf("((%s)%sULL)", g.ctype(t), n.String())
		}
	}
	g.scalarConsts[key] = lit
	return lit, true
}

func (fc *fnGen) isBoxTy(t Ty) bool {
	return t != nil && t.Kind == TAdt && t.Def != nil && t.Def.LangItem == "owned_box"
}

// upvarOperand is the place of capture i of the closure being generated.
func (fc *fnGen) upvarOperand(i int, t Ty) operand {
	c := fc.closure.Captures[i]
	if c.ByRef {
		return operand{s: fmt.Sprintf("(*env->c%d)", i), place: true, ty: t}
	}
	flag := ""
	if fc.closure.Kind == 2 && fc.g.needsDrop(c.Ty) {
		// FnOnce body owns its by-value captures
		flag = fmt.Sprintf("dfc%d", i)
	}
	return operand{s: fmt.Sprintf("env->c%d", i), place: true, flag: flag, movable: true, ty: t}
}

// capturePlace evaluates the place a closure being created captures (in
// the enclosing body, possibly itself a closure capturing a part of it)
// and returns it with the drop flag to clear when it is moved.
func (fc *fnGen) capturePlace(c *typeck.Capture) (string, string) {
	if len(c.Path) == 0 {
		return fc.localPlace(c.Local)
	}
	var base, flag string
	baseTy := c.Local.Ty
	rest := c.Path
	if fc.pathCaps[c.Local] {
		// a part of a place the enclosing closure captured
		found := false
		for j, oc := range fc.closure.Captures {
			if oc.Local == c.Local && len(oc.Path) > 0 && typeck.PathPrefix(oc.Path, c.Path) {
				op := fc.upvarOperand(j, oc.Ty)
				base, flag, baseTy, rest = op.s, op.flag, oc.Ty, c.Path[len(oc.Path):]
				found = true
				break
			}
		}
		if !found {
			fc.g.fatalf("%s: internal error: captured place of `%s` not found in the enclosing closure", c.Local.Span.Lo, c.Local.Name)
		}
	} else {
		base, flag = fc.localPlace(c.Local)
		if _, captured := fc.capIndex[c.Local]; !captured || fc.ownedUpvars[c.Local] {
			// the longest tracked field path of the captured place
			var path []int
			for _, pr := range rest {
				if pr.Deref {
					break
				}
				path = append(path, pr.Field)
			}
			for k := len(path); k > 0; k-- {
				if fc.tracked(c.Local, path[:k]) {
					flag = fc.pathFlags(c.Local, path[:k])
					break
				}
			}
		}
	}
	p := base
	cur := baseTy
	for _, pr := range rest {
		if pr.Deref {
			flag = "" // moves never go through a dereference
			if cur.Kind == TAdt {
				p = "(*" + p + ".f0)" // Box
			} else {
				p = "(*" + p + ")"
			}
		} else {
			p = fmt.Sprintf("%s.f%d", p, pr.Field)
		}
		cur = pr.Ty
	}
	return p, flag
}
