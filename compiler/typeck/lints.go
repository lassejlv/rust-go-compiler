package typeck

// Deny-by-default lints that rustc reports as errors: overflowing_literals
// (integer and float literals out of range for their type). Like rustc
// under cargo's `--cap-lints allow` for dependencies, they are only
// reported in local crates, and `#[allow(...)]` on the body, an enclosing
// item/module/crate or a statement/expression disables them. Unary minus
// on unsigned integers (E0600) is checked here as well.

import (
	"math"
	"math/big"
	"strconv"
	"strings"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

// attrsAllowLint reports whether attrs contain `#[allow(name)]` (or
// `expect`/`warn`, which also lower a deny lint to a non-error level).
func attrsAllowLint(attrs []*ast.Attr, name string) bool {
	for _, a := range attrs {
		switch a.Path {
		case "allow", "expect", "warn":
		case "cfg_attr":
			// `#[cfg_attr(pred, allow(x))]` is expanded before typeck;
			// a leftover one is ignored
			continue
		default:
			continue
		}
		if tokensMention(a.Tokens, name) {
			return true
		}
	}
	return false
}

func tokensMention(ts []ast.TokenTree, name string) bool {
	for _, t := range ts {
		if t.Delim != 0 {
			if tokensMention(t.Inner, name) {
				return true
			}
			continue
		}
		if t.Tok.Text == name {
			return true
		}
	}
	return false
}

// lintAllowedAt reports whether lint name is allowed for the whole body of
// def (by an attribute on def or any enclosing item, module or the crate).
func lintAllowedAt(def *Def, name string) bool {
	for d := def; d != nil; d = d.Parent {
		if attrsAllowLint(d.Attrs, name) {
			return true
		}
		if d.Item != nil && attrsAllowLint(d.Item.Attrs, name) {
			return true
		}
		if d.Kind == DefCrate {
			if d.Crate != nil && d.Crate.Ast != nil && attrsAllowLint(d.Crate.Ast.Attrs, name) {
				return true
			}
		}
	}
	if def != nil && def.Crate != nil && def.Crate.Ast != nil && attrsAllowLint(def.Crate.Ast.Attrs, name) {
		return true
	}
	return false
}

// checkLints runs the deny-by-default lints over a checked body.
func (t *Tcx) checkLints(r *Results, def *Def, root any) {
	if def == nil || def.Crate == nil || !def.Crate.Local {
		return
	}
	if lintAllowedAt(def, "overflowing_literals") {
		return
	}
	negated := map[*ast.LitExpr]bool{}
	ast.Walk(root, func(n any) bool {
		if attrsAllowLint(exprOrStmtAttrs(n), "overflowing_literals") {
			return false
		}
		switch x := n.(type) {
		case *ast.ItemStmt:
			return false
		case *ast.UnaryExpr:
			if x.Op == ast.UnNeg {
				if l, ok := unparen(x.X).(*ast.LitExpr); ok {
					negated[l] = true
				}
			}
		case *ast.LitExpr:
			ty := r.NodeTypes[x.ID]
			if ty == nil {
				return true
			}
			t.checkLitRange(x, ty, negated[x])
		}
		return true
	})
}

// exprOrStmtAttrs returns the outer attributes of a statement or expression.
func exprOrStmtAttrs(n any) []*ast.Attr {
	switch x := n.(type) {
	case *ast.LetStmt:
		return x.Attrs
	case *ast.MacStmt:
		return x.Attrs
	case *ast.Arm:
		return x.Attrs
	case ast.Expr:
		return ast.ExprAttrs(x)
	}
	return nil
}

// checkLitRange reports a literal whose value does not fit its type.
func (t *Tcx) checkLitRange(x *ast.LitExpr, ty Ty, neg bool) {
	switch {
	case x.Kind == ast.LitInt && ty.Kind == TInt:
		it := ty.IntKind()
		v := new(big.Int)
		if x.Big != "" {
			if _, ok := v.SetString(x.Big, 10); !ok {
				return
			}
		} else {
			v.SetUint64(x.IntVal)
		}
		bits := uint(it.Bits())
		max := new(big.Int)
		if it.Signed() {
			max.Lsh(big.NewInt(1), bits-1) // 2^(n-1): allowed only when negated
			if !neg {
				max.Sub(max, big.NewInt(1))
			}
		} else {
			max.Lsh(big.NewInt(1), bits)
			max.Sub(max, big.NewInt(1))
		}
		if v.Cmp(max) > 0 {
			t.Errorf(x.Span, "literal out of range for `%s` (#[deny(overflowing_literals)] on by default)", it)
		}
	case (x.Kind == ast.LitFloat || x.Kind == ast.LitInt) && ty.Kind == TFloat:
		if x.Kind == ast.LitInt && x.Big == "" && x.IntVal < 1<<53 {
			return
		}
		text := strings.ReplaceAll(x.Value, "_", "")
		if x.Kind == ast.LitInt {
			text = strconv.FormatUint(x.IntVal, 10)
			if x.Big != "" {
				text = x.Big
			}
		}
		bits := 64
		if ty.Sub == TyF32.Sub {
			bits = 32
		}
		f, err := strconv.ParseFloat(text, bits)
		if err != nil && !math.IsInf(f, 0) {
			return
		}
		if math.IsInf(f, 0) {
			t.Errorf(x.Span, "literal out of range for `%s` (#[deny(overflowing_literals)] on by default)", ty)
		}
	}
}

// checkNegUnsigned reports `-x` on an unsigned integer (E0600).
func (t *Tcx) checkNegUnsigned(r *Results, x *ast.UnaryExpr) {
	if x.Op != ast.UnNeg {
		return
	}
	ty := r.NodeTypes[x.X.GetID()]
	if ty != nil && ty.Kind == TInt && !ty.IntKind().Signed() {
		t.ErrorCode(x.Span, "E0600", "cannot apply unary operator `-` to type `%s`", ty)
	}
}
