package typeck

// The deny-by-default `overflowing_literals` lint: an integer literal whose
// value does not fit its (inferred) type, or a float literal that rounds to
// infinity, is a hard error unless `#[allow(overflowing_literals)]` is in
// effect on an enclosing item, module or the crate.

import (
	"math/big"
	"strconv"
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
)

// lintLevelAllowed reports whether `#[allow(name)]` (or `expect`) applies
// to def: on the def itself, an enclosing item or module, or the crate.
func lintLevelAllowed(def *Def, name string) bool {
	for d := def; d != nil; d = d.Parent {
		if attrsAllow(d.Attrs, name) {
			return true
		}
		if d.Kind == DefCrate && d.Crate != nil && d.Crate.Ast != nil && attrsAllow(d.Crate.Ast.Attrs, name) {
			return true
		}
	}
	if def != nil && def.Crate != nil && def.Crate.Ast != nil && attrsAllow(def.Crate.Ast.Attrs, name) {
		return true
	}
	return false
}

func attrsAllow(attrs []*ast.Attr, name string) bool {
	for _, a := range attrs {
		if a.Path != "allow" && a.Path != "expect" {
			continue
		}
		if len(a.Tokens) == 0 || a.Tokens[0].Delim != '(' {
			continue
		}
		for _, tt := range a.Tokens[0].Inner {
			if tt.Delim == 0 && tt.Tok.Text == name {
				return true
			}
		}
	}
	return false
}

// checkOverflowingLiterals walks a type-checked body.
func (t *Tcx) checkOverflowingLiterals(r *Results, def *Def, root any) {
	if lintLevelAllowed(def, "overflowing_literals") {
		return
	}
	done := map[*ast.LitExpr]bool{}
	litOf := func(e ast.Expr) *ast.LitExpr {
		for {
			switch x := e.(type) {
			case *ast.ParenExpr:
				e = x.X
			case *ast.LitExpr:
				return x
			default:
				return nil
			}
		}
	}
	check := func(lit *ast.LitExpr, neg bool, fallback Ty) {
		if done[lit] {
			return
		}
		done[lit] = true
		ty := r.NodeTypes[lit.ID]
		if ty == nil || (ty.Kind != TInt && ty.Kind != TFloat) {
			ty = fallback
		}
		if ty == nil {
			return
		}
		switch {
		case lit.Kind == ast.LitInt && ty.Kind == TInt:
			v := new(big.Int)
			if lit.Big != "" {
				if _, ok := v.SetString(lit.Big, 10); !ok {
					return
				}
			} else {
				v.SetUint64(lit.IntVal)
			}
			if neg {
				v.Neg(v)
			}
			if v.Cmp(tyMin(ty)) < 0 || v.Cmp(tyMax(ty)) > 0 {
				t.Errorf(lit.Span, "literal out of range for `%s`", ty.IntKind())
			}
		case (lit.Kind == ast.LitFloat || lit.Kind == ast.LitInt) && ty.Kind == TFloat:
			text := strings.ReplaceAll(lit.Value, "_", "")
			if lit.Kind == ast.LitInt {
				if lit.Big != "" {
					text = lit.Big
				} else {
					text = strconv.FormatUint(lit.IntVal, 10)
				}
			}
			bits := 64
			if ty.Sub == 32 {
				bits = 32
			}
			if f, err := strconv.ParseFloat(text, bits); err != nil && (f > 1 || f < -1) {
				t.Errorf(lit.Span, "literal out of range for `f%d`", bits)
			}
		}
	}
	var patTy func(p ast.Pat) Ty
	patTy = func(p ast.Pat) Ty {
		if p == nil {
			return nil
		}
		return r.NodeTypes[p.GetID()]
	}
	ast.Walk(root, func(n any) bool {
		switch x := n.(type) {
		case *ast.ItemStmt:
			return false
		case *ast.UnaryExpr:
			if x.Op == ast.UnNeg {
				if lit := litOf(x.X); lit != nil {
					check(lit, true, nil)
				}
			}
		case *ast.LitExpr:
			check(x, false, nil)
		case *ast.LitPat:
			pt := patTy(x)
			if u, ok := x.Expr.(*ast.UnaryExpr); ok && u.Op == ast.UnNeg {
				if lit := litOf(u.X); lit != nil {
					check(lit, true, pt)
				}
			} else if lit := litOf(x.Expr); lit != nil {
				check(lit, false, pt)
			}
		case *ast.RangePat:
			pt := patTy(x)
			for _, e := range []ast.Expr{x.Lo, x.Hi} {
				if u, ok := e.(*ast.UnaryExpr); ok && u.Op == ast.UnNeg {
					if lit := litOf(u.X); lit != nil {
						check(lit, true, pt)
					}
				} else if lit := litOf(e); lit != nil {
					check(lit, false, pt)
				}
			}
		}
		return true
	})
}

// constIntOperand returns the value of an integer literal operand
// (possibly parenthesized or negated), or nil.
func constIntOperand(e ast.Expr) *big.Int {
	neg := false
	for {
		switch x := e.(type) {
		case *ast.ParenExpr:
			e = x.X
			continue
		case *ast.UnaryExpr:
			if x.Op != ast.UnNeg || neg {
				return nil
			}
			neg = true
			e = x.X
			continue
		case *ast.LitExpr:
			if x.Kind != ast.LitInt {
				return nil
			}
			v := new(big.Int)
			if x.Big != "" {
				if _, ok := v.SetString(x.Big, 10); !ok {
					return nil
				}
			} else {
				v.SetUint64(x.IntVal)
			}
			if neg {
				v.Neg(v)
			}
			return v
		}
		return nil
	}
}

// checkKnownPanics implements the simplest cases of the deny-by-default
// `arithmetic_overflow` and `unconditional_panic` lints: arithmetic on
// two integer literals that overflows, shifts by a literal amount not
// below the bit width, division or remainder by a literal zero, and
// indexing an array with a literal index past its length.
func (t *Tcx) checkKnownPanics(r *Results, def *Def, root any) {
	overflowOK := lintLevelAllowed(def, "arithmetic_overflow")
	panicOK := lintLevelAllowed(def, "unconditional_panic")
	if overflowOK && panicOK {
		return
	}
	intTy := func(e ast.Expr) Ty {
		if ty := r.NodeTypes[e.GetID()]; ty != nil && ty.Kind == TInt && len(r.Adjust[e.GetID()]) == 0 {
			return ty
		}
		return nil
	}
	binop := func(sp lexer.Span, op ast.BinOp, l, rr ast.Expr, assign bool) {
		lt := intTy(l)
		if lt == nil || intTy(rr) == nil {
			return
		}
		rv := constIntOperand(rr)
		if rv == nil {
			return
		}
		switch op {
		case ast.BinDiv, ast.BinRem:
			if rv.Sign() == 0 {
				if !panicOK {
					what := "divide"
					if op == ast.BinRem {
						what = "calculate the remainder of"
					}
					t.Errorf(sp, "this operation will panic at runtime: attempt to %s `_` with a divisor of zero", what)
				}
				return
			}
		case ast.BinShl, ast.BinShr:
			if rv.Sign() < 0 || rv.Cmp(big.NewInt(int64(lt.IntKind().Bits()))) >= 0 {
				if !overflowOK {
					t.Errorf(sp, "this arithmetic operation will overflow")
				}
			}
			return
		}
		if assign || intTy(rr) != lt {
			return
		}
		lv := constIntOperand(l)
		if lv == nil {
			return
		}
		res := new(big.Int)
		switch op {
		case ast.BinAdd:
			res.Add(lv, rv)
		case ast.BinSub:
			res.Sub(lv, rv)
		case ast.BinMul:
			res.Mul(lv, rv)
		case ast.BinDiv, ast.BinRem:
			// `MIN / -1` (and `MIN % -1`) is an unconditional panic
			if !panicOK && lt.IntKind().Signed() && rv.Cmp(big.NewInt(-1)) == 0 && lv.Cmp(tyMin(lt)) == 0 {
				t.Errorf(sp, "this operation will panic at runtime: attempt to compute `%s::MIN / -1`, which would overflow", lt.IntKind())
			}
			return
		default:
			return
		}
		if !overflowOK && (res.Cmp(tyMin(lt)) < 0 || res.Cmp(tyMax(lt)) > 0) {
			t.Errorf(sp, "this arithmetic operation will overflow")
		}
	}
	ast.Walk(root, func(n any) bool {
		switch x := n.(type) {
		case *ast.ItemStmt:
			return false
		case *ast.BinaryExpr:
			if r.Callees[x.ID] == nil {
				binop(x.Span, x.Op, x.L, x.R, false)
			}
		case *ast.AssignOpExpr:
			if r.Callees[x.ID] == nil {
				binop(x.Span, x.Op, x.L, x.R, true)
			}
		case *ast.IndexExpr:
			if panicOK || r.Callees[x.ID] != nil {
				break
			}
			at := r.NodeTypes[x.X.GetID()]
			if at == nil || at.Kind != TArray || at.Len == nil || at.Len.Kind != TConst || len(r.Adjust[x.X.GetID()]) != 0 {
				break
			}
			if iv := constIntOperand(x.Index); iv != nil && iv.Cmp(new(big.Int).SetUint64(at.Len.Val)) >= 0 {
				t.Errorf(x.Span, "this operation will panic at runtime: index out of bounds: the length is %d but the index is %s", at.Len.Val, iv)
			}
		}
		return true
	})
}
