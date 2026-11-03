package typeck

// Item-level checks of impls (mirrors parts of rustc_hir_analysis):
// trait impls must implement exactly the trait's items with matching
// signatures (E0046, E0407, E0437, E0438, E0050, E0053, E0185, E0186),
// inherent impls of one type must not define an item twice (E0592), trait
// impls must respect the orphan rule (E0117) and must not overlap (E0119).
//
// Impls in std and impls of std traits are not checked: rustgo's std does
// not mirror the real std's traits item by item.

import (
	"fmt"
	"strings"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

func (t *Tcx) checkImpls() {
	for _, impl := range t.R.Impls {
		if t.isStd(impl) || impl.Impl == nil || impl.Impl.Ast == nil {
			continue
		}
		if impl.Impl.TraitRef != nil {
			if !impl.Impl.Negative {
				t.checkTraitImplItems(impl)
			}
			t.checkOrphan(impl)
		}
	}
	t.checkInherentDups()
	t.checkLifetimes()
	// signatures of all functions are checked, even of unused ones
	for _, d := range t.R.Defs {
		if d.Kind == DefFn && d.Fn != nil && d.Fn.Ast != nil && !t.isStd(d) && d.Intrinsic == "" {
			t.FnSig(d)
		}
	}
	// constants and statics are checked even when unused (their
	// evaluation errors and non-const calls are errors)
	for _, d := range t.R.Defs {
		if (d.Kind != DefConst && d.Kind != DefStatic) || t.isStd(d) || d.Const == nil || d.Const.Expr == nil || d.IsAssoc() || d.Name == "_" {
			continue
		}
		if t.Generics(d).Count() != 0 {
			continue
		}
		t.CheckInstance(d, nil)
	}
}

// fnHasSelfSized reports whether a trait item has `where Self: Sized`.
func whereSelfSized(g *ast.Generics) bool {
	if g == nil {
		return false
	}
	for _, w := range g.Where {
		pt, ok := w.Ty.(*ast.PathType)
		if !ok || pt.QSelf != nil || len(pt.Path.Segments) != 1 || pt.Path.Segments[0].Ident.Name != "Self" {
			continue
		}
		for _, b := range w.Bounds {
			if b.Path != nil && !b.Maybe && len(b.Path.Segments) > 0 && b.Path.Segments[len(b.Path.Segments)-1].Ident.Name == "Sized" {
				return true
			}
		}
	}
	return false
}

func itemKindWord(d *Def) string {
	switch d.Kind {
	case DefAssocTy:
		return "type"
	case DefConst:
		return "const"
	}
	return "method"
}

func (t *Tcx) checkTraitImplItems(impl *Def) {
	tr := impl.Impl.TraitRef.Trait
	if tr == nil || tr.Trait == nil || t.isStd(tr) {
		return
	}
	for _, it := range impl.Impl.Order {
		ti := tr.Trait.Items[it.Name]
		if ti == nil || ti.Kind != it.Kind {
			code := "E0407"
			switch it.Kind {
			case DefAssocTy:
				code = "E0437"
			case DefConst:
				code = "E0438"
			}
			t.ErrorCode(it.Item.Name.Span, code, "%s `%s` is not a member of trait `%s`", itemKindWord(it), it.Name, tr.Name)
			continue
		}
		if it.Kind == DefFn {
			t.compareImplMethod(impl, it, ti)
		}
	}
	var missing []string
	for _, ti := range tr.Trait.Order {
		if _, ok := impl.Impl.Items[ti.Name]; ok {
			continue
		}
		required := false
		switch ti.Kind {
		case DefFn:
			required = ti.Fn.Ast.Body == nil
		case DefConst:
			required = ti.Const.Expr == nil
		case DefAssocTy:
			required = ti.AssocTy.TyAst == nil
		}
		if required {
			missing = append(missing, "`"+ti.Name+"`")
		}
	}
	if len(missing) > 0 {
		sp := impl.Span
		t.ErrorCode(sp, "E0046", "not all trait items implemented, missing: %s", strings.Join(missing, ", "))
	}
}

func selfWord(sp *ast.SelfParam) string {
	switch sp.Kind {
	case ast.SelfRef:
		return "&self"
	case ast.SelfRefMut:
		return "&mut self"
	}
	return "self"
}

// compareImplMethod compares an impl method's signature with the trait's.
func (t *Tcx) compareImplMethod(impl, im, tm *Def) {
	idecl, tdecl := im.Fn.Ast.Decl, tm.Fn.Ast.Decl
	switch {
	case idecl.SelfParam != nil && tdecl.SelfParam == nil:
		t.ErrorCode(im.Item.Name.Span, "E0185", "method `%s` has a `%s` declaration in the impl, but not in the trait", im.Name, selfWord(idecl.SelfParam))
		return
	case idecl.SelfParam == nil && tdecl.SelfParam != nil:
		t.ErrorCode(im.Item.Name.Span, "E0186", "method `%s` has a `%s` declaration in the trait, but not in the impl", im.Name, selfWord(tdecl.SelfParam))
		return
	}
	if len(idecl.Params) != len(tdecl.Params) {
		n := len(idecl.Params)
		if idecl.SelfParam != nil {
			n++
		}
		m := len(tdecl.Params)
		if tdecl.SelfParam != nil {
			m++
		}
		t.ErrorCode(im.Item.Name.Span, "E0050", "method `%s` has %d parameter%s but the declaration in trait `%s::%s` has %d", im.Name, n, plural(n), impl.Impl.TraitRef.Trait.Name, tm.Name, m)
		return
	}
	// types: lowered quietly; anything not fully comparable is skipped
	ig, tg := t.Generics(im), t.Generics(tm)
	iown, town := ig.Own(), tg.Own()
	if len(iown) != len(town) {
		return
	}
	for i := range iown {
		if iown[i].Synthetic || town[i].Synthetic || iown[i].Const != town[i].Const {
			return
		}
	}
	trArgs := impl.Impl.TraitRef.Args
	if len(trArgs) != tg.OwnStart {
		return
	}
	var isig, tsig *FnSig
	ok := t.quietly(func() {
		isig = t.FnSig(im)
		tsig = t.FnSig(tm)
	})
	if !ok || isig == nil || tsig == nil || len(isig.Inputs) != len(tsig.Inputs) {
		return
	}
	args := append([]Ty{}, trArgs...)
	ident := ig.Identity()
	args = append(args, ident[ig.OwnStart:]...)
	same := func(a, b Ty) (equal, comparable bool) {
		var x, y Ty
		if !t.quietly(func() {
			x = t.Normalize(Subst(a, args))
			y = t.Normalize(b)
		}) {
			return false, false
		}
		if x == y {
			return true, true
		}
		for _, z := range []Ty{x, y} {
			if z == nil || z.HasProj() || z.HasOpaque() || z.HasError() || z.HasInfer() {
				return false, false
			}
		}
		return false, true
	}
	for i := range tsig.Inputs {
		if eq, cmp := same(tsig.Inputs[i], isig.Inputs[i]); cmp && !eq {
			t.ErrorCode(im.Item.Name.Span, "E0053", "method `%s` has an incompatible type for trait", im.Name)
			return
		}
	}
	if eq, cmp := same(tsig.Output, isig.Output); cmp && !eq {
		t.ErrorCode(im.Item.Name.Span, "E0053", "method `%s` has an incompatible type for trait", im.Name)
	}
}

// quietly runs f with error reporting suppressed; false if it reported
// errors or aborted.
func (t *Tcx) quietly(f func()) (ok bool) {
	before := t.quietErrs
	t.quiet++
	defer func() {
		t.quiet--
		if r := recover(); r != nil {
			ok = false
			return
		}
		ok = t.quietErrs == before
	}()
	f()
	return true
}

// isLocalTy reports whether ty counts as local to crate c for the orphan
// rule (local ADTs and dyn local traits, also behind the fundamental
// &, &mut, Box and Pin).
func (t *Tcx) isLocalTy(ty Ty, c *Crate) bool {
	switch ty.Kind {
	case TAdt:
		if ty.Def.Crate == c {
			return true
		}
		if (ty.Def.LangItem == "owned_box" || ty.Def.Name == "Pin") && len(ty.Args) > 0 {
			return t.isLocalTy(ty.Args[0], c)
		}
	case TRef:
		return t.isLocalTy(ty.Elem, c)
	case TDyn:
		return ty.Def != nil && ty.Def.Crate == c
	case TClosure, TParam, TInfer, TError, TProjection, TOpaque:
		return true // not modelled: assume fine
	}
	return false
}

func (t *Tcx) checkOrphan(impl *Def) {
	tr := impl.Impl.TraitRef
	if tr.Trait.Crate == impl.Crate || t.Generics(impl).Count() != 0 {
		return
	}
	for _, a := range tr.Args {
		if a == nil || a.HasParam() || a.HasError() || t.isLocalTy(a, impl.Crate) {
			return
		}
	}
	t.ErrorCode(impl.Span, "E0117", "only traits defined in the current crate can be implemented for types defined outside of the crate")
}

// checkInherentDups reports items defined twice for the same type by
// non-generic inherent impls.
func (t *Tcx) checkInherentDups() {
	for _, impls := range t.inherent {
		seen := map[string]*Def{}
		for _, impl := range impls {
			if t.isStd(impl) || t.Generics(impl).Count() != 0 || impl.Impl.SelfTy == nil {
				continue
			}
			for _, it := range impl.Impl.Order {
				key := fmt.Sprintf("%p/%d/%s", impl.Impl.SelfTy, it.Kind, it.Name)
				if it.Kind == DefAssocTy {
					continue
				}
				if prev := seen[key]; prev != nil && prev.Parent != impl {
					t.ErrorCode(it.Item.Name.Span, "E0592", "duplicate definitions with name `%s`", it.Name)
					continue
				}
				seen[key] = it
			}
		}
	}
}

// blanketOverlap reports whether generic impl a (without where clauses)
// applies to every trait ref of impl b.
func (t *Tcx) blanketCovers(a, b *Def) bool {
	ga := t.Generics(a)
	for _, p := range ga.Predicates {
		if p.Maybe || (p.Trait != nil && p.Trait.Trait.LangItem == "sized") {
			continue
		}
		return false
	}
	for _, p := range ga.Params {
		if p.Const {
			return false
		}
	}
	binds := map[int]Ty{}
	var match func(x, y Ty) bool
	match = func(x, y Ty) bool {
		if x.Kind == TParam && x.Index < len(ga.Params) && ga.Params[x.Index].Name == x.Name {
			if prev, ok := binds[x.Index]; ok {
				return prev == y
			}
			if y.IsUnsized() || y.Kind == TParam {
				return false // implicit Sized bound / generic: be conservative
			}
			binds[x.Index] = y
			return true
		}
		if x.HasParam() || x.HasProj() || y.HasProj() || y.HasError() {
			if x.Kind != y.Kind {
				return false
			}
			switch x.Kind {
			case TRef, TPtr, TSlice:
				return x.Mut == y.Mut && match(x.Elem, y.Elem)
			case TAdt:
				if x.Def != y.Def || len(x.Args) != len(y.Args) {
					return false
				}
				for i := range x.Args {
					if !match(x.Args[i], y.Args[i]) {
						return false
					}
				}
				return true
			case TTuple:
				if len(x.Elems) != len(y.Elems) {
					return false
				}
				for i := range x.Elems {
					if !match(x.Elems[i], y.Elems[i]) {
						return false
					}
				}
				return true
			}
			return false
		}
		return x == y
	}
	ar, br := a.Impl.TraitRef.Args, b.Impl.TraitRef.Args
	if len(ar) != len(br) {
		return false
	}
	for i := range ar {
		if !match(ar[i], br[i]) {
			return false
		}
	}
	return true
}
