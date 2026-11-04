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
