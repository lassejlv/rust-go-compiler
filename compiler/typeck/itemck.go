package typeck

// Well-formedness checks of struct, enum and union definitions:
// recursive types without indirection (E0072), duplicate field names
// (E0124), unsized fields that are not the last field of a struct (E0277),
// duplicate enum discriminants (E0081), `repr` on zero-variant enums
// (E0084) and explicit discriminants on enums with fields without an
// integer/C representation (E0732).

import (
	"strconv"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
)

func (t *Tcx) checkAdtDefs() {
	for _, d := range t.R.Defs {
		if t.isStd(d) || d.Item == nil || d.Adt == nil {
			continue
		}
		if d.Kind != DefStruct && d.Kind != DefEnum && d.Kind != DefUnion {
			continue
		}
		t.AdtFields(d)
		t.checkFieldNames(d)
		t.checkUnsizedFields(d)
		if d.Kind == DefEnum {
			t.checkEnumDiscs(d)
		}
		if t.adtContainsSelf(d) {
			t.ErrorCode(d.Item.Name.Span, "E0072", "recursive type `%s` has infinite size", d.Name)
		}
	}
}

// adtContainsSelf reports whether the fields of ADT def d contain d itself
// by value (not behind a pointer).
func (t *Tcx) adtContainsSelf(d *Def) bool {
	seen := map[Ty]bool{}
	for _, v := range d.Adt.Variants {
		for _, f := range v.Fields {
			if t.tyContainsAdt(d, f.Ty, seen, 0) {
				return true
			}
		}
	}
	return false
}

func (t *Tcx) tyContainsAdt(root *Def, ty Ty, seen map[Ty]bool, depth int) bool {
	if ty == nil || depth > 64 {
		return false
	}
	switch ty.Kind {
	case TTuple:
		for _, e := range ty.Elems {
			if t.tyContainsAdt(root, e, seen, depth+1) {
				return true
			}
		}
	case TArray:
		return t.tyContainsAdt(root, ty.Elem, seen, depth+1)
	case TAdt:
		if ty.Def == root {
			return true
		}
		if seen[ty] || ty.Def.Adt == nil || ty.Def.Item == nil {
			return false
		}
		seen[ty] = true
		t.AdtFields(ty.Def)
		for _, v := range ty.Def.Adt.Variants {
			for _, f := range v.Fields {
				if t.tyContainsAdt(root, Subst(f.Ty, ty.Args), seen, depth+1) {
					return true
				}
			}
		}
	}
	return false
}

// checkFieldNames reports a field declared twice (E0124).
func (t *Tcx) checkFieldNames(d *Def) {
	check := func(fields []*ast.FieldDef) {
		seen := map[string]bool{}
		for _, f := range fields {
			n := f.Name.Name
			if n == "" || n == "_" {
				continue
			}
			if seen[n] {
				t.ErrorCode(f.Name.Span, "E0124", "field `%s` is already declared", n)
			}
			seen[n] = true
		}
	}
	switch k := d.Item.Kind.(type) {
	case *ast.StructItem:
		check(k.Fields)
	case *ast.UnionItem:
		check(k.Fields)
	case *ast.EnumItem:
		for _, v := range k.Variants {
			check(v.Fields)
		}
	}
}

// checkUnsizedFields reports dynamically sized field types anywhere but in
// the last field of a struct (E0277).
func (t *Tcx) checkUnsizedFields(d *Def) {
	for _, v := range d.Adt.Variants {
		for i, f := range v.Fields {
			if f.Ty == nil || !f.Ty.IsUnsized() || f.Ast == nil {
				continue
			}
			if d.Kind == DefStruct && i == len(v.Fields)-1 {
				continue
			}
			t.ErrorCode(f.Ast.Ty.GetSpan(), "E0277", "the size for values of type `%s` cannot be known at compilation time", f.Ty)
		}
	}
}

func enumReprs(d *Def) (intRepr, cRepr bool) {
	for _, a := range d.Attrs {
		if a.Path != "repr" || len(a.Tokens) == 0 || a.Tokens[0].Delim != '(' {
			continue
		}
		for _, tt := range a.Tokens[0].Inner {
			if tt.Delim != 0 {
				continue
			}
			if _, ok := IntTyByName(tt.Tok.Text); ok {
				intRepr = true
			}
			if tt.Tok.Text == "C" {
				cRepr = true
			}
		}
	}
	return
}

// checkEnumDiscs checks the discriminants and representation of an enum.
func (t *Tcx) checkEnumDiscs(d *Def) {
	k, ok := d.Item.Kind.(*ast.EnumItem)
	if !ok {
		return
	}
	intRepr, cRepr := enumReprs(d)
	if len(k.Variants) == 0 {
		if intRepr || cRepr {
			t.ErrorCode(d.Item.Name.Span, "E0084", "unsupported representation for zero-variant enum")
		}
		return
	}
	explicit, nonUnit := false, false
	for _, v := range k.Variants {
		if v.Disc != nil {
			explicit = true
		}
		if v.Shape != ast.ShapeUnit {
			nonUnit = true
		}
	}
	if explicit && nonUnit && !intRepr {
		t.ErrorCode(d.Item.Name.Span, "E0732", "`#[repr(inttype)]` must be specified for enums with explicit discriminants and non-unit variants")
		return
	}
	seen := map[int64]string{}
	for i, v := range d.Adt.Variants {
		if i >= len(k.Variants) {
			break
		}
		if prev, dup := seen[v.Disc]; dup {
			t.ErrorCode(k.Variants[i].Name.Span, "E0081", "discriminant value `%s` assigned more than once (`%s` and `%s`)", strconv.FormatInt(v.Disc, 10), prev, v.Name)
			continue
		}
		seen[v.Disc] = v.Name
	}
}

type pendingBinding struct {
	trait *TraitRef
	name  string
	sp    lexer.Span
}

// checkBindingNames reports associated type bindings in trait bounds whose
// name is not an associated type of the trait or its supertraits (E0220).
func (t *Tcx) checkBindingNames() {
	pend := t.pendingBindings
	t.pendingBindings = nil
	for _, pb := range pend {
		if pb.trait == nil || pb.trait.Trait == nil || pb.trait.Trait.Trait == nil {
			continue
		}
		found, ok := false, true
		ok = t.quietly(func() {
			for _, st := range t.AllSupertraits(pb.trait) {
				if st.Trait == nil || st.Trait.Trait == nil {
					found = true // unknown: be quiet
					return
				}
				if it := st.Trait.Trait.Items[pb.name]; it != nil && it.Kind == DefAssocTy {
					found = true
					return
				}
				if t.isStd(st.Trait) && !stdAssocComplete[st.Trait.Name] {
					// rustgo's std may not declare every associated type
					found = true
					return
				}
			}
		})
		if ok && !found {
			t.ErrorCode(pb.sp, "E0220", "associated type `%s` not found for `%s`", pb.name, pb.trait.Trait.Name)
		}
	}
}

// stdAssocComplete lists std traits whose associated types rustgo's std
// declares like the real std.
var stdAssocComplete = map[string]bool{
	"Iterator": true, "IntoIterator": true, "DoubleEndedIterator": true, "ExactSizeIterator": true,
	"Deref": true, "DerefMut": true, "Add": true, "Sub": true, "Mul": true, "Div": true, "Rem": true,
	"Neg": true, "Not": true, "FromStr": true, "TryFrom": true, "TryInto": true, "Future": true,
	"Index": true, "IndexMut": true, "FnOnce": true, "FnMut": true, "Fn": true,
	"Clone": true, "Copy": true, "Debug": true, "Display": true, "PartialEq": true, "Eq": true,
	"PartialOrd": true, "Ord": true, "Hash": true, "Default": true, "From": true, "Into": true,
	"AsRef": true, "AsMut": true, "Send": true, "Sync": true, "Sized": true,
}

// checkImplHeaderLifetimes reports paths in an impl header (the self type
// and the trait) to types or traits with lifetime parameters that give no
// lifetime arguments (E0726; `impl Trait for S` with `struct S<'a>` needs
// `S<'_>`). Fn pointer types and `Fn(..)` sugar have their own elision
// scopes and are not searched.
func (t *Tcx) checkImplHeaderLifetimes(impl *Def) {
	ia := impl.Impl.Ast
	if ia == nil || impl.Crate == nil || !impl.Crate.Local || impl.Crate.Is2015() || impl.HasAttr("automatically_derived") {
		return
	}
	ac := t.itemConv(impl)
	check := func(p *ast.Path) {
		if p == nil || len(p.Segments) == 0 {
			return
		}
		pr, err := t.R.ResolvePath(ac.scope, pathSegs(p), p.Global, TypeNS, ac.firstLookup)
		if err != nil || pr.Consumed != len(p.Segments) || pr.Res.Kind != ResDef || pr.Res.Def == nil {
			return
		}
		g := genericsAst(pr.Res.Def)
		if g == nil {
			return
		}
		n := 0
		for _, gp := range g.Params {
			if gp.Kind == ast.GPLifetime {
				n++
			}
		}
		if n == 0 {
			return
		}
		seg := p.Segments[len(p.Segments)-1]
		if seg.Args != nil {
			if seg.Args.Parenthesized {
				return
			}
			for _, a := range seg.Args.Args {
				if a.Lifetime != "" {
					return
				}
			}
		}
		t.ErrorCode(p.Span, "E0726", "implicit elided lifetime not allowed here")
	}
	walk := func(n any) {
		ast.Walk(n, func(x any) bool {
			switch x := x.(type) {
			case *ast.FnPtrType:
				return false
			case *ast.Path:
				for _, s := range x.Segments {
					if s.Args != nil && s.Args.Parenthesized {
						return false
					}
				}
			case *ast.PathType:
				if x.QSelf == nil {
					check(x.Path)
				}
				for _, s := range x.Path.Segments {
					if s.Args != nil && s.Args.Parenthesized {
						return false
					}
				}
			}
			return true
		})
	}
	if ia.SelfTy != nil {
		walk(ia.SelfTy)
	}
	if ia.Trait != nil {
		check(ia.Trait)
		for _, s := range ia.Trait.Segments {
			if s.Args != nil && !s.Args.Parenthesized {
				for _, a := range s.Args.Args {
					if a.Type != nil {
						walk(a.Type)
					}
				}
			}
		}
	}
}

// adtBoundViolation finds, in a fully known type, an ADT of a non-std
// crate whose where-clauses do not hold for its arguments (`W<u8>` with
// `struct W<X: Trait>` and no `impl Trait for u8`): the type is not
// well-formed (E0277). It returns the unsatisfied trait ref, or nil.
func (t *Tcx) adtBoundViolation(ty Ty) *TraitRef {
	if ty == nil || ty.HasParam() || ty.HasInfer() || ty.HasProj() || ty.HasOpaque() || ty.HasError() {
		return nil
	}
	var bad *TraitRef
	seen := map[Ty]bool{}
	Walk(ty, func(x Ty) bool {
		if bad != nil {
			return false
		}
		if x.Kind != TAdt || seen[x] || t.isStd(x.Def) || len(x.Args) == 0 {
			return true
		}
		seen[x] = true
		g := t.Generics(x.Def)
		if g == nil || len(g.Params) != len(x.Args) {
			return true
		}
		for _, p := range g.Predicates {
			if p.Maybe || p.Trait == nil || p.Trait.Trait == nil || len(p.Bindings) > 0 {
				continue
			}
			ref := p.Trait.Subst(x.Args)
			ok := true
			for _, a := range ref.Args {
				if a == nil || a.HasParam() || a.HasInfer() || a.HasProj() || a.HasOpaque() || a.HasError() {
					ok = false
				}
			}
			if !ok {
				continue
			}
			if newInferCtxt(t).evaluate(&Predicate{Trait: ref}, 0) == SelNone {
				bad = ref
				return false
			}
		}
		return true
	})
	return bad
}

// typeNamesUsed adds to used the identifiers (first path segments and
// generic arguments) and lifetimes a type mentions; macro types make it
// report ok=false (unknown).
func typeNamesUsed(ty ast.Type, used map[string]bool) (ok bool) {
	ok = true
	ast.Walk(ty, func(n any) bool {
		switch x := n.(type) {
		case *ast.MacroType:
			ok = false
			return false
		case *ast.RefType:
			if x.Lifetime != "" {
				used[x.Lifetime] = true
			}
		case *ast.DynTraitType:
			for _, b := range x.Bounds {
				if b.Lifetime != "" {
					used[b.Lifetime] = true
				}
			}
		case *ast.ImplTraitType:
			for _, b := range x.Bounds {
				if b.Lifetime != "" {
					used[b.Lifetime] = true
				}
			}
		case *ast.Path:
			for _, s := range x.Segments {
				used[s.Ident.Name] = true
				if s.Args != nil {
					for _, a := range s.Args.Args {
						if a.Lifetime != "" {
							used[a.Lifetime] = true
						}
					}
				}
			}
		}
		return true
	})
	return ok
}

// checkUnusedParams reports type and lifetime parameters of a struct,
// enum or union that no field mentions (E0392), and of a type alias that
// its type does not mention (E0091).
func (t *Tcx) checkUnusedParams(d *Def) {
	if d.Item == nil || d.Crate == nil || !d.Crate.Local {
		return
	}
	g := genericsAst(d)
	if g == nil || len(g.Params) == 0 {
		return
	}
	used := map[string]bool{}
	var types []ast.Type
	code, what := "E0392", ""
	switch k := d.Item.Kind.(type) {
	case *ast.StructItem:
		for _, f := range k.Fields {
			types = append(types, f.Ty)
		}
		what = "struct"
	case *ast.UnionItem:
		for _, f := range k.Fields {
			types = append(types, f.Ty)
		}
		what = "union"
	case *ast.EnumItem:
		for _, v := range k.Variants {
			for _, f := range v.Fields {
				types = append(types, f.Ty)
			}
		}
		what = "enum"
	case *ast.TypeAliasItem:
		if k.Ty == nil || d.IsAssoc() {
			return
		}
		types = append(types, k.Ty)
		code, what = "E0091", "type alias"
	default:
		return
	}
	for _, ty := range types {
		if !typeNamesUsed(ty, used) {
			return
		}
	}
	// parameters in associated type bindings of the bounds are
	// constrained by them (`I: Iterator<Item = &'a T>`)
	var bounds []*ast.Bound
	for _, p := range g.Params {
		bounds = append(bounds, p.Bounds...)
	}
	for _, w := range g.Where {
		bounds = append(bounds, w.Bounds...)
	}
	for _, b := range bounds {
		if b.Path == nil {
			continue
		}
		for _, s := range b.Path.Segments {
			if s.Args == nil {
				continue
			}
			for _, ab := range s.Args.Bindings {
				if ab.Ty != nil && !typeNamesUsed(ab.Ty, used) {
					return
				}
			}
			if s.Args.Parenthesized && s.Args.Output != nil {
				// `F: Fn() -> T` binds Output
				if !typeNamesUsed(s.Args.Output, used) {
					return
				}
			}
		}
	}
	for _, p := range g.Params {
		n := p.Name.Name
		if used[n] {
			continue
		}
		switch p.Kind {
		case ast.GPLifetime:
			if code == "E0091" {
				continue // unused lifetimes of aliases are allowed
			}
			t.ErrorCode(p.Name.Span, code, "lifetime parameter `%s` is never used", n)
		case ast.GPType:
			if code == "E0091" {
				t.ErrorCode(p.Name.Span, code, "type parameter `%s` is never used in %s", n, what)
			} else {
				t.ErrorCode(p.Name.Span, code, "type parameter `%s` is never used", n)
			}
		}
	}
}
