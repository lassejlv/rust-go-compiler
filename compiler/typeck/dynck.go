package typeck

// Checks of `dyn Trait` types written outside std: the trait must be dyn
// compatible (E0038) and every associated type must be specified (E0191).

import (
	"sort"
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
)

func (t *Tcx) checkDynTrait(principal *TraitRef, bindings []Binding, sp lexer.Span) {
	bound := map[string]bool{}
	for _, b := range bindings {
		bound[b.Name] = true
	}
	var missing []string
	seen := map[*Def]bool{}
	var visit func(tr *Def)
	visit = func(tr *Def) {
		if tr == nil || seen[tr] || tr.Trait == nil {
			return
		}
		seen[tr] = true
		// bindings in supertrait bounds (`trait A: Iterator<Item = u8>`)
		for _, p := range t.Generics(tr).Predicates {
			if p.Trait != nil && len(p.Trait.Args) > 0 && p.Trait.Args[0].Kind == TParam && p.Trait.Args[0].Index == 0 {
				for _, b := range p.Bindings {
					bound[b.Name] = true
				}
			}
		}
		for _, it := range tr.Trait.Order {
			if it.Kind != DefAssocTy || bound[it.Name] || it.AssocTy == nil || it.AssocTy.TyAst != nil {
				continue
			}
			if ta, ok := it.Item.Kind.(*ast.TypeAliasItem); ok && (whereSelfSized(ta.Generics) || (ta.Generics != nil && len(ta.Generics.Params) > 0)) {
				continue
			}
			missing = append(missing, "`"+it.Name+"`")
			bound[it.Name] = true
		}
		for _, st := range t.Supertraits(tr) {
			visit(st.Trait)
		}
	}
	visit(principal.Trait)
	if len(missing) > 0 {
		sort.Strings(missing)
		what := "type"
		if len(missing) > 1 {
			what = "types"
		}
		t.ErrorCode(sp, "E0191", "the value of the associated %s %s in `%s` must be specified", what, strings.Join(missing, ", "), principal.Trait.Name)
	}
	if !t.isStd(principal.Trait) && !t.dynCompatible(principal.Trait, map[*Def]bool{}) {
		t.ErrorCode(sp, "E0038", "the trait `%s` is not dyn compatible", principal.Trait.Name)
	}
}

// mentionsSelf reports whether a type mentions `Self` other than through
// an associated type projection (`Self::Item`).
func mentionsSelf(ty ast.Type) bool {
	found := false
	ast.Walk(ty, func(n any) bool {
		if found {
			return false
		}
		switch x := n.(type) {
		case *ast.PathType:
			if x.QSelf == nil && len(x.Path.Segments) == 1 && x.Path.Segments[0].Ident.Name == "Self" {
				found = true
				return false
			}
			if x.QSelf == nil && len(x.Path.Segments) > 1 && x.Path.Segments[0].Ident.Name == "Self" {
				return false // Self::Assoc
			}
		case *ast.ImplTraitType:
			return false
		}
		return true
	})
	return found
}

// dynCompatible checks the trait's own items (and those of non-std
// supertraits) for what makes a trait not dyn compatible: associated
// functions without a receiver, generic methods, methods mentioning Self
// outside the receiver, `impl Trait` returns, async methods, associated
// consts and generic associated types (items with `where Self: Sized`
// are exempt), and a `Sized` supertrait.
func (t *Tcx) dynCompatible(tr *Def, seen map[*Def]bool) bool {
	if seen[tr] || tr.Item == nil {
		return true
	}
	seen[tr] = true
	ti, ok := tr.Item.Kind.(*ast.TraitItem)
	if !ok {
		return true
	}
	for _, b := range ti.Supertraits {
		if b.Path != nil && !b.Maybe && len(b.Path.Segments) > 0 && b.Path.Segments[len(b.Path.Segments)-1].Ident.Name == "Sized" {
			return false
		}
	}
	if ti.Generics != nil && whereSelfSized(ti.Generics) {
		return false
	}
	for _, it := range ti.Items {
		switch k := it.Kind.(type) {
		case *ast.ConstItem:
			return false
		case *ast.TypeAliasItem:
			if k.Generics != nil && len(k.Generics.Params) > 0 && !whereSelfSized(k.Generics) {
				return false
			}
		case *ast.FnItem:
			if whereSelfSized(k.Generics) {
				continue
			}
			if k.Decl.SelfParam == nil || k.Async {
				return false
			}
			if k.Generics != nil {
				for _, p := range k.Generics.Params {
					if p.Kind != ast.GPLifetime {
						return false
					}
				}
			}
			for _, p := range k.Decl.Params {
				if mentionsSelf(p.Ty) {
					return false
				}
				if _, ok := p.Ty.(*ast.ImplTraitType); ok {
					return false
				}
			}
			if k.Decl.Output != nil {
				if mentionsSelf(k.Decl.Output) {
					return false
				}
				implTrait := false
				ast.Walk(k.Decl.Output, func(n any) bool {
					if _, ok := n.(*ast.ImplTraitType); ok {
						implTrait = true
					}
					return !implTrait
				})
				if implTrait {
					return false
				}
			}
		}
	}
	for _, st := range t.Supertraits(tr) {
		if len(st.Args) > 1 {
			// a supertrait with `Self` as a type argument
			// (`trait T: PartialEq<Self>`, or `PartialEq`'s default)
			for _, a := range st.Args[1:] {
				if a != nil && mentionsParam(a, Param(0, "Self")) {
					return false
				}
			}
		}
		if st.Trait != nil && t.isStd(st.Trait) && stdSizedTraits[st.Trait.Name] && st.Trait.Trait != nil && t.stdTraitSized(st.Trait) {
			return false // `trait Shape: Clone`
		}
		if st.Trait != nil && !t.isStd(st.Trait) && !t.dynCompatible(st.Trait, seen) {
			return false
		}
	}
	return true
}

// stdSizedTraits are the std traits that have `Sized` as a (transitive)
// supertrait in the real std (rustgo's std is only trusted for these).
var stdSizedTraits = map[string]bool{"Clone": true, "Copy": true, "Default": true, "From": true, "Into": true,
	"TryFrom": true, "TryInto": true, "FromStr": true, "FromIterator": true, "Sum": true, "Product": true}

// stdTraitSized reports whether a std trait has `Sized` among its
// transitive supertraits.
func (t *Tcx) stdTraitSized(tr *Def) bool {
	for _, st := range t.AllSupertraits(&TraitRef{Trait: tr, Args: t.Generics(tr).Identity()}) {
		if st.Trait.LangItem == "sized" {
			return true
		}
	}
	return false
}
