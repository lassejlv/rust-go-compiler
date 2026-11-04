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
