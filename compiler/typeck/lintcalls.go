package typeck

// Deny-by-default lints about calls of std functions:
// invalid_atomic_ordering (`load(Release)`, `store(Acquire)`, a
// compare_exchange failure ordering that writes, a Relaxed fence),
// let_underscore_lock (`let _ = mutex.lock();`) and
// undropped_manually_drops (`drop(ManuallyDrop::new(x))`).

import (
	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

// atomicOrdering returns the name of the std::sync::atomic::Ordering
// variant an argument names ("" if it is not one).
func atomicOrdering(r *Results, e ast.Expr) string {
	pe, ok := unparen(e).(*ast.PathExpr)
	if !ok {
		return ""
	}
	vr := r.Paths[pe.ID]
	if vr == nil || vr.Kind != VUnitCtor || vr.Def == nil || vr.Def.Kind != DefVariant {
		return ""
	}
	en := vr.Def.Parent
	if en == nil || en.Name != "Ordering" || en.Crate == nil || en.Crate.Name != "std" || en.Parent == nil || en.Parent.Name != "atomic" {
		return ""
	}
	return vr.Def.Name
}

// isAtomicMethod reports a method of one of std's atomic types.
func isAtomicMethod(d *Def) bool {
	if d == nil || d.Crate == nil || d.Crate.Name != "std" || d.Parent == nil || d.Parent.Kind != DefImpl {
		return false
	}
	p := d.Parent
	if p.Impl == nil || p.Impl.TraitRef != nil || p.Impl.SelfTy == nil || p.Impl.SelfTy.Kind != TAdt || p.Impl.SelfTy.Def == nil {
		return false
	}
	n := p.Impl.SelfTy.Def.Name
	return len(n) > 6 && n[:6] == "Atomic"
}

func (t *Tcx) checkCallLints(r *Results, def *Def, root any) {
	atomicAllowed := lintAllowedAt(def, "invalid_atomic_ordering")
	ast.Walk(root, func(n any) bool {
		switch x := n.(type) {
		case *ast.ItemStmt:
			return false
		case *ast.LetStmt:
			t.checkLetUnderscoreLock(r, def, x)
		case *ast.MethodCallExpr:
			c := r.Callees[x.ID]
			if atomicAllowed || c == nil || c.Def == nil || !isAtomicMethod(c.Def) || x.Method.Hyg != 0 {
				return true
			}
			arg := func(i int) string {
				if i < len(x.Args) {
					return atomicOrdering(r, x.Args[i])
				}
				return ""
			}
			switch c.Def.Name {
			case "load":
				if o := arg(0); o == "Release" || o == "AcqRel" {
					t.Errorf(x.Args[0].GetSpan(), "atomic loads cannot have `Release` or `AcqRel` ordering")
				}
			case "store":
				if o := arg(1); o == "Acquire" || o == "AcqRel" {
					t.Errorf(x.Args[1].GetSpan(), "atomic stores cannot have `Acquire` or `AcqRel` ordering")
				}
			case "compare_exchange", "compare_exchange_weak":
				if o := arg(3); o == "Release" || o == "AcqRel" {
					t.Errorf(x.Args[3].GetSpan(), "`%s`'s failure ordering may not be `Release` or `AcqRel`, since a failed `%s` does not result in a write", c.Def.Name, c.Def.Name)
				}
			case "fetch_update":
				if o := arg(1); o == "Release" || o == "AcqRel" {
					t.Errorf(x.Args[1].GetSpan(), "`fetch_update`'s failure ordering may not be `Release` or `AcqRel`, since a failed `fetch_update` does not result in a write")
				}
			}
		case *ast.CallExpr:
			t.checkDropManuallyDrop(r, def, x)
			c := r.Callees[x.ID]
			if atomicAllowed || c == nil || c.Def == nil || c.Def.Crate == nil || c.Def.Crate.Name != "std" || c.Def.Parent == nil || c.Def.Parent.Name != "atomic" {
				return true
			}
			if (c.Def.Name == "fence" || c.Def.Name == "compiler_fence") && len(x.Args) == 1 && atomicOrdering(r, x.Args[0]) == "Relaxed" {
				t.Errorf(x.Args[0].GetSpan(), "memory fences cannot have `Relaxed` ordering")
			}
		}
		return true
	})
}

// checkLetUnderscoreLock reports `let _ = m.lock();` (the guard is
// dropped at once: let_underscore_lock).
func (t *Tcx) checkLetUnderscoreLock(r *Results, def *Def, x *ast.LetStmt) {
	if _, ok := x.Pat.(*ast.WildPat); !ok || x.Init == nil || x.Else != nil || lintAllowedAt(def, "let_underscore_lock") {
		return
	}
	ty := r.NodeTypes[x.Init.GetID()]
	// (`lock()` returns a LockResult<MutexGuard>; also bare guards)
	found := false
	Walk(ty, func(y Ty) bool {
		if y.Kind == TRef || y.Kind == TPtr {
			return false
		}
		if y.Kind == TAdt && y.Def != nil && y.Def.Crate != nil && y.Def.Crate.Name == "std" {
			switch y.Def.Name {
			case "MutexGuard", "RwLockReadGuard", "RwLockWriteGuard":
				found = true
			}
		}
		return !found
	})
	if found {
		t.Errorf(x.Init.GetSpan(), "non-binding let on a synchronization lock")
	}
}

// checkDropManuallyDrop reports `drop(x)` of a ManuallyDrop value, which
// does nothing (undropped_manually_drops).
func (t *Tcx) checkDropManuallyDrop(r *Results, def *Def, x *ast.CallExpr) {
	c := r.Callees[x.ID]
	if c == nil || c.Def == nil || c.Def.Name != "drop" || c.Def.Crate == nil || c.Def.Crate.Name != "std" || c.Def.Parent == nil || c.Def.Parent.Name != "mem" || len(x.Args) != 1 {
		return
	}
	ty := r.AdjustedType(x.Args[0])
	if ty != nil && ty.Kind == TAdt && ty.Def != nil && ty.Def.Name == "ManuallyDrop" && t.isStd(ty.Def) && !lintAllowedAt(def, "undropped_manually_drops") {
		t.Errorf(x.Span, "calls to `std::mem::drop` with `std::mem::ManuallyDrop` instead of the inner value does nothing")
	}
}
