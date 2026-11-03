package typeck

// Checks of constant contexts outside std: only const fns may be called in
// constants, statics and const fns (E0015), and evaluating a constant must
// not overflow, divide by zero or index out of bounds (E0080).
//
// The evaluation check is a small interpreter for integer arithmetic over
// the type-checked body (literal types come from inference); anything it
// does not model makes it give up silently.

import (
	"fmt"
	"math/big"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

func (t *Tcx) checkConstContext(r *Results, def *Def) {
	if t.isStd(def) || len(r.Inst.Args) != 0 {
		return
	}
	var root any
	what := ""
	switch {
	case (def.Kind == DefConst || def.Kind == DefStatic) && r.ConstExpr != nil:
		root = r.ConstExpr
		what = "constants"
		if def.Kind == DefStatic {
			what = "statics"
		}
	case def.Kind == DefFn && def.Fn != nil && def.Fn.Ast != nil && def.Fn.Ast.Const && def.Fn.Ast.Body != nil:
		root = def.Fn.Ast.Body
		what = "constant functions"
	default:
		return
	}
	ast.Walk(root, func(n any) bool {
		var c *Callee
		var sp ast.Span
		switch x := n.(type) {
		case *ast.ItemStmt, *ast.ClosureExpr:
			return false
		case *ast.CallExpr:
			c, sp = r.Callees[x.ID], x.Span
			if pe, ok := x.Fn.(*ast.PathExpr); ok && fromMacro(pe.Path) {
				c = nil
			}
		case *ast.MethodCallExpr:
			c, sp = r.Callees[x.ID], x.Span
			if x.Method.Hyg != 0 {
				c = nil
			}
		}
		if c != nil && c.Kind == CallFn && c.Def != nil && c.Def.Kind == DefFn && c.Def.Fn != nil && c.Def.Fn.Ast != nil &&
			!c.Def.Fn.Ast.Const && c.Def.Intrinsic == "" && !c.Def.Fn.Extern {
			inTrait := c.Def.InTrait() || (c.Def.InImpl() && c.Def.Parent.Impl != nil && c.Def.Parent.Impl.Ast != nil && c.Def.Parent.Impl.Ast.Trait != nil)
			if inTrait || !t.isStd(c.Def) {
				kind := "fn"
				if c.Def.IsAssoc() {
					kind = "associated function"
					if c.Def.Fn.HasSelf {
						kind = "method"
					}
				}
				t.ErrorCode(sp, "E0015", "cannot call non-const %s `%s` in %s", kind, c.Def.Path(), what)
			}
		}
		return true
	})
	if def.Kind == DefConst || def.Kind == DefStatic {
		ev := &miniEval{t: t}
		if err := ev.run(r, r.ConstExpr); err != "" {
			t.ErrorCode(r.ConstExpr.GetSpan(), "E0080", "evaluation of constant value failed: %s", err)
		}
	}
}

type miniEval struct {
	t     *Tcx
	depth int
}

type evalStop struct{ err string } // err == "": unsupported

type mframe struct {
	r      *Results
	locals map[*Local]*big.Int
}

func (ev *miniEval) run(r *Results, e ast.Expr) (err string) {
	defer func() {
		if x := recover(); x != nil {
			if s, ok := x.(evalStop); ok {
				err = s.err
				return
			}
			if _, fatal := x.(fatalError); fatal {
				panic(x)
			}
			err = "" // unexpected: no verdict
		}
	}()
	ev.eval(&mframe{r: r, locals: map[*Local]*big.Int{}}, e)
	return ""
}

func unsupported()        { panic(evalStop{}) }
func evalFail(msg string) { panic(evalStop{err: msg}) }

func (ev *miniEval) intTy(fr *mframe, e ast.Expr) Ty {
	t := fr.r.NodeTypes[e.GetID()]
	if t == nil || t.Kind != TInt {
		unsupported()
	}
	return t
}

func lit(n *big.Int, t Ty) string { return n.String() + "_" + t.IntKind().String() }

func (ev *miniEval) eval(fr *mframe, e ast.Expr) *big.Int {
	switch x := e.(type) {
	case *ast.ParenExpr:
		return ev.eval(fr, x.X)
	case *ast.LitExpr:
		ty := ev.intTy(fr, x)
		if x.Kind != ast.LitInt {
			unsupported()
		}
		var n *big.Int
		if x.Big != "" {
			var ok bool
			if n, ok = new(big.Int).SetString(x.Big, 10); !ok {
				unsupported()
			}
		} else {
			n = new(big.Int).SetUint64(x.IntVal)
		}
		if !inRange(n, ty.IntKind()) {
			unsupported() // `-128i8`-style literals are negated later
		}
		return n
	case *ast.UnaryExpr:
		if x.Op != ast.UnNeg {
			unsupported()
		}
		ty := ev.intTy(fr, x)
		if lt, ok := x.X.(*ast.LitExpr); ok {
			// a negated literal is one value (`-128i8`)
			n := new(big.Int).SetUint64(lt.IntVal)
			if lt.Big != "" {
				unsupported()
			}
			return n.Neg(n)
		}
		v := ev.eval(fr, x.X)
		r := new(big.Int).Neg(v)
		if !inRange(r, ty.IntKind()) {
			evalFail(fmt.Sprintf("attempt to negate `%s`, which would overflow", lit(v, ty)))
		}
		return r
	case *ast.BinaryExpr:
		ty := ev.intTy(fr, x)
		if fr.r.Callees[x.ID] != nil {
			unsupported()
		}
		var opName string
		switch x.Op {
		case ast.BinAdd:
			opName = "add"
		case ast.BinSub:
			opName = "subtract"
		case ast.BinMul:
			opName = "multiply"
		case ast.BinDiv:
			opName = "divide"
		case ast.BinRem:
			opName = "calculate the remainder of"
		default:
			unsupported()
		}
		a := ev.eval(fr, x.L)
		b := ev.eval(fr, x.R)
		r := new(big.Int)
		switch x.Op {
		case ast.BinAdd:
			r.Add(a, b)
		case ast.BinSub:
			r.Sub(a, b)
		case ast.BinMul:
			r.Mul(a, b)
		case ast.BinDiv, ast.BinRem:
			if b.Sign() == 0 {
				if x.Op == ast.BinDiv {
					evalFail(fmt.Sprintf("attempt to divide `%s` by zero", lit(a, ty)))
				}
				evalFail(fmt.Sprintf("attempt to calculate the remainder of `%s` with a divisor of zero", lit(a, ty)))
			}
			if x.Op == ast.BinDiv {
				r.Quo(a, b)
			} else {
				r.Rem(a, b)
			}
		}
		if !inRange(r, ty.IntKind()) {
			sym := map[ast.BinOp]string{ast.BinAdd: "+", ast.BinSub: "-", ast.BinMul: "*", ast.BinDiv: "/", ast.BinRem: "%"}[x.Op]
			evalFail(fmt.Sprintf("attempt to compute `%s %s %s`, which would overflow", lit(a, ty), sym, lit(b, ty)))
		}
		_ = opName
		return r
	case *ast.CastExpr:
		ty := ev.intTy(fr, x)
		ev.intTy(fr, x.X)
		return wrapInt(ev.eval(fr, x.X), ty.IntKind())
	case *ast.PathExpr:
		vr := fr.r.Paths[x.ID]
		if vr == nil {
			unsupported()
		}
		switch vr.Kind {
		case VLocal:
			if v, ok := fr.locals[vr.Local]; ok {
				return v
			}
		case VConst:
			ev.intTy(fr, x)
			if n, ok := ev.t.ScalarConst(vr.Def, vr.Args); ok {
				return n
			}
		}
		unsupported()
	case *ast.IndexExpr:
		bt := fr.r.AdjustedType(x.X)
		if bt == nil || bt.Kind != TArray || bt.Len == nil || bt.Len.Kind != TConst || !fr.r.IndexBuiltin[x.ID] {
			unsupported()
		}
		idx := ev.eval(fr, x.Index)
		if idx.Sign() < 0 || idx.Cmp(new(big.Int).SetUint64(bt.Len.Val)) >= 0 {
			evalFail(fmt.Sprintf("index out of bounds: the length is %d but the index is %s", bt.Len.Val, idx))
		}
		unsupported()
	case *ast.BlockExpr:
		if x.Label != "" || x.Async || x.Const {
			unsupported()
		}
		return ev.block(fr, x.Block)
	case *ast.CallExpr:
		c := fr.r.Callees[x.ID]
		if c == nil || c.Kind != CallFn || c.Def == nil || c.Def.Kind != DefFn || len(c.Args) != 0 ||
			ev.t.isStd(c.Def) || c.Def.Fn.Ast == nil || !c.Def.Fn.Ast.Const || c.Def.Fn.Ast.Body == nil || ev.depth > 16 {
			unsupported()
		}
		args := make([]*big.Int, len(x.Args))
		for i, a := range x.Args {
			args[i] = ev.eval(fr, a)
		}
		cr := ev.t.quietCheck(c.Def)
		if cr == nil || len(cr.Params) != len(args) {
			unsupported()
		}
		nf := &mframe{r: cr, locals: map[*Local]*big.Int{}}
		for i, p := range cr.Params {
			if p == nil {
				unsupported()
			}
			nf.locals[p] = args[i]
		}
		ev.depth++
		v := ev.block(nf, c.Def.Fn.Ast.Body)
		ev.depth--
		return v
	}
	unsupported()
	return nil
}

func (ev *miniEval) block(fr *mframe, b *ast.Block) *big.Int {
	for i, s := range b.Stmts {
		switch st := s.(type) {
		case *ast.LetStmt:
			ip, ok := st.Pat.(*ast.IdentPat)
			l := fr.r.Locals[st.Pat.GetID()]
			if !ok || ip.Sub != nil || st.Init == nil || st.Else != nil || l == nil {
				unsupported()
			}
			fr.locals[l] = ev.eval(fr, st.Init)
		case *ast.ExprStmt:
			if i == len(b.Stmts)-1 && !st.Semi {
				return ev.eval(fr, st.Expr)
			}
			unsupported()
		default:
			unsupported()
		}
	}
	unsupported()
	return nil
}
