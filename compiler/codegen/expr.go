package codegen

import (
	"fmt"
	"math"
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
		flag := base.flag
		if pe, ok := unparen(x.X).(*ast.PathExpr); ok && len(fc.r.Adjust[x.X.GetID()]) == 0 {
			if vr := fc.r.Paths[pe.ID]; vr != nil && vr.Kind == typeck.VLocal && fc.split[vr.Local][fr.Index] && fc.capIndex[vr.Local] == 0 {
				if _, captured := fc.capIndex[vr.Local]; !captured {
					flag = fmt.Sprintf("%s_%d", fc.flagName(vr.Local), fr.Index)
				}
			}
		}
		return operand{s: fmt.Sprintf("%s.f%d", base.s, fr.Index), place: true, flag: flag, movable: base.movable, ty: t}
	case *ast.IndexExpr:
		return fc.indexOperand(x, t)
	case *ast.RefExpr:
		inner := fc.adjusted(x.X)
		if x.Mut {
			// ensure place
		}
		return val(fc.addrOf(inner))
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
		for i, el := range x.Elems {
			vs = append(vs, fmt.Sprintf(".f%d = %s", i, fc.spillIfNeeded(fc.value(el), fc.aty(el))))
		}
		return val(fmt.Sprintf("((%s){ %s })", fc.ctype(t), strings.Join(vs, ", ")))
	case *ast.ArrayExpr:
		var vs []string
		for _, el := range x.Elems {
			vs = append(vs, fc.spillIfNeeded(fc.value(el), fc.aty(el)))
		}
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

func intLit(x *ast.LitExpr, t Ty, ct string) string {
	if x.Big != "" {
		// 128-bit literal from decimal text
		return fmt.Sprintf("((%s)rg_u128_from_str(%q))", ct, x.Big)
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
				return operand{s: fmt.Sprintf("((%s)(0 - (unsigned __int128)rg_u128_from_str(%q)))", fc.ctype(t), lit.Big), ty: t}
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
			v := fc.callResolved(c.Def, c.Args, []string{l, r}, TyBool)
			return fc.spill(v, TyBool)
		}
		l := fc.spillIfNeeded(fc.value(x.L), fc.aty(x.L))
		r := fc.value(x.R)
		return fc.spill(fc.callResolved(c.Def, c.Args, []string{l, r}, t), t)
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
		fc.line("%s;", fc.callResolved(c.Def, c.Args, []string{l, r}, TyUnit))
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
	v := fc.spill(fc.callResolved(c.Def, c.Args, []string{recv, idx}, rt), rt)
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
	switch {
	case from.Kind == TFloat && t.Kind == TInt:
		return fmt.Sprintf("RG_F2I(%s, %s, %s, %s)", ct, v, intMinStr(t), intMaxStr(t))
	case from.Kind == TAdt && from.Def.Adt.Kind == AdtEnum:
		return fmt.Sprintf("((%s)(%s).tag)", ct, fc.spillIfNeeded(v, from))
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
	for _, f := range x.Fields {
		i := v.FieldIndex(f.Name.Name)
		vals[i] = fc.spillIfNeeded(fc.value(f.Expr), fc.aty(f.Expr))
	}
	if x.Base != nil {
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
		p, flag := fc.localPlace(c.Local)
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
		fc.line("if (%s.tag != %d) {", v, some.Disc)
		fc.ind++
		none := tcx.Lang("None")
		fc.emitReturn(fc.g.adtLiteral(info.OutTy, none, nil))
		fc.ind--
		fc.line("}")
		if isZST(t) {
			return fc.unit()
		}
		return fmt.Sprintf("%s.u.v%d.f0", v, some.Index)
	}
	ok := tcx.Lang("Ok").Variant
	errD := tcx.Lang("Err")
	fc.line("if (%s.tag != %d) {", v, ok.Disc)
	fc.ind++
	e := fmt.Sprintf("%s.u.v%d.f0", v, errD.Variant.Index)
	if info.From != nil {
		e = fc.spill(fc.callResolved(info.From.Def, info.From.Args, []string{e}, info.OutErrTy), info.OutErrTy)
	}
	fc.emitReturn(fc.g.adtLiteral(info.OutTy, errD, []string{e}))
	fc.ind--
	fc.line("}")
	if isZST(t) {
		return fc.unit()
	}
	return fmt.Sprintf("%s.u.v%d.f0", v, ok.Index)
}

// ---------------------------------------------------------------------------
// Calls

// argValues lowers call arguments left to right.
func (fc *fnGen) argValues(args []ast.Expr) []string {
	var out []string
	for _, a := range args {
		v := fc.value(a)
		if !isSimple(v) && !strings.HasPrefix(v, "((") {
			v = fc.spill(v, fc.aty(a))
		}
		out = append(out, v)
	}
	return out
}

func (fc *fnGen) callExpr(x *ast.CallExpr, t Ty) string {
	c := fc.r.Callees[x.ID]
	if c == nil {
		fc.g.fatalf("%s: unresolved call", x.Span.Lo)
	}
	switch c.Kind {
	case typeck.CallFn, typeck.CallCtor:
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
		return fc.finishCall(fmt.Sprintf("%s(%s)", fp, strings.Join(all, ", ")), t)
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
	recv := fc.value(x.Receiver)
	if !isSimple(recv) && !strings.HasPrefix(recv, "((") {
		recv = fc.spill(recv, fc.aty(x.Receiver))
	}
	args := fc.argValues(x.Args)
	if c.Kind == typeck.CallVirtual {
		dyn := c.Args[0]
		idx := fc.g.vtableIndex(dyn, c.Def)
		var pts []string
		pts = append(pts, "void*")
		for _, a := range x.Args {
			pts = append(pts, fc.ctype(fc.aty(a)))
		}
		fp := fmt.Sprintf("((%s(*)(%s))((void**)(%s).meta)[%d])", fc.ctype(t), strings.Join(pts, ", "), recv, idx)
		all := append([]string{recv + ".ptr"}, args...)
		return fc.finishCall(fmt.Sprintf("%s(%s)", fp, strings.Join(all, ", ")), t)
	}
	all := append([]string{recv}, args...)
	return fc.finishCall(fc.callResolvedSpan(c.Def, c.Args, all, t, x.Span), t)
}

// callResolved builds a call expression to a (possibly trait) function.
func (fc *fnGen) callResolved(d *Def, args []Ty, vals []string, ret Ty) string {
	return fc.callResolvedSpan(d, args, vals, ret, d.Span)
}

func (fc *fnGen) callResolvedSpan(d *Def, args []Ty, vals []string, ret Ty, sp ast.Span) string {
	if d.InTrait() {
		if len(args) > 0 && args[0].Kind == TDyn {
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
			fp := fmt.Sprintf("((%s(*)(%s))((void**)(%s).meta)[%d])", fc.ctype(ret), strings.Join(pts, ", "), recv, idx)
			return fmt.Sprintf("%s(%s)", fp, strings.Join(append([]string{recv + ".ptr"}, vals[1:]...), ", "))
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
	return fmt.Sprintf("%s(%s)", name, strings.Join(vals, ", "))
}

// builtinTraitCall implements trait methods with compiler-provided impls.
func (fc *fnGen) builtinTraitCall(d *Def, args []Ty, vals []string, ret Ty) string {
	switch d.Parent.LangItem {
	case "clone":
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
