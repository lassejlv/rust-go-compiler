package typeck

// The deny-by-default `dangerous_implicit_autorefs` lint (rustc_lint's
// autorefs.rs): an implicit reference taken to the dereference of a raw
// pointer, by an overloaded index (`(*vec_ptr)[i]`, `&mut (*p)[..]`), a
// field access through an overloaded deref, or a call of one of the std
// methods core marks `#[rustc_no_implicit_autorefs]` (`(*slice_ptr).len()`).
// Explicit references (`(&*p).len()`, `&(*p)`) are fine.

import (
	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

func (t *Tcx) checkImplicitAutorefs(r *Results, def *Def, root any) {
	if lintLevelAllowed(def, "dangerous_implicit_autorefs") {
		return
	}
	ast.Walk(root, func(x any) bool {
		switch e := x.(type) {
		case *ast.ItemStmt:
			return false
		case *ast.IndexExpr:
			// an overloaded index borrows its base; a builtin one only
			// through the base's adjustments (an overloaded deref)
			if c := r.Callees[e.ID]; !r.IndexBuiltin[e.ID] && c != nil {
				t.lintAutoref(r, e.Span, e.X, autorefIndex)
			} else {
				t.lintAutoref(r, e.Span, e.X, autorefNone)
			}
		case *ast.FieldExpr:
			t.lintAutoref(r, e.Span, e.X, autorefNone)
		case *ast.MethodCallExpr:
			if c := r.Callees[e.ID]; c != nil && t.noImplicitAutorefs(c.Def) {
				t.lintAutoref(r, e.Span, e.Receiver, autorefMethod)
			}
		}
		return true
	})
}

// How the expression using a base borrows it implicitly (lintAutoref).
const (
	autorefNone   = iota // only through the base's own adjustments
	autorefIndex         // an overloaded index borrows the base
	autorefMethod        // a `&self` method: a reference receiver is reborrowed
)

// lintAutoref reports base when it is (a place projection of) the
// dereference of a raw pointer and its adjustments end in one implicit
// borrow (after any number of derefs): an autoref, an overloaded deref,
// or the implicit borrow of an overloaded index.
func (t *Tcx) lintAutoref(r *Results, sp ast.Span, base ast.Expr, mode int) {
	adjs := r.Adjust[base.GetID()]
	for p, ok := base.(*ast.ParenExpr); ok && len(adjs) == 0; p, ok = p.X.(*ast.ParenExpr) {
		adjs = r.Adjust[p.X.GetID()]
	}
	kinds := make([]AdjustKind, 0, len(adjs)+2)
	for _, a := range adjs {
		kinds = append(kinds, a.Kind)
	}
	switch mode {
	case autorefIndex:
		kinds = append(kinds, AdjBorrow)
	case autorefMethod:
		// rustgo passes a reference receiver of a `&self` method as is
		// where rustc reborrows it (deref, then autoref)
		last := len(kinds) - 1
		if rt := r.AdjustedType(base); rt != nil && rt.Kind == TRef && (last < 0 || kinds[last] == AdjDeref || kinds[last] == AdjDerefOverloaded) {
			kinds = append(kinds, AdjDeref, AdjBorrow)
		}
	}
	for len(kinds) > 1 && (kinds[0] == AdjDeref || kinds[0] == AdjDerefOverloaded) {
		kinds = kinds[1:]
	}
	if len(kinds) != 1 || (kinds[0] != AdjBorrow && kinds[0] != AdjDerefOverloaded) {
		return
	}
	// place projections down to the deref
	e := base
	for {
		switch x := e.(type) {
		case *ast.ParenExpr:
			e = x.X
			continue
		case *ast.IndexExpr:
			e = x.X
			continue
		case *ast.FieldExpr:
			e = x.X
			continue
		}
		break
	}
	u, ok := e.(*ast.UnaryExpr)
	if !ok || u.Op != ast.UnDeref {
		return
	}
	if pt := r.NodeTypes[u.X.GetID()]; pt == nil || pt.Kind != TPtr {
		return
	}
	t.Errorf(sp, "implicit autoref creates a reference to the dereference of a raw pointer")
}

// noImplicitAutorefs reports the std methods marked
// `#[rustc_no_implicit_autorefs]` in core and alloc: `len`/`is_empty` of
// slices, str and String, the slice getters and `Index::index` /
// `IndexMut::index_mut`.
func (t *Tcx) noImplicitAutorefs(d *Def) bool {
	if d == nil || d.Parent == nil || !t.isStd(d) {
		return false
	}
	p := d.Parent
	switch d.Name {
	case "index", "index_mut":
		if p.Kind == DefImpl && p.Impl != nil && p.Impl.TraitRef != nil {
			p = p.Impl.TraitRef.Trait
		}
		return p.LangItem == "index" || p.LangItem == "index_mut"
	case "len", "is_empty", "get", "get_mut", "get_unchecked", "get_unchecked_mut":
	default:
		return false
	}
	if p.Kind != DefImpl || p.Impl == nil || p.Impl.TraitRef != nil || p.Impl.SelfTy == nil {
		return false
	}
	switch st := p.Impl.SelfTy; st.Kind {
	case TSlice:
		return true
	case TStr:
		return d.Name == "len" || d.Name == "is_empty"
	case TAdt:
		return st.Def.Name == "String" && (d.Name == "len" || d.Name == "is_empty")
	}
	return false
}

// fnPtrBlanket reports a blanket impl over fn pointer types only (std's
// `impl<F: FnPtr> Trait for F`): it never applies to a type parameter.
func (t *Tcx) fnPtrBlanket(impl *Def) bool {
	self := impl.Impl.TraitRef.Args[0]
	for _, p := range t.Generics(impl).Predicates {
		if p.Trait != nil && !p.Maybe && p.Trait.Trait.LangItem == "fn_ptr_trait" && len(p.Trait.Args) > 0 && p.Trait.Args[0] == self {
			return true
		}
	}
	return false
}
