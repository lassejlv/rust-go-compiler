package typeck

import (
	"strings"

	. "rustgo/compiler/middle"
)

// CLayout computes the size and alignment of a concrete, sized type as laid
// out by codegen's C translation (x86-64 / LP64 GCC): structs and tuples are
// C structs in declaration order, arrays are `struct { T a[N]; }`, unit-like
// types are empty structs (size 0), wide pointers are `{ptr, meta}`, enums
// are `struct { tag; union { struct {..} vN; } u; }` except niche-optimized
// Option-like enums, which are just their payload. It must stay in sync with
// codegen/types.go and codegen/layout.go. ok is false for unsized or
// non-concrete types.
func (t *Tcx) CLayout(ty Ty) (size, align uint64, ok bool) {
	return t.cLayout(ty, 0)
}

func (t *Tcx) cLayout(ty Ty, depth int) (size, align uint64, ok bool) {
	if depth > 64 || ty == nil || ty.HasError() || ty.HasParam() || ty.HasInfer() {
		return 0, 0, false
	}
	if ty.HasProj() || ty.HasOpaque() {
		ty = t.Normalize(ty)
		if ty.HasProj() || ty.HasOpaque() {
			return 0, 0, false
		}
	}
	switch ty.Kind {
	case TBool:
		return 1, 1, true
	case TChar:
		return 4, 4, true
	case TInt:
		n := uint64(ty.IntKind().Bits() / 8)
		return n, n, true
	case TFloat:
		n := uint64(ty.Sub / 8)
		return n, n, true
	case TNever, TFnDef:
		return 0, 1, true
	case TRef, TPtr:
		if ty.Elem.IsUnsized() {
			return 16, 8, true
		}
		return 8, 8, true
	case TFnPtr:
		return 8, 8, true
	case TTuple:
		return t.structLayout(ty.Elems, false, depth)
	case TArray:
		if ty.Len == nil || ty.Len.Kind != TConst {
			return 0, 0, false
		}
		es, ea, ok := t.cLayout(ty.Elem, depth+1)
		if !ok {
			return 0, 0, false
		}
		return es * ty.Len.Val, ea, true
	case TClosure:
		ci := t.Closures[ty]
		if ci == nil {
			return 0, 0, false
		}
		fts := make([]Ty, len(ci.Captures))
		for i, c := range ci.Captures {
			if c.ByRef {
				fts[i] = Ref(c.Ty, false)
			} else {
				fts[i] = c.Ty
			}
		}
		return t.structLayout(fts, false, depth)
	case TAdt:
		d := ty.Def
		t.AdtFields(d)
		if d.Adt == nil {
			return 0, 0, false
		}
		switch d.Adt.Kind {
		case AdtStruct:
			fts := t.layoutFields(ty, 0)
			for _, f := range fts {
				if f.IsUnsized() {
					return 0, 0, false
				}
			}
			sz, al, ok := t.structLayout(fts, layoutPacked(d), depth)
			if n := d.ReprAlign(); ok && n > al {
				al = n
				sz = alignUp(sz, al)
			}
			return sz, al, ok
		case AdtUnion:
			var sz, al uint64 = 0, 1
			for _, f := range t.layoutFields(ty, 0) {
				s, a, ok := t.cLayout(f, depth+1)
				if !ok {
					return 0, 0, false
				}
				sz, al = max(sz, s), max(al, a)
			}
			return alignUp(sz, al), al, true
		case AdtEnum:
			if data := t.nicheData(ty); data != nil {
				return t.structLayout(t.layoutFields(ty, data.Index)[:1], false, depth)
			}
			ts, ta := enumTagLayout(d)
			// union of per-variant structs
			var us, ua uint64 = 0, 1
			for vi := range d.Adt.Variants {
				s, a, ok := t.structLayout(t.layoutFields(ty, vi), false, depth)
				if !ok {
					return 0, 0, false
				}
				us, ua = max(us, s), max(ua, a)
			}
			us = alignUp(us, ua)
			al := max(ta, ua)
			return alignUp(alignUp(ts, ua)+us, al), al, true
		}
	}
	return 0, 0, false
}

func (t *Tcx) structLayout(fts []Ty, packed bool, depth int) (uint64, uint64, bool) {
	var off, al uint64 = 0, 1
	for _, f := range fts {
		s, a, ok := t.cLayout(f, depth+1)
		if !ok {
			return 0, 0, false
		}
		if packed {
			a = 1
		}
		off = alignUp(off, a) + s
		al = max(al, a)
	}
	return alignUp(off, al), al, true
}

func alignUp(n, a uint64) uint64 {
	if a <= 1 {
		return n
	}
	return (n + a - 1) / a * a
}

func (t *Tcx) layoutFields(ty Ty, vi int) []Ty {
	v := ty.Def.Adt.Variants[vi]
	out := make([]Ty, len(v.Fields))
	for i, f := range v.Fields {
		out[i] = t.Normalize(Subst(f.Ty, ty.Args))
	}
	return out
}

// enumTagLayout mirrors codegen's tagType: the repr integer, int32_t for
// repr(C), int64_t otherwise.
func enumTagLayout(d *Def) (uint64, uint64) {
	if it, ok := IntTyByName(d.Adt.Repr); ok {
		n := uint64(it.Bits() / 8)
		return n, n
	}
	for _, a := range d.Attrs {
		if a.Path == "repr" && len(a.Tokens) > 0 && a.Tokens[0].Delim == '(' {
			for _, tt := range a.Tokens[0].Inner {
				if tt.Tok.Text == "C" {
					return 4, 4
				}
			}
		}
	}
	return 8, 8
}

func layoutPacked(d *Def) bool {
	for _, a := range d.Attrs {
		if a.Path == "repr" && len(a.Tokens) > 0 && a.Tokens[0].Delim == '(' {
			for _, tt := range a.Tokens[0].Inner {
				if tt.Tok.Text == "packed" {
					return true
				}
			}
		}
	}
	return false
}

// nicheData mirrors codegen's niche(): for a two-variant enum (no repr)
// with one empty variant and one single-field variant whose field has a
// never-zero scalar, it returns the data variant (the enum is laid out as
// that field alone).
func (t *Tcx) nicheData(ty Ty) *VariantDef {
	d := ty.Def
	if len(d.Adt.Variants) != 2 || d.Adt.Repr != "" || d.HasAttr("repr") {
		return nil
	}
	var data, empty *VariantDef
	for vi, v := range d.Adt.Variants {
		switch len(t.layoutFields(ty, vi)) {
		case 0:
			empty = v
		case 1:
			data = v
		}
	}
	if data == nil || empty == nil {
		return nil
	}
	if !t.hasNiche(t.layoutFields(ty, data.Index)[0], 0) {
		return nil
	}
	return data
}

// hasNiche mirrors codegen's nichePath.
func (t *Tcx) hasNiche(ty Ty, depth int) bool {
	if depth > 4 {
		return false
	}
	switch ty.Kind {
	case TRef, TFnPtr:
		return true
	case TAdt:
		d := ty.Def
		t.AdtFields(d)
		if d.Adt == nil || d.Adt.Kind != AdtStruct {
			return false
		}
		fts := t.layoutFields(ty, 0)
		if len(fts) != 1 {
			return false
		}
		nonnull := d.LangItem == "owned_box" || d.HasAttr("rustc_nonnull_optimization_guaranteed") ||
			(d.Crate != nil && d.Crate.Name == "std" && (d.Name == "NonNull" || strings.HasPrefix(d.Name, "NonZero")))
		if nonnull {
			f := fts[0]
			if f.Kind == TPtr || f.Kind == TInt {
				return true
			}
			return t.hasNiche(f, depth+1)
		}
		return t.hasNiche(fts[0], depth+1)
	}
	return false
}
