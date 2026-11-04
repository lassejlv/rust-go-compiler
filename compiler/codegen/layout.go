package codegen

// Enum layout. The representation of each enum type (tagged, repr(C)
// tagged, niche-filled, single variant) is decided by typeck's layout
// (typeck/layout.go), which models rustc's: e.g. Option<&T> and
// Option<extern "C" fn()> are nullable pointers, Option<bool> is one byte,
// Option<u32> is a one-byte tag plus the payload.

import (
	"fmt"
	"strings"

	. "rustgo/compiler/middle"
	"rustgo/compiler/typeck"
)

func (g *Gen) enumLayout(t Ty) *typeck.EnumLayout { return g.tcx.EnumLayoutOf(t) }

// uintC is the unsigned C integer type of n bytes.
func uintC(n uint64) string {
	switch n {
	case 1:
		return "uint8_t"
	case 2:
		return "uint16_t"
	case 4:
		return "uint32_t"
	case 16:
		return "unsigned __int128"
	}
	return "uint64_t"
}

// nicheRead is the niche scalar of an EnumNiche place as an unsigned int.
func nicheRead(el *typeck.EnumLayout, place string) string {
	n := el.Niche
	return fmt.Sprintf("((%s)(%s)%s)", uintC(n.Bytes), place, n.Path)
}

// nicheRel is (scalar - Base) in the scalar's unsigned type.
func nicheRel(el *typeck.EnumLayout, place string) string {
	ut := uintC(el.Niche.Bytes)
	return fmt.Sprintf("((%s)(%s - (%s)%dull))", ut, nicheRead(el, place), ut, el.Base)
}

// enumTag returns a C expression for the discriminant of an enum place.
func (g *Gen) enumTag(t Ty, place string) string {
	el := g.enumLayout(t)
	switch el.Kind {
	case typeck.EnumSingle:
		if el.Data < 0 {
			return "0"
		}
		return fmt.Sprintf("%d", t.Def.Adt.Variants[el.Data].Disc)
	case typeck.EnumNiche:
		return fmt.Sprintf("({ %s __nr = %s; __nr < %d ? (int64_t)(%d + __nr) : (int64_t)%d; })",
			uintC(el.Niche.Bytes), nicheRel(el, place), el.Count, el.First, el.Data)
	}
	return "(" + place + ").tag"
}

// enumIs returns a C condition testing whether place holds variant v.
func (g *Gen) enumIs(t Ty, place string, v *VariantDef) string {
	el := g.enumLayout(t)
	switch el.Kind {
	case typeck.EnumSingle:
		return "1"
	case typeck.EnumNiche:
		if v.Index != el.Data {
			return fmt.Sprintf("(%s == (%s)%dull)", nicheRead(el, place), uintC(el.Niche.Bytes), el.NicheValue(v.Index))
		}
		if el.Count == 1 {
			return fmt.Sprintf("(%s != (%s)%dull)", nicheRead(el, place), uintC(el.Niche.Bytes), el.NicheValue(el.First))
		}
		rel := nicheRel(el, place)
		if el.Data >= el.First && el.Data < el.First+el.Count {
			return fmt.Sprintf("({ %s __nr = %s; __nr >= %d || __nr == %d; })", uintC(el.Niche.Bytes), rel, el.Count, el.Data-el.First)
		}
		return fmt.Sprintf("(%s >= %d)", rel, el.Count)
	}
	return fmt.Sprintf("((%s).tag == %d)", place, v.Disc)
}

// enumPayload returns the C prefix for fields of variant vi of place.
func (g *Gen) enumPayload(t Ty, place string, vi int) string {
	el := g.enumLayout(t)
	switch el.Kind {
	case typeck.EnumSingle:
		return "(" + place + ")"
	case typeck.EnumNiche:
		if !el.Union {
			return "(" + place + ")"
		}
	case typeck.EnumTaggedC:
		return fmt.Sprintf("(%s).u.v%d", place, vi)
	}
	return fmt.Sprintf("(%s).v%d", place, vi)
}

// enumLiteral builds a compound literal for variant v of enum type t.
func (g *Gen) enumLiteral(t Ty, v *VariantDef, fields []string) string {
	ct := g.ctype(t)
	el := g.enumLayout(t)
	var fs []string
	for i, f := range fields {
		fs = append(fs, fmt.Sprintf(".f%d = %s", i, f))
	}
	switch el.Kind {
	case typeck.EnumSingle:
		return fmt.Sprintf("((%s){ %s })", ct, strings.Join(fs, ", "))
	case typeck.EnumNiche:
		if el.Union {
			if v.Index != el.Data {
				fs = append([]string{fmt.Sprintf(".__n = %dull", el.NicheValue(v.Index))}, fs...)
			}
			return fmt.Sprintf("((%s){ .v%d = { %s } })", ct, v.Index, strings.Join(fs, ", "))
		}
		if v.Index == el.Data {
			return fmt.Sprintf("((%s){ %s })", ct, strings.Join(fs, ", "))
		}
		val := el.NicheValue(v.Index)
		if val == 0 {
			return fmt.Sprintf("((%s){})", ct)
		}
		return fmt.Sprintf("((%s){ %s = %dull })", ct, el.Niche.Path, val)
	case typeck.EnumTaggedC:
		if len(fs) == 0 {
			return fmt.Sprintf("((%s){ .tag = %d })", ct, v.Disc)
		}
		return fmt.Sprintf("((%s){ .tag = %d, .u.v%d = { %s } })", ct, v.Disc, v.Index, strings.Join(fs, ", "))
	}
	return fmt.Sprintf("((%s){ .v%d = { %s } })", ct, v.Index, strings.Join(append([]string{fmt.Sprintf(".tag = %d", v.Disc)}, fs...), ", "))
}

// enumDef writes the C struct definition of enum type t.
func (g *Gen) enumDef(b *strings.Builder, t Ty, name string) {
	d := t.Def
	el := g.enumLayout(t)
	// the end of the enum's struct: `#[repr(align(N))]` raises its alignment
	end := " };\n"
	if n := d.ReprAlign(); n > 0 {
		end = fmt.Sprintf(" } __attribute__((aligned(%d)));\n", n)
	}
	fieldsIn := func(vi int) string {
		var sb strings.Builder
		fts := g.fieldTypes(t, vi)
		for _, i := range g.variantOrder(t, vi) {
			fmt.Fprintf(&sb, " %s f%d;", g.ctype(fts[i]), i)
		}
		return sb.String()
	}
	switch el.Kind {
	case typeck.EnumNiche:
		if !el.Union {
			fmt.Fprintf(b, "struct %s {%s%s", name, fieldsIn(el.Data), end)
			return
		}
		// union of variant structs; the others replicate the niche scalar
		// (__n) at its offset and place their fields before or after it
		nt := uintC(el.Niche.Bytes)
		no, nb := el.Niche.Offset, el.Niche.Bytes
		fmt.Fprintf(b, "struct %s { union {", name)
		for vi := range d.Adt.Variants {
			if vi == el.Data {
				fmt.Fprintf(b, " struct {%s } v%d;", fieldsIn(vi), vi)
				continue
			}
			off := el.Offsets[vi]
			if off == 0 && len(d.Adt.Variants[vi].Fields) > 0 {
				end := g.tcx.VariantEnd(t, vi)
				fmt.Fprintf(b, " struct {%s uint8_t __p[%d]; %s __n; } v%d;", fieldsIn(vi), no-end, nt, vi)
			} else {
				fmt.Fprintf(b, " struct { uint8_t __p0[%d]; %s __n; uint8_t __p1[%d];%s } v%d;", no, nt, off-min(off, no+nb), fieldsIn(vi), vi)
			}
		}
		b.WriteString(" };" + end)
	case typeck.EnumSingle:
		if el.Data < 0 {
			fmt.Fprintf(b, "struct %s {%s", name, end)
			return
		}
		fmt.Fprintf(b, "struct %s {%s%s", name, fieldsIn(el.Data), end)
	case typeck.EnumTaggedC:
		fmt.Fprintf(b, "struct %s { %s tag; union {", name, g.ctype(IntT(el.Tag)))
		for vi := range d.Adt.Variants {
			fmt.Fprintf(b, " struct {%s } v%d;", fieldsIn(vi), vi)
		}
		b.WriteString(" } u;" + end)
	default:
		tag := g.ctype(IntT(el.Tag))
		fmt.Fprintf(b, "struct %s { union { %s tag;", name, tag)
		for vi := range d.Adt.Variants {
			fmt.Fprintf(b, " struct { %s tag;%s } v%d;", tag, fieldsIn(vi), vi)
		}
		b.WriteString(" };" + end)
	}
}

// variantOrder is the memory order of the fields of variant vi.
func (g *Gen) variantOrder(t Ty, vi int) []int { return g.tcx.FieldOrder(t, vi) }

func hasReprAttr(d *Def) bool {
	for _, a := range d.Attrs {
		if a.Path == "repr" {
			return true
		}
	}
	return false
}
