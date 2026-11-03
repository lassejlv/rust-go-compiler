package typeck

// Unsafety checking (mirrors rustc_mir_build's check_unsafety, E0133):
// calls to unsafe functions, dereferences of raw pointers, uses of mutable
// or extern statics and inline assembly need an unsafe block or function.

import (
	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

type unsafeCx struct {
	t *Tcx
	r *Results
	// rawStatic: path expressions of statics whose address is taken with
	// `&raw const`/`&raw mut` (allowed outside unsafe).
	rawStatic map[ast.NodeID]bool
}

func (t *Tcx) checkUnsafety(r *Results, def *Def) {
	cx := &unsafeCx{t: t, r: r, rawStatic: map[ast.NodeID]bool{}}
	switch {
	case def.Kind == DefFn && def.Fn != nil && def.Fn.Ast != nil && def.Fn.Ast.Body != nil:
		cx.walk(def.Fn.Ast.Body, def.Fn.Ast.Unsafe)
	case r.ConstExpr != nil:
		cx.walk(r.ConstExpr, false)
	}
}

// unsafeFn reports whether calling d requires an unsafe context.
func unsafeFn(d *Def) bool {
	if d == nil || d.Fn == nil || d.Fn.Ast == nil || d.Intrinsic != "" {
		return false
	}
	if d.Fn.Ast.Unsafe {
		return true
	}
	return d.Fn.Extern && !d.Fn.Ast.Safe
}

// placeBase returns the path expression at the base of a place expression
// (`S`, `S.f`, `S[0].g`), or nil.
func placeBase(e ast.Expr) *ast.PathExpr {
	for {
		switch x := e.(type) {
		case *ast.ParenExpr:
			e = x.X
		case *ast.FieldExpr:
			e = x.X
		case *ast.IndexExpr:
			e = x.X
		case *ast.PathExpr:
			return x
		default:
			return nil
		}
	}
}

func (cx *unsafeCx) report(sp ast.Span, what string) {
	cx.t.ErrorCode(sp, "E0133", "%s is unsafe and requires unsafe function or block", what)
}

func (cx *unsafeCx) walk(n any, inUnsafe bool) {
	ast.Walk(n, func(x any) bool {
		switch e := x.(type) {
		case *ast.ItemStmt:
			return false // nested items are checked on their own
		case *ast.Block:
			if e.Unsafe && !inUnsafe {
				for _, s := range e.Stmts {
					cx.walk(s, true)
				}
				return false
			}
		case *ast.CallExpr:
			if !inUnsafe {
				if c := cx.r.Callees[e.ID]; c != nil && (c.Kind == CallFn || c.Kind == CallVirtual) && unsafeFn(c.Def) {
					cx.report(e.Span, "call to unsafe function `"+c.Def.Path()+"`")
				}
			}
		case *ast.MethodCallExpr:
			if !inUnsafe {
				if c := cx.r.Callees[e.ID]; c != nil && (c.Kind == CallFn || c.Kind == CallVirtual) && unsafeFn(c.Def) {
					cx.report(e.Span, "call to unsafe function `"+c.Def.Path()+"`")
				}
			}
		case *ast.UnaryExpr:
			if !inUnsafe && e.Op == ast.UnDeref {
				if t := cx.r.AdjustedType(e.X); t != nil && t.Kind == TPtr {
					cx.report(e.Span, "dereference of raw pointer")
				}
			}
		case *ast.RefExpr:
			if e.Raw {
				if b := placeBase(e.X); b != nil {
					cx.rawStatic[b.ID] = true
				}
			}
		case *ast.PathExpr:
			if !inUnsafe && !cx.rawStatic[e.ID] {
				if vr := cx.r.Paths[e.ID]; vr != nil && vr.Kind == VStatic && vr.Def != nil && vr.Def.Const != nil {
					if vr.Def.Const.Mut {
						cx.report(e.Span, "use of mutable static")
					} else if vr.Def.Const.Expr == nil {
						cx.report(e.Span, "use of extern static")
					}
				}
			}
		case *ast.AsmExpr:
			if !inUnsafe {
				cx.report(e.Span, "use of inline assembly")
			}
		}
		return true
	})
}
