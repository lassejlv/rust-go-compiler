package typeck

// Checks run on a type-checked body: `as` casts (E0054, E0604, E0605,
// E0606), explicit destructor calls (E0040) and locals of unsized types
// (E0277).

import (
	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

func (t *Tcx) checkBodyMisc(r *Results, def *Def) {
	var root any
	switch {
	case def.Kind == DefFn && def.Fn != nil && def.Fn.Ast != nil && def.Fn.Ast.Body != nil:
		root = def.Fn.Ast.Body
	case r.ConstExpr != nil:
		root = r.ConstExpr
	default:
		return
	}
	std := t.isStd(def)
	ast.Walk(root, func(n any) bool {
		switch x := n.(type) {
		case *ast.ItemStmt:
			return false
		case *ast.CastExpr:
			t.checkCast(r, x)
		case *ast.AssignExpr:
			t.checkAssignLHS(r, x.L, "E0070")
		case *ast.AssignOpExpr:
			t.checkAssignLHS(r, x.L, "E0067")
		case *ast.LetStmt:
			t.checkPatBindings(r, x.Pat)
		case *ast.LetExpr:
			t.checkPatBindings(r, x.Pat)
		case *ast.Arm:
			t.checkPatBindings(r, x.Pat)
		case *ast.ForExpr:
			t.checkPatBindings(r, x.Pat)
		case *ast.ClosureExpr:
			for _, p := range x.Params {
				t.checkPatBindings(r, p.Pat)
			}
		case *ast.MethodCallExpr:
			if !std {
				t.checkCallBounds(r, r.Callees[x.ID], x.Span)
			}
			if !std && x.Method.Hyg == 0 {
				if c := r.Callees[x.ID]; c != nil && isDropMethod(c.Def) {
					t.ErrorCode(x.Method.Span, "E0040", "explicit use of destructor method")
				}
			}
		case *ast.CallExpr:
			if !std {
				t.checkCallBounds(r, r.Callees[x.ID], x.Span)
			}
			if !std {
				if c := r.Callees[x.ID]; c != nil && isDropMethod(c.Def) {
					if pe, ok := x.Fn.(*ast.PathExpr); ok && !fromMacro(pe.Path) {
						t.ErrorCode(x.Fn.GetSpan(), "E0040", "explicit use of destructor method")
					}
				}
			}
		}
		return true
	})
	// locals must have a statically known size
	if !std {
		for id, l := range r.Locals {
			if l == nil || l.ByRef || l.Node != id || l.Ty == nil || !l.Ty.IsUnsized() {
				continue
			}
			t.ErrorCode(l.Span, "E0277", "the size for values of type `%s` cannot be known at compilation time", l.Ty)
		}
	}
}

func isDropMethod(d *Def) bool {
	return d != nil && d.Name == "drop" && d.Parent != nil && d.Parent.Kind == DefTrait && d.Parent.LangItem == "drop"
}

func castable(t Ty) bool {
	return t != nil && !t.HasInfer() && !t.HasError() && !t.HasParam() && !t.HasProj() && !t.HasOpaque()
}

// checkCast reports `as` casts between types that cannot be cast.
func (t *Tcx) checkCast(r *Results, x *ast.CastExpr) {
	from := r.AdjustedType(x.X)
	to := r.NodeTypes[x.ID]
	if !castable(from) || !castable(to) || from == to {
		return
	}
	isNum := func(t Ty) bool { return t.Kind == TInt || t.Kind == TFloat }
	switch {
	case to.Kind == TBool && (isNum(from) || from.Kind == TChar):
		t.ErrorCode(x.Span, "E0054", "cannot cast `%s` as `bool`", from)
	case to.Kind == TChar && from.Kind == TInt && from.IntKind() != U8:
		t.ErrorCode(x.Span, "E0604", "only `u8` can be cast as `char`, not `%s`", from)
	case to.Kind == TChar && from.Kind == TFloat:
		t.ErrorCode(x.Span, "E0604", "only `u8` can be cast as `char`, not `%s`", from)
	case from.Kind == TRef && (isNum(to) || to.Kind == TBool || to.Kind == TChar):
		t.ErrorCode(x.Span, "E0606", "casting `%s` as `%s` is invalid", from, to)
	case (isNum(to) || to.Kind == TBool || to.Kind == TChar) &&
		((from.Kind == TAdt && from.Def.Adt != nil && from.Def.Adt.Kind != AdtEnum) || from.Kind == TStr || from.Kind == TTuple || from.Kind == TArray || from.Kind == TSlice):
		t.ErrorCode(x.Span, "E0605", "non-primitive cast: `%s` as `%s`", from, to)
	case to.Kind == TAdt || to.Kind == TTuple || to.Kind == TArray || to.Kind == TStr || to.Kind == TSlice:
		t.ErrorCode(x.Span, "E0605", "non-primitive cast: `%s` as `%s`", from, to)
	}
}

// checkAssignLHS reports assignments to expressions that are not places.
func (t *Tcx) checkAssignLHS(r *Results, l ast.Expr, code string) {
	switch x := unparen(l).(type) {
	case *ast.CallExpr:
		if c := r.Callees[x.ID]; c == nil || c.Kind == CallCtor {
			return
		}
	case *ast.MethodCallExpr, *ast.LitExpr, *ast.BinaryExpr:
	case *ast.UnaryExpr:
		if x.Op == ast.UnDeref {
			return
		}
	default:
		return
	}
	t.ErrorCode(l.GetSpan(), code, "invalid left-hand side of assignment")
}

// checkPatBindings reports identifiers bound twice in one pattern (E0416)
// and or-patterns whose alternatives bind different names (E0408).
func (t *Tcx) checkPatBindings(r *Results, p ast.Pat) {
	if p == nil {
		return
	}
	t.patBinds(r, p)
}

type patBinding struct {
	name string
	sp   ast.Span
}

func (t *Tcx) patBinds(r *Results, p ast.Pat) []patBinding {
	var out []patBinding
	merge := func(bs []patBinding) {
		for _, b := range bs {
			for _, o := range out {
				if o.name == b.name {
					t.ErrorCode(b.sp, "E0416", "identifier `%s` is bound more than once in the same pattern", displayName(b.name))
				}
			}
			out = append(out, b)
		}
	}
	switch x := p.(type) {
	case *ast.IdentPat:
		if r.Paths[x.ID] == nil {
			out = append(out, patBinding{hygKey(x.Name.Name, x.Name.Hyg), x.Name.Span})
		}
		if x.Sub != nil {
			merge(t.patBinds(r, x.Sub))
		}
	case *ast.ParenPat:
		merge(t.patBinds(r, x.Pat))
	case *ast.RefPat:
		merge(t.patBinds(r, x.Pat))
	case *ast.BoxPat:
		merge(t.patBinds(r, x.Pat))
	case *ast.TuplePat:
		for _, e := range x.Elems {
			merge(t.patBinds(r, e))
		}
	case *ast.TupleStructPat:
		for _, e := range x.Elems {
			merge(t.patBinds(r, e))
		}
	case *ast.SlicePat:
		for _, e := range x.Elems {
			merge(t.patBinds(r, e))
		}
	case *ast.StructPat:
		for _, f := range x.Fields {
			merge(t.patBinds(r, f.Pat))
		}
	case *ast.OrPat:
		var alts [][]patBinding
		for _, a := range x.Alts {
			alts = append(alts, t.patBinds(r, a))
		}
		has := func(bs []patBinding, n string) bool {
			for _, b := range bs {
				if b.name == n {
					return true
				}
			}
			return false
		}
		reported := map[string]bool{}
		for _, a := range alts {
			for _, b := range a {
				for _, other := range alts {
					if !has(other, b.name) && !reported[b.name] {
						reported[b.name] = true
						t.ErrorCode(b.sp, "E0408", "variable `%s` is not bound in all patterns", displayName(b.name))
					}
				}
			}
		}
		if len(alts) > 0 {
			out = alts[0]
		}
	}
	return out
}

func displayName(key string) string {
	for i := 0; i < len(key); i++ {
		if key[i] == 0 {
			return key[:i]
		}
	}
	return key
}

// checkCallBounds re-checks bounds of a called generic function that type
// checking accepted before all facts were known: closure kinds (`F: Fn`
// with a closure that is only FnOnce, E0525) and `Send` bounds on values
// containing `Rc` (E0277).
func (t *Tcx) checkCallBounds(r *Results, c *Callee, sp ast.Span) {
	if c == nil || c.Kind != CallFn || c.Def == nil || c.Def.Kind != DefFn {
		return
	}
	g := t.Generics(c.Def)
	if len(c.Args) != len(g.Params) {
		return
	}
	for _, p := range g.Predicates {
		if p.Trait == nil || p.Maybe || len(p.Trait.Args) == 0 {
			continue
		}
		self := Subst(p.Trait.Args[0], c.Args)
		if !castable(self) {
			continue
		}
		tr := p.Trait.Trait
		switch {
		case tr.LangItem == "fn" || tr.LangItem == "fn_mut":
			if self.Kind != TClosure {
				continue
			}
			ci := t.Closures[self]
			if ci == nil || ci.Expr == nil {
				continue
			}
			need := 0
			if tr.LangItem == "fn_mut" {
				need = 1
			}
			if ci.Kind == 2 && ci.MovesValue {
				kinds := []string{"Fn", "FnMut", "FnOnce"}
				t.ErrorCode(ci.Expr.Span, "E0525", "expected a closure that implements the `%s` trait, but this closure only implements `%s`", kinds[need], kinds[ci.Kind])
			}
		case tr.Trait != nil && tr.Trait.Auto && t.isStd(tr) && tr.Name == "Send":
			if bad := t.notSend(self, tr, map[Ty]bool{}, 0); bad != nil {
				t.ErrorCode(sp, "E0277", "`%s` cannot be sent between threads safely", bad)
			}
		}
	}
}

// notSend returns a type inside t that is certainly not Send (std's Rc and
// rc::Weak, found through tuples, arrays, ADT fields and type arguments,
// references and closure captures), or nil. ADTs with an explicit Send impl
// are taken as Send.
func (t *Tcx) notSend(ty Ty, send *Def, seen map[Ty]bool, depth int) Ty {
	if ty == nil || seen[ty] || depth > 12 {
		return nil
	}
	seen[ty] = true
	switch ty.Kind {
	case TRef, TSlice, TArray:
		return t.notSend(ty.Elem, send, seen, depth+1)
	case TTuple:
		for _, e := range ty.Elems {
			if b := t.notSend(e, send, seen, depth+1); b != nil {
				return b
			}
		}
	case TClosure:
		ci := t.Closures[ty]
		if ci == nil {
			return nil
		}
		for _, cp := range ci.Captures {
			if cp.Ty != nil && castable(cp.Ty) {
				if b := t.notSend(cp.Ty, send, seen, depth+1); b != nil {
					return b
				}
			}
		}
	case TAdt:
		d := ty.Def
		if t.isStd(d) && (d.Name == "Rc" || d.Name == "Weak") && d.Parent != nil && d.Parent.Name == "rc" {
			return ty
		}
		{
			for _, impl := range t.traitImpls[send] {
				if impl.Impl.TraitRef != nil && len(impl.Impl.TraitRef.Args) > 0 {
					if s := impl.Impl.TraitRef.Args[0]; s.Kind == TAdt && s.Def == d {
						return nil
					}
				}
			}
		}
		for _, a := range ty.Args {
			if b := t.notSend(a, send, seen, depth+1); b != nil {
				return b
			}
		}
		if !t.isStd(d) && d.Adt != nil {
			t.AdtFields(d)
			for _, v := range d.Adt.Variants {
				for _, f := range v.Fields {
					ft := Subst(f.Ty, ty.Args)
					if castable(ft) {
						if b := t.notSend(ft, send, seen, depth+1); b != nil {
							return b
						}
					}
				}
			}
		}
	}
	return nil
}
