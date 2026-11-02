package codegen

// Enum layout. Most enums are `struct { tag; union { variants } }`.
// Option-like enums whose payload has a non-null niche (references, Box,
// fn pointers, NonNull, NonZero*) are laid out as the payload alone, with
// the empty variant encoded as all-zero bits — matching Rust's layout, so
// e.g. Option<&T> and Option<extern "C" fn()> are FFI-compatible
// nullable pointers.

import (
	"fmt"
	"strings"

	. "rustgo/compiler/middle"
)

type nicheLayout struct {
	data, empty *VariantDef // variant with the payload / the empty variant
	path        string      // C member path from the payload to the niche scalar
}

// niche returns the niche layout of an enum type, or nil.
func (g *Gen) niche(t Ty) *nicheLayout {
	if t.Kind != TAdt || t.Def.Adt == nil || t.Def.Adt.Kind != AdtEnum {
		return nil
	}
	if n, ok := g.nicheCache[t]; ok {
		return n
	}
	g.nicheCache[t] = nil
	d := t.Def
	if len(d.Adt.Variants) != 2 || d.Adt.Repr != "" || hasReprAttr(d) {
		return nil
	}
	var data, empty *VariantDef
	for vi, v := range d.Adt.Variants {
		n := len(g.fieldTypes(t, vi))
		switch {
		case n == 0:
			empty = v
		case n == 1:
			data = v
		}
	}
	if data == nil || empty == nil {
		return nil
	}
	ft := g.fieldTypes(t, data.Index)[0]
	path, ok := g.nichePath(ft, 0)
	if !ok {
		return nil
	}
	n := &nicheLayout{data: data, empty: empty, path: ".f0" + path}
	g.nicheCache[t] = n
	return n
}

func hasReprAttr(d *Def) bool {
	for _, a := range d.Attrs {
		if a.Path == "repr" {
			return true
		}
	}
	return false
}

// nichePath finds a never-zero scalar inside a type.
func (g *Gen) nichePath(t Ty, depth int) (string, bool) {
	if depth > 4 {
		return "", false
	}
	switch t.Kind {
	case TRef:
		if t.Elem.IsUnsized() {
			return ".ptr", true
		}
		return "", true
	case TFnPtr:
		return "", true
	case TAdt:
		d := t.Def
		if d.Adt.Kind != AdtStruct {
			return "", false
		}
		fts := g.fieldTypes(t, 0)
		if len(fts) != 1 {
			return "", false
		}
		nonnull := d.LangItem == "owned_box" || d.HasAttr("rustc_nonnull_optimization_guaranteed") ||
			(d.Crate != nil && d.Crate.Name == "std" && (d.Name == "NonNull" || strings.HasPrefix(d.Name, "NonZero")))
		if nonnull {
			f := fts[0]
			switch {
			case f.Kind == TPtr && f.Elem.IsUnsized():
				return ".f0.ptr", true
			case f.Kind == TPtr || f.Kind == TInt:
				return ".f0", true
			}
			if p, ok := g.nichePath(f, depth+1); ok {
				return ".f0" + p, true
			}
			return "", false
		}
		// transparent newtype around a niche-carrying type
		if p, ok := g.nichePath(fts[0], depth+1); ok {
			return ".f0" + p, true
		}
	}
	return "", false
}

// enumTag returns a C expression for the discriminant of an enum place.
func (g *Gen) enumTag(t Ty, place string) string {
	if n := g.niche(t); n != nil {
		return fmt.Sprintf("((%s)%s == 0 ? %d : %d)", "uint64_t", "("+place+")"+n.path, n.empty.Disc, n.data.Disc)
	}
	return "(" + place + ").tag"
}

// enumIs returns a C condition testing whether place holds variant v.
func (g *Gen) enumIs(t Ty, place string, v *VariantDef) string {
	if n := g.niche(t); n != nil {
		if v == n.empty {
			return fmt.Sprintf("((%s)%s == 0)", place, n.path)
		}
		return fmt.Sprintf("((%s)%s != 0)", place, n.path)
	}
	return fmt.Sprintf("((%s).tag == %d)", place, v.Disc)
}

// enumPayload returns the C prefix for fields of variant vi of place.
func (g *Gen) enumPayload(t Ty, place string, vi int) string {
	if g.niche(t) != nil {
		return "(" + place + ")"
	}
	return fmt.Sprintf("(%s).u.v%d", place, vi)
}
