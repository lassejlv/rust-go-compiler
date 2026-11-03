package typeck

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
)

// checkExprCoerce checks e and coerces it to target (a coercion site).
func (fcx *FnCtxt) checkExprCoerce(e ast.Expr, target Ty) Ty {
	switch x := e.(type) {
	case *ast.ParenExpr:
		t := fcx.checkExprCoerce(x.X, target)
		return fcx.record(x.ID, t)
	case *ast.BlockExpr:
		if x.Label == "" && !x.Const && !x.Async {
			t := fcx.checkBlock(x.Block, target)
			if blockTail(x.Block) == nil {
				if fcx.shallow(t).Kind != TNever {
					fcx.demandEq(x.Span, target, TyUnit)
				}
				t = target
			}
			return fcx.record(x.ID, t)
		}
	case *ast.IfExpr:
		if x.Else != nil {
			return fcx.checkIf(x, target, true)
		}
	case *ast.MatchExpr:
		return fcx.checkMatch(x, target, true)
	}
	ty := fcx.checkExpr(e, target)
	if !fcx.coerce(e, ty, target) {
		fcx.errorf(e.GetSpan(), "mismatched types: expected `%s`, found `%s`", fcx.resolve(target), fcx.resolve(ty))
	}
	return target
}

// checkExprHas checks e and demands its type equals ty.
func (fcx *FnCtxt) checkExprHas(e ast.Expr, ty Ty) Ty {
	t := fcx.checkExpr(e, ty)
	fcx.demandEq(e.GetSpan(), ty, t)
	return t
}

// checkExpr checks an expression with an optional type hint.
func (fcx *FnCtxt) checkExpr(e ast.Expr, expected Ty) Ty {
	ty := fcx.checkExprKind(e, expected)
	if ty == nil {
		ty = TyErr
	}
	fcx.record(e.GetID(), ty)
	return ty
}

func (fcx *FnCtxt) checkExprKind(e ast.Expr, expected Ty) Ty {
	switch x := e.(type) {
	case *ast.LitExpr:
		return fcx.checkLit(x, expected)
	case *ast.PathExpr:
		return fcx.checkPathExpr(x, expected)
	case *ast.ParenExpr:
		return fcx.checkExpr(x.X, expected)
	case *ast.UnaryExpr:
		return fcx.checkUnary(x, expected)
	case *ast.BinaryExpr:
		return fcx.checkBinary(x, expected)
	case *ast.AssignExpr:
		return fcx.checkAssign(x)
	case *ast.AssignOpExpr:
		return fcx.checkAssignOp(x)
	case *ast.CallExpr:
		return fcx.checkCall(x, expected)
	case *ast.MethodCallExpr:
		return fcx.checkMethodCall(x, expected)
	case *ast.FieldExpr:
		return fcx.checkField(x)
	case *ast.IndexExpr:
		return fcx.checkIndex(x)
	case *ast.RefExpr:
		return fcx.checkRef(x, expected)
	case *ast.AsmExpr:
		return fcx.checkAsm(x)
	case *ast.CastExpr:
		target := fcx.lowerTy(x.Ty)
		var hint Ty
		if lit, ok := x.X.(*ast.LitExpr); ok && lit.Suffix == "" && (lit.Kind == ast.LitInt) {
			if tt := fcx.shallow(target); tt.Kind == TInt {
				hint = tt
			}
		}
		st := fcx.structurally(fcx.checkExpr(x.X, hint))
		switch st.Kind {
		case TRef, TAdt, TFnDef, TClosure:
			// casts that are coercions (unsizing, reborrows, fn pointers)
			s := fcx.ic.snapshot()
			prev := append([]Adjustment(nil), fcx.r.Adjust[x.X.GetID()]...)
			if !fcx.coerce(x.X, st, target) {
				fcx.ic.rollback(s)
				fcx.r.Adjust[x.X.GetID()] = prev
			}
		}
		fcx.casts = append(fcx.casts, x)
		return target
	case *ast.BlockExpr:
		return fcx.checkBlockExpr(x, expected)
	case *ast.IfExpr:
		return fcx.checkIf(x, expected, false)
	case *ast.MatchExpr:
		return fcx.checkMatch(x, expected, false)
	case *ast.LetExpr:
		ty := fcx.checkExpr(x.Expr, nil)
		fcx.checkPat(x.Pat, ty, bmByValue, false)
		return TyBool
	case *ast.WhileExpr:
		fcx.pushScope()
		fcx.checkCond(x.Cond)
		ls := fcx.loopStack()
		lc := &loopCtx{label: x.Label}
		*ls = append(*ls, lc)
		fcx.checkBlockUnit(x.Body)
		*ls = (*ls)[:len(*ls)-1]
		fcx.popScope()
		return TyUnit
	case *ast.LoopExpr:
		ls := fcx.loopStack()
		bt := expected
		if bt == nil || fcx.shallow(bt).Kind == TNever {
			bt = fcx.ic.newVar()
		}
		lc := &loopCtx{label: x.Label, isLoop: true, breakTy: bt}
		*ls = append(*ls, lc)
		fcx.checkBlockUnit(x.Body)
		*ls = (*ls)[:len(*ls)-1]
		if !lc.broke {
			return TyNever
		}
		return bt
	case *ast.ForExpr:
		return fcx.checkFor(x)
	case *ast.BreakExpr:
		lc := fcx.findLoop(x.Label, x.Span, false)
		if lc != nil {
			lc.broke = true
			if x.Value != nil {
				if lc.isLoop || lc.block {
					fcx.checkExprCoerce(x.Value, lc.breakTy)
				} else {
					fcx.errorf(x.Span, "`break` with value from a `while`/`for` loop")
					fcx.checkExpr(x.Value, nil)
				}
			} else if lc.isLoop || lc.block {
				fcx.demandEq(x.Span, lc.breakTy, TyUnit)
			}
		} else if x.Value != nil {
			fcx.checkExpr(x.Value, nil)
		}
		return TyNever
	case *ast.ContinueExpr:
		fcx.findLoop(x.Label, x.Span, true)
		return TyNever
	case *ast.ReturnExpr:
		ret := fcx.retTy
		if len(fcx.closures) > 0 {
			ret = fcx.closures[len(fcx.closures)-1].retTy
		}
		if x.Value != nil {
			fcx.checkExprCoerce(x.Value, ret)
		} else {
			fcx.demandEq(x.Span, ret, TyUnit)
		}
		return TyNever
	case *ast.TupleExpr:
		var exp []Ty
		if expected != nil {
			if et := fcx.shallow(expected); et.Kind == TTuple && len(et.Elems) == len(x.Elems) {
				exp = et.Elems
			}
		}
		elems := make([]Ty, len(x.Elems))
		for i, el := range x.Elems {
			if exp != nil {
				elems[i] = fcx.checkExprCoerce(el, exp[i])
			} else {
				elems[i] = fcx.checkExpr(el, nil)
			}
		}
		return Tuple(elems)
	case *ast.ArrayExpr:
		var elem Ty
		if expected != nil {
			if et := fcx.shallow(expected); et.Kind == TArray || et.Kind == TSlice {
				elem = et.Elem
			}
		}
		if elem == nil {
			elem = fcx.ic.newVar()
			// several distinct closures: their common type is a fn pointer
			if len(x.Elems) > 1 {
				allClosures := true
				var first *ast.ClosureExpr
				for _, el := range x.Elems {
					ce, ok := unparen(el).(*ast.ClosureExpr)
					if !ok || len(ce.Params) != len(firstClosure(x.Elems).Params) {
						allClosures = false
						break
					}
					if first == nil {
						first = ce
					}
				}
				if allClosures {
					ins := make([]Ty, len(first.Params))
					for i := range ins {
						ins[i] = fcx.ic.newVar()
					}
					elem = FnPtr(ins, fcx.ic.newVar(), false)
				}
			}
		}
		for _, el := range x.Elems {
			fcx.checkExprCoerce(el, elem)
		}
		return ArrayN(elem, uint64(len(x.Elems)))
	case *ast.RepeatExpr:
		var elem Ty
		if expected != nil {
			if et := fcx.shallow(expected); et.Kind == TArray || et.Kind == TSlice {
				elem = et.Elem
			}
		}
		if elem == nil {
			elem = fcx.ic.newVar()
		}
		fcx.checkExprCoerce(x.Elem, elem)
		fcx.ac.scope = fcx.curMod()
		n := fcx.ac.lowerConstArg(x.Count)
		if n.Kind == TError {
			fcx.checkExpr(x.Count, TyUsize)
		}
		return Array(elem, n)
	case *ast.StructExpr:
		return fcx.checkStructExpr(x, expected)
	case *ast.RangeExpr:
		return fcx.checkRange(x, expected)
	case *ast.TryExpr:
		return fcx.checkTry(x)
	case *ast.ClosureExpr:
		return fcx.checkClosure(x, expected)
	case *ast.MacCallExpr:
		fcx.errorf(x.Span, "macro `%s!` was not expanded", ast.PathString(x.Mac.Path))
		return TyErr
	case *ast.AwaitExpr:
		fcx.errorf(x.Span, "async/await is not supported")
		return TyErr
	case *ast.UnderscoreExpr:
		fcx.errorf(x.Span, "`_` can only be used on the left-hand side of an assignment")
		return TyErr
	}
	fcx.errorf(e.GetSpan(), "unsupported expression")
	return TyErr
}

func (fcx *FnCtxt) checkBlockUnit(b *ast.Block) {
	t := fcx.checkBlock(b, TyUnit)
	if blockTail(b) == nil && fcx.shallow(t).Kind != TNever {
		// ok: unit
	}
}

func (fcx *FnCtxt) checkCond(c ast.Expr) {
	fcx.checkExprHas(c, TyBool)
}

func (fcx *FnCtxt) checkBlockExpr(x *ast.BlockExpr, expected Ty) Ty {
	if x.Async {
		fcx.errorf(x.Span, "async blocks are not supported")
		return TyErr
	}
	if x.Label != "" {
		ls := fcx.loopStack()
		bt := expected
		if bt == nil {
			bt = fcx.ic.newVar()
		}
		lc := &loopCtx{label: x.Label, block: true, breakTy: bt}
		*ls = append(*ls, lc)
		t := fcx.checkBlock(x.Block, bt)
		*ls = (*ls)[:len(*ls)-1]
		if blockTail(x.Block) == nil && fcx.shallow(t).Kind != TNever {
			fcx.demandEq(x.Span, bt, TyUnit)
		}
		if fcx.shallow(t).Kind == TNever && !lc.broke {
			return TyNever
		}
		return bt
	}
	return fcx.checkBlock(x.Block, expected)
}

func (fcx *FnCtxt) checkLit(x *ast.LitExpr, expected Ty) Ty {
	switch x.Kind {
	case ast.LitInt:
		if x.Suffix != "" {
			if it, ok := IntTyByName(x.Suffix); ok {
				return IntT(it)
			}
			if x.Suffix == "f32" {
				return TyF32
			}
			if x.Suffix == "f64" {
				return TyF64
			}
			fcx.errorf(x.Span, "invalid suffix `%s` for number literal", x.Suffix)
			return TyErr
		}
		if expected != nil {
			et := fcx.shallow(expected)
			if et.Kind == TInt || (et.Kind == TInfer && et.Sub == 1) {
				return et
			}
		}
		return fcx.ic.newIntVar()
	case ast.LitFloat:
		if x.Suffix == "f32" {
			return TyF32
		}
		if x.Suffix == "f64" {
			return TyF64
		}
		if expected != nil {
			et := fcx.shallow(expected)
			if et.Kind == TFloat || (et.Kind == TInfer && et.Sub == 2) {
				return et
			}
		}
		return fcx.ic.newFloatVar()
	case ast.LitStr:
		return Ref(TyStr, false)
	case ast.LitByteStr:
		return Ref(ArrayN(TyU8, uint64(len(x.Value))), false)
	case ast.LitChar:
		return TyChar
	case ast.LitByte:
		return TyU8
	case ast.LitBool:
		return TyBool
	case ast.LitCStr:
		fcx.errorf(x.Span, "C string literals are not supported")
		return TyErr
	}
	return TyErr
}

// ---------------------------------------------------------------------------
// Operators

func (fcx *FnCtxt) isBuiltinArith(t Ty) bool {
	t = fcx.shallow(t)
	return t.Kind == TInt || t.Kind == TFloat || (t.Kind == TInfer && t.Sub != 0)
}

var binOpLang = map[ast.BinOp][2]string{
	ast.BinAdd:    {"add", "add"},
	ast.BinSub:    {"sub", "sub"},
	ast.BinMul:    {"mul", "mul"},
	ast.BinDiv:    {"div", "div"},
	ast.BinRem:    {"rem", "rem"},
	ast.BinBitAnd: {"bitand", "bitand"},
	ast.BinBitOr:  {"bitor", "bitor"},
	ast.BinBitXor: {"bitxor", "bitxor"},
	ast.BinShl:    {"shl", "shl"},
	ast.BinShr:    {"shr", "shr"},
	ast.BinEq:     {"eq", "eq"},
	ast.BinNe:     {"eq", "ne"},
	ast.BinLt:     {"partial_ord", "lt"},
	ast.BinLe:     {"partial_ord", "le"},
	ast.BinGt:     {"partial_ord", "gt"},
	ast.BinGe:     {"partial_ord", "ge"},
}

var assignOpLang = map[ast.BinOp][2]string{
	ast.BinAdd:    {"add_assign", "add_assign"},
	ast.BinSub:    {"sub_assign", "sub_assign"},
	ast.BinMul:    {"mul_assign", "mul_assign"},
	ast.BinDiv:    {"div_assign", "div_assign"},
	ast.BinRem:    {"rem_assign", "rem_assign"},
	ast.BinBitAnd: {"bitand_assign", "bitand_assign"},
	ast.BinBitOr:  {"bitor_assign", "bitor_assign"},
	ast.BinBitXor: {"bitxor_assign", "bitxor_assign"},
	ast.BinShl:    {"shl_assign", "shl_assign"},
	ast.BinShr:    {"shr_assign", "shr_assign"},
}

func isCompare(op ast.BinOp) bool {
	return op >= ast.BinEq
}

func isScalarForCompare(t Ty) bool {
	switch t.Kind {
	case TInt, TFloat, TBool, TChar, TPtr, TFnPtr:
		return true
	case TInfer:
		return t.Sub != 0
	}
	return false
}

func (fcx *FnCtxt) checkBinary(x *ast.BinaryExpr, expected Ty) Ty {
	switch x.Op {
	case ast.BinAnd, ast.BinOr:
		fcx.checkExprHas(x.L, TyBool)
		fcx.checkExprHas(x.R, TyBool)
		return TyBool
	}
	lt := fcx.checkExpr(x.L, nil)
	ls := fcx.structurally(lt)
	if isCompare(x.Op) {
		if isScalarForCompare(ls) || ls.Kind == TNever {
			rt := fcx.checkExpr(x.R, ls)
			rs := fcx.structurally(rt)
			if isScalarForCompare(rs) || rs.Kind == TInfer || rs.Kind == TNever {
				if ls.Kind != TNever && rs.Kind != TNever {
					fcx.demandEq(x.Span, ls, rs)
				}
				return TyBool
			}
			fcx.overloadedBinary(x, lt, rt, true)
			return TyBool
		}
		rv := fcx.ic.newVar()
		fcx.registerOpObligation(x.Op, lt, rv, x.Span, binOpLang)
		fcx.selectWherePossible()
		// a unique PartialEq impl fixes the rhs type: use it as a hint so
		// e.g. `Some(b"ab")` checks against `Option<&[u8]>`
		var hint Ty
		if r := fcx.shallow(rv); r.Kind != TInfer {
			hint = r
		}
		rt := fcx.checkExpr(x.R, hint)
		if !fcx.ic.unify(rv, rt) {
			// try coercing (e.g. &String vs &str)
			if !fcx.coerce(x.R, rt, rv) {
				fcx.errorf(x.Span, "can't compare `%s` with `%s`", fcx.resolve(lt), fcx.resolve(rt))
			}
		}
		fcx.overloadedBinary(x, lt, fcx.resolve(rv), true)
		return TyBool
	}
	isShift := x.Op == ast.BinShl || x.Op == ast.BinShr
	isBit := x.Op == ast.BinBitAnd || x.Op == ast.BinBitOr || x.Op == ast.BinBitXor
	if ls.Kind == TRef && fcx.isBuiltinArith(fcx.shallow(ls.Elem)) {
		// &int op int / &int op &int: builtin on the dereferenced values
		rt := fcx.checkExpr(x.R, nil)
		rs := fcx.structurally(rt)
		rinner := rs
		if rs.Kind == TRef {
			rinner = fcx.shallow(rs.Elem)
		}
		if fcx.isBuiltinArith(rinner) {
			fcx.addAdjust(x.L, Adjustment{Kind: AdjDeref, Target: ls.Elem})
			if rs.Kind == TRef {
				fcx.addAdjust(x.R, Adjustment{Kind: AdjDeref, Target: rs.Elem})
			}
			if !isShift && !fcx.ic.unify(ls.Elem, rinner) {
				fcx.errorf(x.Span, "cannot apply `%s` to `%s` and `%s`", x.Op, fcx.resolve(lt), fcx.resolve(rt))
			}
			return fcx.shallow(ls.Elem)
		}
		return fcx.overloadedBinary(x, lt, rt, false)
	}
	builtinL := fcx.isBuiltinArith(ls) || (isBit && ls.Kind == TBool)
	if builtinL {
		var hint Ty = ls
		if isShift {
			hint = nil
		}
		rt := fcx.checkExpr(x.R, hint)
		rs := fcx.structurally(rt)
		if rs.Kind == TRef && fcx.isBuiltinArith(fcx.shallow(rs.Elem)) {
			fcx.addAdjust(x.R, Adjustment{Kind: AdjDeref, Target: rs.Elem})
			rs = fcx.shallow(rs.Elem)
		}
		if fcx.isBuiltinArith(rs) || (isBit && rs.Kind == TBool) {
			if isShift {
				if !rs.IsInteger() || !ls.IsInteger() {
					fcx.errorf(x.Span, "no implementation for `%s %s %s`", fcx.resolve(lt), x.Op, fcx.resolve(rt))
				}
				return ls
			}
			if !fcx.ic.unify(ls, rs) {
				fcx.errorf(x.Span, "cannot apply `%s` to `%s` and `%s`", x.Op, fcx.resolve(lt), fcx.resolve(rt))
			}
			return ls
		}
		return fcx.overloadedBinary(x, lt, rt, false)
	}
	if ls.Kind == TInfer && ls.Sub == 0 {
		// unknown lhs: assume the same type as rhs
		rt := fcx.checkExpr(x.R, nil)
		rs := fcx.structurally(rt)
		if fcx.isBuiltinArith(rs) {
			fcx.ic.unify(ls, rs)
			return rs
		}
		return fcx.overloadedBinary(x, lt, rt, false)
	}
	rv := fcx.ic.newVar()
	fcx.registerOpObligation(x.Op, lt, rv, x.Span, binOpLang)
	fcx.selectWherePossible()
	rvr := fcx.shallow(rv)
	var rt Ty
	if rvr.Kind != TInfer {
		rt = fcx.checkExprCoerce(x.R, rvr)
	} else {
		rt = fcx.checkExpr(x.R, nil)
		fcx.ic.unify(rv, rt)
	}
	return fcx.overloadedBinary(x, lt, rt, false)
}

func (fcx *FnCtxt) registerOpObligation(op ast.BinOp, lhs, rhs Ty, sp lexer.Span, table map[ast.BinOp][2]string) {
	names, ok := table[op]
	if !ok {
		return
	}
	tr := fcx.t.LangOpt(names[0])
	if tr == nil {
		return
	}
	fcx.ic.obligations = append(fcx.ic.obligations, &Obligation{Pred: &Predicate{Trait: &TraitRef{Trait: tr, Args: []Ty{lhs, rhs}}}, Span: sp})
}

// overloadedBinary records a trait-method callee for a binary operator.
func (fcx *FnCtxt) overloadedBinary(x *ast.BinaryExpr, lt, rt Ty, cmp bool) Ty {
	names := binOpLang[x.Op]
	tr := fcx.t.LangOpt(names[0])
	if tr == nil {
		fcx.errorf(x.Span, "missing lang item for operator `%s`", x.Op)
		return TyErr
	}
	m := tr.Trait.Items[names[1]]
	if m == nil {
		fcx.errorf(x.Span, "operator trait method `%s` missing", names[1])
		return TyErr
	}
	args := []Ty{lt, rt}
	fcx.ic.obligations = append(fcx.ic.obligations, &Obligation{Pred: &Predicate{Trait: &TraitRef{Trait: tr, Args: args}}, Span: x.Span})
	callee := &Callee{Kind: CallFn, Def: m, Args: args}
	fcx.r.Callees[x.ID] = callee
	fcx.deferred = append(fcx.deferred, callee)
	if cmp {
		return TyBool
	}
	out := fcx.normalizeInBody(Projection(tr, args, "Output"), x.Span)
	return out
}

func (fcx *FnCtxt) checkAssignOp(x *ast.AssignOpExpr) Ty {
	lt := fcx.checkExpr(x.L, nil)
	ls := fcx.structurally(lt)
	isShift := x.Op == ast.BinShl || x.Op == ast.BinShr
	isBit := x.Op == ast.BinBitAnd || x.Op == ast.BinBitOr || x.Op == ast.BinBitXor
	fcx.markMutPlace(x.L)
	if fcx.isBuiltinArith(ls) || (isBit && ls.Kind == TBool) {
		var hint Ty = ls
		if isShift {
			hint = nil
		}
		rt := fcx.checkExpr(x.R, hint)
		rs := fcx.structurally(rt)
		if fcx.isBuiltinArith(rs) || (isBit && rs.Kind == TBool) {
			if !isShift {
				fcx.demandEq(x.Span, ls, rs)
			}
			return TyUnit
		}
		fcx.overloadedAssignOp(x, lt, rt)
		return TyUnit
	}
	rv := fcx.ic.newVar()
	fcx.registerOpObligation(x.Op, lt, rv, x.Span, assignOpLang)
	fcx.selectWherePossible()
	var rt Ty
	if rvr := fcx.shallow(rv); rvr.Kind != TInfer {
		rt = fcx.checkExprCoerce(x.R, rvr)
	} else {
		rt = fcx.checkExpr(x.R, nil)
		fcx.ic.unify(rv, rt)
	}
	fcx.overloadedAssignOp(x, lt, rt)
	return TyUnit
}

func (fcx *FnCtxt) overloadedAssignOp(x *ast.AssignOpExpr, lt, rt Ty) {
	names := assignOpLang[x.Op]
	tr := fcx.t.LangOpt(names[0])
	if tr == nil {
		fcx.errorf(x.Span, "binary assignment operation `%s=` cannot be applied to type `%s`", x.Op, fcx.resolve(lt))
		return
	}
	m := tr.Trait.Items[names[1]]
	args := []Ty{lt, rt}
	fcx.ic.obligations = append(fcx.ic.obligations, &Obligation{Pred: &Predicate{Trait: &TraitRef{Trait: tr, Args: args}}, Span: x.Span})
	callee := &Callee{Kind: CallFn, Def: m, Args: args}
	fcx.r.Callees[x.ID] = callee
	fcx.deferred = append(fcx.deferred, callee)
}

func (fcx *FnCtxt) checkUnary(x *ast.UnaryExpr, expected Ty) Ty {
	switch x.Op {
	case ast.UnDeref:
		t := fcx.structurally(fcx.checkExpr(x.X, nil))
		switch t.Kind {
		case TRef, TPtr:
			return t.Elem
		case TAdt:
			if t.Def.LangItem == "owned_box" {
				return t.Args[0]
			}
		case TInfer:
			fcx.errorf(x.Span, "type annotations needed (cannot dereference unknown type)")
			return TyErr
		}
		target := fcx.overloadedDeref(t, x.Span)
		if target == nil {
			fcx.errorf(x.Span, "type `%s` cannot be dereferenced", fcx.resolve(t))
			return TyErr
		}
		tr := fcx.t.Lang("deref")
		callee := &Callee{Kind: CallFn, Def: tr.Trait.Items["deref"], Args: []Ty{t}}
		fcx.r.Callees[x.ID] = callee
		fcx.deferred = append(fcx.deferred, callee)
		return target
	case ast.UnNot, ast.UnNeg:
		t := fcx.checkExpr(x.X, expected)
		s := fcx.structurally(t)
		if fcx.isBuiltinArith(s) || (x.Op == ast.UnNot && s.Kind == TBool) {
			if x.Op == ast.UnNot && s.IsFloat() {
				fcx.errorf(x.Span, "cannot apply unary operator `!` to type `%s`", s)
			}
			return s
		}
		name := "neg"
		if x.Op == ast.UnNot {
			name = "not"
		}
		tr := fcx.t.LangOpt(name)
		if tr == nil {
			fcx.errorf(x.Span, "cannot apply unary operator to type `%s`", fcx.resolve(t))
			return TyErr
		}
		callee := &Callee{Kind: CallFn, Def: tr.Trait.Items[name], Args: []Ty{t}}
		fcx.ic.obligations = append(fcx.ic.obligations, &Obligation{Pred: &Predicate{Trait: &TraitRef{Trait: tr, Args: []Ty{t}}}, Span: x.Span})
		fcx.r.Callees[x.ID] = callee
		fcx.deferred = append(fcx.deferred, callee)
		return fcx.normalizeInBody(Projection(tr, []Ty{t}, "Output"), x.Span)
	}
	return TyErr
}

// overloadedDeref returns <t as Deref>::Target, or nil if t isn't Deref.
func (fcx *FnCtxt) overloadedDeref(t Ty, sp lexer.Span) Ty {
	tr := fcx.t.LangOpt("deref")
	if tr == nil {
		return nil
	}
	ref := &TraitRef{Trait: tr, Args: []Ty{t}}
	k := fcx.ic.evaluate(&Predicate{Trait: ref}, 0)
	if k == SelNone || k == SelAmbig {
		return nil
	}
	n := fcx.ic.normalizeProjection(tr, []Ty{fcx.resolve(t)}, "Target", 0)
	if n == nil {
		return nil
	}
	return fcx.normalizeInBody(n, sp)
}

// ---------------------------------------------------------------------------
// Assignment

func (fcx *FnCtxt) checkAssign(x *ast.AssignExpr) Ty {
	switch l := x.L.(type) {
	case *ast.UnderscoreExpr:
		fcx.checkExpr(x.R, nil)
		fcx.record(l.ID, fcx.r.NodeTypes[x.R.GetID()])
		return TyUnit
	case *ast.TupleExpr:
		rt := fcx.checkExpr(x.R, nil)
		fcx.checkDestructure(l, rt)
		return TyUnit
	}
	lt := fcx.checkExpr(x.L, nil)
	fcx.checkExprCoerce(x.R, lt)
	fcx.markMutPlace(x.L)
	return TyUnit
}

// checkDestructure handles destructuring assignment `(a, b) = rhs`.
func (fcx *FnCtxt) checkDestructure(l ast.Expr, rt Ty) {
	switch x := l.(type) {
	case *ast.UnderscoreExpr:
		fcx.record(x.ID, rt)
	case *ast.TupleExpr:
		st := fcx.structurally(rt)
		if st.Kind != TTuple || len(st.Elems) != len(x.Elems) {
			elems := make([]Ty, len(x.Elems))
			for i := range elems {
				elems[i] = fcx.ic.newVar()
			}
			fcx.demandEq(x.Span, Tuple(elems), rt)
			st = Tuple(elems)
		}
		fcx.record(x.ID, rt)
		for i, el := range x.Elems {
			fcx.checkDestructure(el, st.Elems[i])
		}
	case *ast.ParenExpr:
		fcx.checkDestructure(x.X, rt)
		fcx.record(x.ID, rt)
	default:
		lt := fcx.checkExpr(l, nil)
		fcx.demandEq(l.GetSpan(), lt, rt)
		fcx.markMutPlace(l)
	}
}

// markMutPlace converts overloaded derefs/indexing in a place expression to
// their mutable variants.
func (fcx *FnCtxt) markMutPlace(e ast.Expr) {
	switch x := e.(type) {
	case *ast.ParenExpr:
		fcx.markMutPlace(x.X)
	case *ast.FieldExpr:
		fcx.markAdjustMut(x.X)
		fcx.markMutPlace(x.X)
	case *ast.IndexExpr:
		if !fcx.r.IndexBuiltin[x.ID] {
			fcx.r.MutPlace[x.ID] = true
		}
		fcx.markAdjustMut(x.X)
		fcx.markMutPlace(x.X)
	case *ast.UnaryExpr:
		if x.Op == ast.UnDeref {
			if _, ok := fcx.r.Callees[x.ID]; ok {
				fcx.r.MutPlace[x.ID] = true
				fcx.markMutPlace(x.X)
			} else {
				t := fcx.shallow(fcx.r.NodeTypes[x.X.GetID()])
				if t != nil && t.Kind == TAdt && t.Def.LangItem == "owned_box" {
					fcx.markMutPlace(x.X)
				}
			}
		}
	}
}

// markAdjustMut switches overloaded autoderef adjustments on e to DerefMut.
func (fcx *FnCtxt) markAdjustMut(e ast.Expr) {
	adj := fcx.r.Adjust[e.GetID()]
	changed := false
	for i := range adj {
		if adj[i].Kind == AdjDerefOverloaded {
			adj[i].Mut = true
			changed = true
		}
	}
	if changed || len(adj) > 0 {
		// if we only did builtin derefs of a Box, mark inner place too
	}
	if len(adj) == 0 || allBuiltinBoxDerefs(adj) {
		fcx.markMutPlace(e)
	}
}

func allBuiltinBoxDerefs(adj []Adjustment) bool {
	for _, a := range adj {
		if a.Kind != AdjDeref {
			return false
		}
	}
	return true
}

// ---------------------------------------------------------------------------
// References, fields, indexing

func (fcx *FnCtxt) checkRef(x *ast.RefExpr, expected Ty) Ty {
	var hint Ty
	if expected != nil {
		et := fcx.shallow(expected)
		if et.Kind == TRef || et.Kind == TPtr {
			hint = et.Elem
			// an unsized-tail struct pointee is only a weak hint for an
			// rvalue operand (rustc's ExpectRvalueLikeUnsized): the literal
			// is checked on its own and the borrow unsize-coerces
			if h := fcx.shallow(hint); h.Kind == TAdt && h.IsUnsized() && !isPlaceExprTy(x.X) {
				hint = nil
			}
			// likewise a call's result: an unsized expectation must not
			// flow into the callee's generics (`&id([1u8, 2])` where `&[u8]`
			// is expected infers `[u8; 2]`, then unsizes)
			if h := fcx.shallow(hint); h != nil && h.IsUnsized() {
				switch unparenExpr(x.X).(type) {
				case *ast.CallExpr, *ast.MethodCallExpr:
					hint = nil
				}
			}
		}
	}
	t := fcx.checkExpr(x.X, hint)
	if x.Mut {
		fcx.markMutPlace(x.X)
	}
	if x.Raw {
		return Ptr(t, x.Mut)
	}
	return Ref(t, x.Mut)
}

// autoderefStep is one step of an autoderef chain.
type autoderefStep struct {
	ty  Ty
	adj Adjustment // how we got here from the previous step (unused for step 0)
}

// autoderef computes the deref chain of t.
func (fcx *FnCtxt) autoderef(t Ty, sp lexer.Span) []autoderefStep {
	steps := []autoderefStep{{ty: fcx.structurally(t)}}
	cur := steps[0].ty
	for i := 0; i < 16; i++ {
		var next Ty
		var adj Adjustment
		switch cur.Kind {
		case TRef:
			next = cur.Elem
			adj = Adjustment{Kind: AdjDeref}
		case TAdt:
			if cur.Def.LangItem == "owned_box" {
				next = cur.Args[0]
				adj = Adjustment{Kind: AdjDeref}
			} else if tgt := fcx.overloadedDeref(cur, sp); tgt != nil {
				next = tgt
				tr := fcx.t.Lang("deref")
				adj = Adjustment{Kind: AdjDerefOverloaded, Callee: &Callee{Kind: CallFn, Def: tr.Trait.Items["deref"], Args: []Ty{cur}}}
			}
		}
		if next == nil {
			break
		}
		next = fcx.structurally(next)
		adj.Target = next
		steps = append(steps, autoderefStep{ty: next, adj: adj})
		cur = next
		if cur.Kind == TInfer {
			break
		}
	}
	return steps
}

// adjustmentsFor returns the adjustments to reach step i.
func adjustmentsFor(steps []autoderefStep, i int) []Adjustment {
	var out []Adjustment
	for k := 1; k <= i; k++ {
		a := steps[k].adj
		if a.Callee != nil {
			c := *a.Callee
			a.Callee = &c
		}
		out = append(out, a)
	}
	return out
}

func (fcx *FnCtxt) addAdjust(e ast.Expr, adj ...Adjustment) {
	if len(adj) == 0 {
		return
	}
	fcx.r.Adjust[e.GetID()] = append(fcx.r.Adjust[e.GetID()], adj...)
	for i := range adj {
		if adj[i].Callee != nil {
			fcx.deferred = append(fcx.deferred, adj[i].Callee)
		}
	}
}

func (fcx *FnCtxt) checkField(x *ast.FieldExpr) Ty {
	bt := fcx.checkExpr(x.X, nil)
	steps := fcx.autoderef(bt, x.Span)
	name := x.Name.Name
	// An inaccessible (private) field is skipped and autoderef continues,
	// like rustc: `r.value` on a `Ref<Node>` reaches `Node::value`, not the
	// private field of `Ref`. If no accessible field exists, fall back to the
	// first inaccessible one.
	privStep, privFi := -1, -1
	use := func(i, fi int) Ty {
		t := steps[i].ty
		v := t.Def.Adt.Variants[0]
		fcx.addAdjust(x.X, adjustmentsFor(steps, i)...)
		fcx.r.Fields[x.ID] = FieldRes{Index: fi, Union: t.Def.Adt.Kind == AdtUnion}
		return fcx.normalizeInBody(Subst(v.Fields[fi].Ty, t.Args), x.Span)
	}
	for i, st := range steps {
		t := st.ty
		switch t.Kind {
		case TAdt:
			if t.Def.Adt.Kind == AdtEnum {
				continue
			}
			fcx.t.AdtFields(t.Def)
			v := t.Def.Adt.Variants[0]
			if fi := v.FieldIndex(name); fi >= 0 {
				if i+1 < len(steps) && !fcx.fieldAccessible(t.Def, v.Fields[fi]) {
					if privStep < 0 {
						privStep, privFi = i, fi
					}
					continue
				}
				return use(i, fi)
			}
		case TTuple:
			for fi := range t.Elems {
				if itoa(fi) == name {
					fcx.addAdjust(x.X, adjustmentsFor(steps, i)...)
					fcx.r.Fields[x.ID] = FieldRes{Index: fi}
					return t.Elems[fi]
				}
			}
		case TInfer:
			if privStep >= 0 {
				return use(privStep, privFi)
			}
			fcx.errorf(x.Span, "type annotations needed: cannot access field `%s` of unknown type", name)
			return TyErr
		}
	}
	if privStep >= 0 {
		return use(privStep, privFi)
	}
	fcx.errorf(x.Name.Span, "no field `%s` on type `%s`", name, fcx.resolve(bt))
	return TyErr
}

// fieldAccessible reports whether field f of ADT adt is visible from the
// body being checked. Private fields are visible in the ADT's (normal)
// module and its descendants; pub(crate)/pub(in ..) fields within the crate.
func (fcx *FnCtxt) fieldAccessible(adt *Def, f *FieldDef) bool {
	switch f.Vis {
	case ast.VisPub:
		return true
	case ast.VisCrate, ast.VisRestricted:
		return fcx.def == nil || adt.Crate == fcx.def.Crate
	}
	if fcx.def == nil || adt.Scope == nil || fcx.def.Scope == nil {
		return true
	}
	if adt.Crate != fcx.def.Crate {
		return false
	}
	target := adt.Scope.Normal
	for m := fcx.def.Scope; m != nil; m = m.Parent {
		if m == target || m.Normal == target {
			return true
		}
	}
	return false
}

func itoa(i int) string {
	if i < 10 {
		return string(rune('0' + i))
	}
	return itoa(i/10) + string(rune('0'+i%10))
}

func (fcx *FnCtxt) checkIndex(x *ast.IndexExpr) Ty {
	bt := fcx.checkExpr(x.X, nil)
	it := fcx.checkExpr(x.Index, nil)
	its := fcx.structurally(it)
	steps := fcx.autoderef(bt, x.Span)
	idxTrait := fcx.t.LangOpt("index")
	for i, st := range steps {
		t := st.ty
		if (t.Kind == TArray || t.Kind == TSlice) && its.IsInteger() {
			fcx.ic.unify(its, TyUsize)
			fcx.addAdjust(x.X, adjustmentsFor(steps, i)...)
			fcx.r.IndexBuiltin[x.ID] = true
			return t.Elem
		}
		if idxTrait == nil || t.Kind == TInfer {
			continue
		}
		ref := &TraitRef{Trait: idxTrait, Args: []Ty{t, it}}
		k := fcx.ic.evaluate(&Predicate{Trait: ref}, 0)
		if k == SelNone {
			continue
		}
		if k == SelAmbig {
			// try committing anyway (e.g. integer var index)
			sel := fcx.ic.selectTrait(ref, 0)
			if sel.Kind != SelImpl && sel.Kind != SelObject && sel.Kind != SelBuiltin {
				if its.Kind == TInfer && its.Sub == 1 {
					fcx.ic.unify(its, TyUsize)
				}
			}
		} else {
			fcx.ic.selectTrait(ref, 0)
		}
		fcx.addAdjust(x.X, adjustmentsFor(steps, i)...)
		callee := &Callee{Kind: CallFn, Def: idxTrait.Trait.Items["index"], Args: []Ty{t, it}}
		fcx.r.Callees[x.ID] = callee
		fcx.deferred = append(fcx.deferred, callee)
		return fcx.normalizeInBody(Projection(idxTrait, []Ty{t, it}, "Output"), x.Span)
	}
	fcx.errorf(x.Span, "cannot index into a value of type `%s`", fcx.resolve(bt))
	return TyErr
}

// ---------------------------------------------------------------------------
// Control flow

func (fcx *FnCtxt) checkIf(x *ast.IfExpr, expected Ty, coerce bool) Ty {
	fcx.pushScope()
	fcx.checkCond(x.Cond)
	if x.Else == nil {
		t := fcx.checkBlock(x.Then, TyUnit)
		if blockTail(x.Then) == nil && fcx.shallow(t).Kind != TNever {
			// fine
		}
		fcx.popScope()
		if coerce && expected != nil {
			fcx.demandEq(x.Span, expected, TyUnit)
		}
		return fcx.record(x.ID, TyUnit)
	}
	var target Ty
	if coerce {
		target = expected
	}
	var thenTy Ty
	if target != nil {
		thenTy = fcx.checkBlock(x.Then, target)
		if blockTail(x.Then) == nil && fcx.shallow(thenTy).Kind != TNever {
			fcx.demandEq(x.Then.Span, target, TyUnit)
		}
	} else {
		thenTy = fcx.checkBlock(x.Then, nil)
		if expected != nil && blockTail(x.Then) != nil {
			_ = expected
		}
	}
	fcx.popScope()
	if target != nil {
		fcx.checkExprCoerce(x.Else, target)
		return fcx.record(x.ID, target)
	}
	ts := fcx.shallow(thenTy)
	elseTy := fcx.checkExpr(x.Else, hintFor(ts, expected))
	es := fcx.shallow(elseTy)
	if ts.Kind == TNever {
		return fcx.record(x.ID, elseTy)
	}
	if es.Kind == TNever {
		return fcx.record(x.ID, thenTy)
	}
	// coerce else to then; else try then to else
	if fcx.coerce(x.Else, elseTy, thenTy) {
		return fcx.record(x.ID, thenTy)
	}
	if tail := blockTail(x.Then); tail != nil && fcx.coerce(tail, thenTy, elseTy) {
		return fcx.record(x.ID, elseTy)
	}
	fcx.errorf(x.Span, "`if` and `else` have incompatible types: `%s` vs `%s`", fcx.resolve(thenTy), fcx.resolve(elseTy))
	return fcx.record(x.ID, thenTy)
}

func hintFor(t Ty, expected Ty) Ty {
	if t.Kind == TNever {
		return expected
	}
	return t
}

func (fcx *FnCtxt) checkMatch(x *ast.MatchExpr, expected Ty, coerce bool) Ty {
	st := fcx.checkExpr(x.Scrutinee, nil)
	var target Ty
	if coerce {
		target = expected
	}
	var result Ty
	allNever := true
	for _, arm := range x.Arms {
		fcx.pushScope()
		fcx.checkPat(arm.Pat, st, bmByValue, false)
		if arm.Guard != nil {
			fcx.checkCond(arm.Guard)
		}
		var at Ty
		if target != nil {
			at = fcx.checkExprCoerce(arm.Body, target)
			if fcx.shallow(fcx.r.NodeTypes[arm.Body.GetID()]).Kind != TNever {
				allNever = false
			}
		} else {
			hint := result
			if hint == nil {
				hint = expected
			}
			at = fcx.checkExpr(arm.Body, hint)
			as := fcx.shallow(at)
			if as.Kind != TNever {
				allNever = false
				if result == nil {
					result = at
				} else if !fcx.coerce(arm.Body, at, result) {
					fcx.errorf(arm.Body.GetSpan(), "`match` arms have incompatible types: expected `%s`, found `%s`", fcx.resolve(result), fcx.resolve(at))
				}
			}
		}
		fcx.popScope()
	}
	if target != nil {
		if allNever && len(x.Arms) > 0 {
			return fcx.record(x.ID, target)
		}
		return fcx.record(x.ID, target)
	}
	if allNever {
		return fcx.record(x.ID, TyNever)
	}
	return fcx.record(x.ID, result)
}

func (fcx *FnCtxt) checkFor(x *ast.ForExpr) Ty {
	itTy := fcx.checkExpr(x.Iter, nil)
	t := fcx.t
	intoIter := t.Lang("into_iterator")
	iterTrait := t.Lang("iterator")
	info := &ForInfo{}
	info.IntoIter = &Callee{Kind: CallFn, Def: intoIter.Trait.Items["into_iter"], Args: []Ty{itTy}}
	fcx.deferred = append(fcx.deferred, info.IntoIter)
	fcx.ic.obligations = append(fcx.ic.obligations, &Obligation{Pred: &Predicate{Trait: &TraitRef{Trait: intoIter, Args: []Ty{itTy}}}, Span: x.Iter.GetSpan()})
	iterTy := fcx.normalizeInBody(Projection(intoIter, []Ty{itTy}, "IntoIter"), x.Iter.GetSpan())
	fcx.selectWherePossible()
	info.IterTy = iterTy
	info.Next = &Callee{Kind: CallFn, Def: iterTrait.Trait.Items["next"], Args: []Ty{iterTy}}
	fcx.deferred = append(fcx.deferred, info.Next)
	itemTy := fcx.normalizeInBody(Projection(iterTrait, []Ty{iterTy}, "Item"), x.Iter.GetSpan())
	fcx.selectWherePossible()
	info.ItemTy = itemTy
	fcx.r.Fors[x.ID] = info
	fcx.pushScope()
	fcx.checkPat(x.Pat, itemTy, bmByValue, true)
	ls := fcx.loopStack()
	lc := &loopCtx{label: x.Label}
	*ls = append(*ls, lc)
	fcx.checkBlockUnit(x.Body)
	*ls = (*ls)[:len(*ls)-1]
	fcx.popScope()
	return TyUnit
}

func (fcx *FnCtxt) checkRange(x *ast.RangeExpr, expected Ty) Ty {
	var name string
	switch {
	case x.Start != nil && x.End != nil && x.Inclusive:
		name = "RangeInclusive"
	case x.Start != nil && x.End != nil:
		name = "Range"
	case x.Start != nil:
		name = "RangeFrom"
	case x.End != nil && x.Inclusive:
		name = "RangeToInclusive"
	case x.End != nil:
		name = "RangeTo"
	default:
		name = "RangeFull"
	}
	d := fcx.t.Lang(name)
	if name == "RangeFull" {
		return Adt(d, nil)
	}
	var elem Ty
	if expected != nil {
		if et := fcx.shallow(expected); et.Kind == TAdt && et.Def == d {
			elem = et.Args[0]
		}
	}
	if elem == nil {
		elem = fcx.ic.newVar()
	}
	if x.Start != nil {
		st := fcx.checkExpr(x.Start, elem)
		fcx.demandEq(x.Start.GetSpan(), elem, st)
	}
	if x.End != nil {
		et := fcx.checkExpr(x.End, elem)
		if !fcx.ic.unify(elem, et) {
			if !fcx.coerce(x.End, et, elem) {
				fcx.errorf(x.End.GetSpan(), "mismatched types in range: `%s` vs `%s`", fcx.resolve(elem), fcx.resolve(et))
			}
		}
	}
	return Adt(d, []Ty{elem})
}

func (fcx *FnCtxt) checkTry(x *ast.TryExpr) Ty {
	t := fcx.structurally(fcx.checkExpr(x.X, nil))
	ret := fcx.retTy
	if len(fcx.closures) > 0 {
		ret = fcx.closures[len(fcx.closures)-1].retTy
	}
	optD := fcx.t.Lang("Option")
	resD := fcx.t.Lang("Result")
	info := &TryInfo{OutTy: ret}
	fcx.r.Tries[x.ID] = info
	if t.Kind == TAdt && t.Def == optD {
		info.IsOption = true
		info.OkTy = t.Args[0]
		rv := Adt(optD, []Ty{fcx.ic.newVar()})
		if !fcx.ic.unify(ret, rv) {
			fcx.errorf(x.Span, "the `?` operator can only be used on `Option`s in a function that returns `Option`")
		}
		return t.Args[0]
	}
	if t.Kind == TAdt && t.Def == resD {
		info.OkTy = t.Args[0]
		info.ErrTy = t.Args[1]
		rs := fcx.structurally(ret)
		if rs.Kind == TInfer {
			fcx.ic.unify(rs, Adt(resD, []Ty{fcx.ic.newVar(), t.Args[1]}))
			rs = fcx.shallow(ret)
		}
		if rs.Kind != TAdt || rs.Def != resD {
			fcx.errorf(x.Span, "the `?` operator can only be used on `Result`s in a function that returns `Result` (found `%s`)", fcx.resolve(ret))
			return t.Args[0]
		}
		outErr := rs.Args[1]
		info.OutErrTy = outErr
		// identity conversion if the types unify trivially
		oe := fcx.shallow(outErr)
		se := fcx.shallow(t.Args[1])
		if oe == se {
			return t.Args[0]
		}
		if oe.Kind == TInfer {
			// like rustc, convert with From and let other uses decide the
			// error type (e.g. a closure's later `io::Result::Ok(..)`);
			// identity if nothing does
			fcx.tryErrFallback = append(fcx.tryErrFallback, [2]Ty{outErr, t.Args[1]})
		}
		from := fcx.t.Lang("from")
		info.From = &Callee{Kind: CallFn, Def: from.Trait.Items["from"], Args: []Ty{outErr, t.Args[1]}}
		fcx.deferred = append(fcx.deferred, info.From)
		fcx.ic.obligations = append(fcx.ic.obligations, &Obligation{Pred: &Predicate{Trait: &TraitRef{Trait: from, Args: []Ty{outErr, t.Args[1]}}}, Span: x.Span})
		return t.Args[0]
	}
	if t.Kind == TInfer {
		fcx.errorf(x.Span, "type annotations needed for `?`")
		return TyErr
	}
	fcx.errorf(x.Span, "the `?` operator can only be applied to values of type `Option` or `Result` (found `%s`)", fcx.resolve(t))
	return TyErr
}

func firstClosure(es []ast.Expr) *ast.ClosureExpr {
	ce, _ := unparen(es[0]).(*ast.ClosureExpr)
	if ce == nil {
		return &ast.ClosureExpr{}
	}
	return ce
}

func unparenExpr(e ast.Expr) ast.Expr {
	for {
		p, ok := e.(*ast.ParenExpr)
		if !ok {
			return e
		}
		e = p.X
	}
}

// isPlaceExprTy reports whether e is syntactically a place expression.
func isPlaceExprTy(e ast.Expr) bool {
	switch x := unparenExpr(e).(type) {
	case *ast.PathExpr, *ast.FieldExpr, *ast.IndexExpr:
		return true
	case *ast.UnaryExpr:
		return x.Op == ast.UnDeref
	}
	return false
}

// checkAsm type-checks asm! operands: inputs are values, outputs places.
func (fcx *FnCtxt) checkAsm(x *ast.AsmExpr) Ty {
	for _, op := range x.Operands {
		switch op.Kind {
		case "sym":
			fcx.checkExpr(op.In, nil)
			continue
		}
		if op.In != nil {
			t := fcx.checkExpr(op.In, nil)
			if st := fcx.shallow(t); st.Kind == TInfer && st.Sub == 1 {
				// unsuffixed integer literal inputs default like rustc
				fcx.ic.unify(st, TyI32)
			}
		}
		if op.Out != nil {
			if op.Out != op.In {
				fcx.checkExpr(op.Out, nil)
			}
			fcx.markMutPlace(op.Out)
		}
	}
	for _, o := range x.Options {
		if o == "noreturn" {
			return TyNever
		}
	}
	return TyUnit
}
