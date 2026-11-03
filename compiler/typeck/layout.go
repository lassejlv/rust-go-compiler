package typeck

import (
	"fmt"
	"math/bits"
	"sort"
	"strings"

	. "rustgo/compiler/middle"
)

// Type layout, modeled on rustc's repr(Rust) layout and shared by codegen
// (which emits C type definitions from these decisions) and const
// evaluation (size_of/align_of):
//
//   - struct, tuple and closure fields are reordered by descending
//     alignment (declaration order for repr(C)/transparent/packed; a field
//     that may be unsized stays last);
//   - enums use the smallest integer tag that fits their discriminants and
//     are laid out as a union of per-variant structs that each start with
//     the tag (variant fields in ascending alignment); repr(C) enums are
//     `struct { tag; union { struct {..} vN; } u; }`;
//   - an enum with one variant carrying data and otherwise field-less
//     variants stores the field-less variants in a niche of the data (null
//     pointers, invalid bools/chars, unused tag values of nested enums) when
//     that is smaller, like Option<&T>, Option<bool>, Option<Ordering>;
//   - a single-variant enum is laid out like a struct.
//
// The C translation targets x86-64 / LP64 GCC.

// EnumKind is the representation chosen for an enum.
type EnumKind int

const (
	EnumTagged  EnumKind = iota // union { tag; struct { tag; fields } vN; }
	EnumTaggedC                 // repr(C): struct { tag; union { struct { fields } vN; } u; }
	EnumNiche                   // the data variant's fields; other variants in a niche
	EnumSingle                  // one variant's fields, no tag (Data = -1: no variants)
)

// EnumLayout describes how an enum type is represented.
type EnumLayout struct {
	Kind EnumKind
	Tag  IntTy // tagged kinds
	Data int   // EnumNiche / EnumSingle: the variant whose fields form the value
	// EnumNiche: the niche scalar (Path from the enum value); variants
	// First..First+Count-1 are encoded as Base+(i-First) (Data may lie in
	// that range and is never encoded). With Union, the enum is a union of
	// per-variant structs (vN): variants other than Data carry fields,
	// placed at Offsets[vi], and a copy of the niche scalar (__n).
	Niche        *Niche
	First, Count int
	Base         uint64
	Union        bool
	Offsets      map[int]uint64
	Size, Align  uint64
	rest         *Niche // the niche left for enclosing enums
}

// Niche is a scalar inside a value with invalid bit patterns that an
// enclosing enum can use to encode its other variants.
type Niche struct {
	Path   string // C member path from the value to the scalar
	Offset uint64 // byte offset of the scalar in the value
	Bytes  uint64 // scalar size: 1, 2, 4, 8 or 16
	Ptr    bool   // the scalar is a pointer
	Start  uint64 // first invalid value (values wrap modulo 2^(8*Bytes))
	Avail  uint64 // number of consecutive invalid values from Start
}

// NicheValue returns the niche value encoding variant vi of an EnumNiche
// layout.
func (l *EnumLayout) NicheValue(vi int) uint64 {
	return wrapBytes(l.Base+uint64(vi-l.First), l.Niche.Bytes)
}

func wrapBytes(v, n uint64) uint64 {
	if n >= 8 {
		return v
	}
	return v & (1<<(8*n) - 1)
}

// reserve takes count values from niche n (like rustc's Niche::reserve):
// it returns the first value used and the niche that remains.
func reserve(n *Niche, count uint64) (uint64, *Niche, bool) {
	if count == 0 || count > n.Avail {
		return 0, nil, false
	}
	endSide := n.Start
	startSide := wrapBytes(n.Start+n.Avail-count, n.Bytes)
	dist := func(v uint64) uint64 {
		neg := wrapBytes(-v, n.Bytes)
		if n.Bytes >= 8 {
			neg = -v
		}
		return min(v, neg)
	}
	useEnd := true
	if count == 1 && !(n.Start == 2 && n.Bytes < 8 && n.Avail == 1<<(8*n.Bytes)-2) {
		// one value: the side closer to zero (ties: below the valid
		// range); a 0..=1 range (bool) grows upwards
		useEnd = dist(endSide) < dist(startSide)
	}
	rest := *n
	rest.Avail -= count
	if useEnd {
		rest.Start = wrapBytes(n.Start+count, n.Bytes)
		return endSide, &rest, true
	}
	return startSide, &rest, true
}

type layoutCaches struct {
	enums  map[Ty]*EnumLayout
	niches map[Ty]*Niche
	nicheK map[Ty]bool // niche computed (possibly nil)
	orders map[string][]int
	sizes  map[Ty][3]uint64 // size, align, ok
}

func (t *Tcx) lc() *layoutCaches {
	if t.layouts == nil {
		t.layouts = &layoutCaches{
			enums:  map[Ty]*EnumLayout{},
			niches: map[Ty]*Niche{},
			nicheK: map[Ty]bool{},
			orders: map[string][]int{},
			sizes:  map[Ty][3]uint64{},
		}
	}
	return t.layouts
}

// CLayout computes the size and alignment of a concrete, sized type as laid
// out by codegen's C translation. ok is false for unsized or non-concrete
// types.
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
	c := t.lc()
	if r, hit := c.sizes[ty]; hit {
		return r[0], r[1], r[2] == 1
	}
	size, align, ok = t.cLayoutUncached(ty, depth)
	okv := uint64(0)
	if ok {
		okv = 1
	}
	c.sizes[ty] = [3]uint64{size, align, okv}
	return
}

func (t *Tcx) cLayoutUncached(ty Ty, depth int) (size, align uint64, ok bool) {
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
		return t.structLayout(ty.Elems, t.FieldOrder(ty, 0), false, depth)
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
		fts := t.closureFieldTys(ty)
		if fts == nil {
			return 0, 0, false
		}
		return t.structLayout(fts, t.FieldOrder(ty, 0), false, depth)
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
			sz, al, ok := t.structLayoutPacked(fts, t.FieldOrder(ty, 0), d.ReprPacked(), depth)
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
			el := t.EnumLayoutOf(ty)
			if el == nil {
				return 0, 0, false
			}
			return t.enumSize(ty, el, depth)
		}
	}
	return 0, 0, false
}

// enumSize computes the size and alignment of an enum with layout el.
func (t *Tcx) enumSize(ty Ty, el *EnumLayout, depth int) (uint64, uint64, bool) {
	d := ty.Def
	switch el.Kind {
	case EnumNiche:
		return el.Size, el.Align, true
	case EnumSingle:
		if el.Data < 0 {
			return 0, 1, true
		}
		return t.structLayout(t.layoutFields(ty, el.Data), t.fieldOrder(ty, el.Data, false), false, depth)
	case EnumTaggedC:
		ts := uint64(el.Tag.Bits() / 8)
		var us, ua uint64 = 0, 1
		for vi := range d.Adt.Variants {
			s, a, ok := t.structLayout(t.layoutFields(ty, vi), t.fieldOrder(ty, vi, false), false, depth)
			if !ok {
				return 0, 0, false
			}
			us, ua = max(us, s), max(ua, a)
		}
		us = alignUp(us, ua)
		al := max(ts, ua)
		return alignUp(alignUp(ts, ua)+us, al), al, true
	default:
		ts := uint64(el.Tag.Bits() / 8)
		sz, al := ts, ts
		for vi := range d.Adt.Variants {
			fts := append([]Ty{IntT(el.Tag)}, t.layoutFields(ty, vi)...)
			order := []int{0}
			for _, i := range t.fieldOrder(ty, vi, true) {
				order = append(order, i+1)
			}
			s, a, ok := t.structLayout(fts, order, false, depth)
			if !ok {
				return 0, 0, false
			}
			sz, al = max(sz, s), max(al, a)
		}
		return alignUp(sz, al), al, true
	}
}

// structLayout lays out fields in the given memory order.
func (t *Tcx) structLayout(fts []Ty, order []int, packed bool, depth int) (uint64, uint64, bool) {
	var p uint64
	if packed {
		p = 1
	}
	return t.structLayoutPacked(fts, order, p, depth)
}

// structLayoutPacked: pack > 0 caps field alignments (repr(packed(N))).
func (t *Tcx) structLayoutPacked(fts []Ty, order []int, pack uint64, depth int) (uint64, uint64, bool) {
	var off, al uint64 = 0, 1
	for _, i := range order {
		s, a, ok := t.cLayout(fts[i], depth+1)
		if !ok {
			return 0, 0, false
		}
		if pack > 0 && a > pack {
			a = pack
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

// closureFieldTys returns the C field types of a closure's environment.
func (t *Tcx) closureFieldTys(ty Ty) []Ty {
	ci := t.Closures[ty]
	if ci == nil {
		return nil
	}
	fts := make([]Ty, len(ci.Captures))
	for i, c := range ci.Captures {
		if c.ByRef {
			fts[i] = Ref(c.Ty, false)
		} else {
			fts[i] = c.Ty
		}
	}
	return fts
}

// reprHas reports whether d has #[repr(.., word, ..)].
func reprHas(d *Def, word string) bool {
	for _, a := range d.Attrs {
		if a.Path == "repr" && len(a.Tokens) > 0 && a.Tokens[0].Delim == '(' {
			for _, tt := range a.Tokens[0].Inner {
				if tt.Tok.Text == word {
					return true
				}
			}
		}
	}
	return false
}

func layoutPacked(d *Def) bool { return reprHas(d, "packed") }

// ---------------------------------------------------------------------------
// Field order

// FieldOrder returns the memory order of the fields of a tuple, closure,
// struct (vi = 0) or enum variant vi: the field indices sorted by offset.
func (t *Tcx) FieldOrder(ty Ty, vi int) []int {
	prefixed := false
	if ty.Kind == TAdt && ty.Def.Adt != nil && ty.Def.Adt.Kind == AdtEnum {
		el := t.EnumLayoutOf(ty)
		prefixed = el != nil && el.Kind == EnumTagged
	}
	return t.fieldOrder(ty, vi, prefixed)
}

// fieldOrder: prefixed orders an enum variant laid out after a tag
// (ascending alignment); otherwise fields are sorted by descending
// alignment.
func (t *Tcx) fieldOrder(ty Ty, vi int, prefixed bool) []int {
	key := fmt.Sprintf("%d/%d/%v", ty.ID(), vi, prefixed)
	c := t.lc()
	if o, ok := c.orders[key]; ok {
		return o
	}
	var fts []Ty
	keepLast, fixed := false, false
	switch ty.Kind {
	case TTuple:
		fts = ty.Elems
	case TClosure:
		fts = t.closureFieldTys(ty)
	case TAdt:
		d := ty.Def
		t.AdtFields(d)
		fts = t.layoutFields(ty, vi)
		switch d.Adt.Kind {
		case AdtStruct:
			fixed = reprHas(d, "C") || reprHas(d, "transparent") || reprHas(d, "packed") || reprHas(d, "simd")
			keepLast = d.Adt.TailParam != -1
		case AdtEnum:
			// primitive/C representations keep declaration order
			fixed = d.Adt.Repr != "" || d.HasAttr("repr")
		default:
			fixed = true
		}
	}
	order := make([]int, len(fts))
	for i := range order {
		order[i] = i
	}
	c.orders[key] = order // provisional (recursive types through pointers)
	end := len(order)
	if keepLast && end > 0 {
		end--
	}
	if fixed || end <= 1 {
		return order
	}
	type info struct {
		size, align, niche uint64
		ok                 bool
	}
	infos := make([]info, len(fts))
	var maxAlign uint64 = 1
	anyNiche := false
	for i, f := range fts {
		s, a, ok := t.cLayout(f, 1)
		if !ok {
			return order // unsized or unknown: keep declaration order
		}
		var nv uint64
		if n := t.NicheOf(f); n != nil {
			nv = n.Avail
			anyNiche = true
		}
		infos[i] = info{s, a, nv, true}
		maxAlign = max(maxAlign, a)
	}
	group := func(i int) int {
		f := infos[i]
		g := bits.TrailingZeros64(max(f.align, f.size))
		if f.size == 0 {
			g = bits.TrailingZeros64(f.align)
		}
		if anyNiche {
			g = min(g, bits.TrailingZeros64(maxAlign))
		}
		return g
	}
	sorted := append([]int(nil), order[:end]...)
	sort.SliceStable(sorted, func(a, b int) bool {
		x, y := sorted[a], sorted[b]
		gx, gy := group(x), group(y)
		if prefixed {
			if gx != gy {
				return gx < gy
			}
			return infos[x].niche < infos[y].niche
		}
		if gx != gy {
			return gx > gy
		}
		return infos[x].niche > infos[y].niche
	})
	copy(order, sorted)
	c.orders[key] = order
	return order
}

// ---------------------------------------------------------------------------
// Enums

// EnumLayoutOf returns the representation of a concrete enum type.
func (t *Tcx) EnumLayoutOf(ty Ty) *EnumLayout {
	if ty.Kind != TAdt || ty.Def.Adt == nil || ty.Def.Adt.Kind != AdtEnum {
		return nil
	}
	c := t.lc()
	if el, ok := c.enums[ty]; ok {
		return el
	}
	d := ty.Def
	t.AdtFields(d)
	// provisional answer for recursive queries (enums containing themselves
	// behind pointers never ask for their own layout by value)
	tag := t.discrTag(d)
	c.enums[ty] = &EnumLayout{Kind: EnumTagged, Tag: tag}
	el := t.computeEnumLayout(ty, tag)
	c.enums[ty] = el
	return el
}

// discrTag returns the tag integer of an enum: its primitive repr, C int
// for repr(C), else the smallest integer fitting all discriminants.
func (t *Tcx) discrTag(d *Def) IntTy {
	if it, ok := IntTyByName(d.Adt.Repr); ok {
		return it
	}
	for _, a := range d.Attrs {
		if a.Path == "repr" && len(a.Tokens) > 0 && a.Tokens[0].Delim == '(' {
			for _, tt := range a.Tokens[0].Inner {
				if it, ok := IntTyByName(tt.Tok.Text); ok {
					return it
				}
			}
		}
	}
	if reprHas(d, "C") {
		return I32
	}
	var lo, hi int64
	for i, v := range d.Adt.Variants {
		if i == 0 || v.Disc < lo {
			lo = v.Disc
		}
		if i == 0 || v.Disc > hi {
			hi = v.Disc
		}
	}
	if lo >= 0 {
		switch {
		case hi <= 0xff:
			return U8
		case hi <= 0xffff:
			return U16
		case hi <= 0xffffffff:
			return U32
		}
		return U64
	}
	switch {
	case lo >= -128 && hi <= 127:
		return I8
	case lo >= -32768 && hi <= 32767:
		return I16
	case lo >= -1<<31 && hi <= 1<<31-1:
		return I32
	}
	return I64
}

func (t *Tcx) computeEnumLayout(ty Ty, tag IntTy) *EnumLayout {
	d := ty.Def
	vs := d.Adt.Variants
	if reprHas(d, "C") {
		return &EnumLayout{Kind: EnumTaggedC, Tag: tag}
	}
	tagged := &EnumLayout{Kind: EnumTagged, Tag: tag}
	if d.Adt.Repr != "" || d.HasAttr("repr") {
		return tagged
	}
	if len(vs) == 0 {
		return &EnumLayout{Kind: EnumSingle, Data: -1}
	}
	if len(vs) == 1 {
		return &EnumLayout{Kind: EnumSingle, Data: 0}
	}
	for _, v := range vs {
		if v.Disc != int64(v.Index) {
			return tagged // explicit discriminants: field-less enum
		}
	}
	// niche filling (rustc's niche_filling_layout): the largest variant
	// holds a niche that encodes all other variants, which must fit before
	// or after the niche scalar
	sizes := make([]uint64, len(vs))
	aligns := make([]uint64, len(vs))
	var maxAlign uint64 = 1
	largest := 0
	for vi := range vs {
		sz, al, ok := t.structLayout(t.layoutFields(ty, vi), t.fieldOrder(ty, vi, false), false, 1)
		if !ok {
			return tagged
		}
		sizes[vi], aligns[vi] = sz, al
		maxAlign = max(maxAlign, al)
		if sz >= sizes[largest] {
			largest = vi
		}
	}
	if sizes[largest] == 0 {
		return tagged
	}
	niche := t.fieldsNiche(t.layoutFields(ty, largest), t.fieldOrder(ty, largest, false), ".f")
	if niche == nil {
		return tagged
	}
	first, last := -1, -1
	for vi := range vs {
		if vi != largest {
			if first < 0 {
				first = vi
			}
			last = vi
		}
	}
	count := last - first + 1
	base, rest, ok := reserve(niche, uint64(count))
	if !ok {
		return tagged
	}
	size := alignUp(sizes[largest], maxAlign)
	nl := &EnumLayout{Kind: EnumNiche, Data: largest, First: first, Count: count, Base: base,
		Offsets: map[int]uint64{}, Size: size, Align: maxAlign}
	for vi := range vs {
		if vi == largest {
			continue
		}
		if len(vs[vi].Fields) > 0 {
			nl.Union = true
		}
		if sizes[vi] <= niche.Offset {
			nl.Offsets[vi] = 0
			continue
		}
		off := alignUp(niche.Offset+niche.Bytes, aligns[vi])
		if off+sizes[vi] > size {
			return tagged
		}
		nl.Offsets[vi] = off
	}
	if nl.Union {
		niche.Path = fmt.Sprintf(".v%d", largest) + niche.Path
		rest.Path = niche.Path
	}
	nl.Niche, nl.rest = niche, rest
	// rustc: use the niche layout when it is smaller, or as small with a
	// larger remaining niche
	ts, _, ok := t.enumSize(ty, tagged, 1)
	if !ok {
		return tagged
	}
	if size < ts || size == ts && rest.Avail > t.tagNicheAvail(d, tag) {
		return nl
	}
	return tagged
}

// VariantEnd returns the end offset of the last field of variant vi laid
// out as a struct (EnumNiche layouts), before trailing padding.
func (t *Tcx) VariantEnd(ty Ty, vi int) uint64 {
	fts := t.layoutFields(ty, vi)
	var off uint64
	for _, i := range t.fieldOrder(ty, vi, false) {
		s, a, ok := t.cLayout(fts[i], 1)
		if !ok {
			return 0
		}
		off = alignUp(off, a) + s
	}
	return off
}

// tagNicheAvail is the number of unused values of a tagged enum's tag.
func (t *Tcx) tagNicheAvail(d *Def, tag IntTy) uint64 {
	n := t.tagNiche(d, tag)
	if n == nil {
		return 0
	}
	return n.Avail
}

// tagNiche returns the niche in the tag of a tagged enum: the tag values
// outside its discriminant range (wrapping around from the largest).
func (t *Tcx) tagNiche(d *Def, tag IntTy) *Niche {
	vs := d.Adt.Variants
	if len(vs) == 0 {
		return nil
	}
	lo, hi := vs[0].Disc, vs[0].Disc
	for _, v := range vs {
		lo, hi = min(lo, v.Disc), max(hi, v.Disc)
	}
	b := uint64(tag.Bits() / 8)
	if b > 8 {
		return nil
	}
	used := uint64(hi-lo) + 1
	var space uint64
	if b == 8 {
		space = ^uint64(0) - used + 1
	} else {
		space = (uint64(1) << (8 * b)) - used
	}
	if space == 0 || used == 0 {
		return nil
	}
	return &Niche{Path: ".tag", Bytes: b, Start: wrapBytes(uint64(hi)+1, b), Avail: space}
}

// ---------------------------------------------------------------------------
// Niches

// NicheOf returns the largest niche of a concrete type, or nil.
func (t *Tcx) NicheOf(ty Ty) *Niche {
	c := t.lc()
	if c.nicheK[ty] {
		return c.niches[ty]
	}
	c.nicheK[ty] = true // cycles: no niche
	n := t.computeNiche(ty)
	c.niches[ty] = n
	return n
}

func ptrNiche(path string) *Niche { return &Niche{Path: path, Bytes: 8, Ptr: true, Start: 0, Avail: 1} }

func (t *Tcx) computeNiche(ty Ty) *Niche {
	if ty == nil || ty.HasParam() || ty.HasInfer() || ty.HasError() {
		return nil
	}
	switch ty.Kind {
	case TBool:
		return &Niche{Bytes: 1, Start: 2, Avail: 254}
	case TChar:
		return &Niche{Bytes: 4, Start: 0x110000, Avail: 1<<32 - 0x110000}
	case TRef:
		if ty.Elem.IsUnsized() {
			return ptrNiche(".ptr")
		}
		return ptrNiche("")
	case TFnPtr:
		return ptrNiche("")
	case TTuple:
		return t.fieldsNiche(ty.Elems, t.FieldOrder(ty, 0), ".f")
	case TArray:
		if ty.Len == nil || ty.Len.Kind != TConst || ty.Len.Val == 0 {
			return nil
		}
		n := t.NicheOf(ty.Elem)
		if n == nil {
			return nil
		}
		m := *n
		m.Path = ".a[0]" + n.Path
		return &m
	case TClosure:
		fts := t.closureFieldTys(ty)
		if fts == nil {
			return nil
		}
		return t.fieldsNiche(fts, t.FieldOrder(ty, 0), ".c")
	case TAdt:
		d := ty.Def
		t.AdtFields(d)
		if d.Adt == nil {
			return nil
		}
		switch d.Adt.Kind {
		case AdtStruct:
			if d.LangItem == "unsafe_cell" || isStdNamed(d, "MaybeUninit") || layoutPacked(d) {
				return nil
			}
			fts := t.layoutFields(ty, 0)
			if isNonNullStruct(d) && len(fts) == 1 {
				return nonZeroScalar(fts[0], ".f0")
			}
			order := t.FieldOrder(ty, 0)
			offs := t.fieldOffsets(fts, order)
			var best *Niche
			for _, i := range order {
				var n *Niche
				if fieldHasAttr(d.Adt.Variants[0].Fields[i], "rustgo_nonnull") {
					n = nonZeroScalar(fts[i], "")
				} else {
					n = t.NicheOf(fts[i])
				}
				if n != nil && (best == nil || n.Avail > best.Avail) {
					m := *n
					m.Path = fmt.Sprintf(".f%d", i) + n.Path
					m.Offset = offs[i] + n.Offset
					best = &m
				}
			}
			return best
		case AdtEnum:
			el := t.EnumLayoutOf(ty)
			switch el.Kind {
			case EnumTagged, EnumTaggedC:
				return t.tagNiche(d, el.Tag)
			case EnumSingle:
				if el.Data < 0 {
					return nil
				}
				return t.fieldsNiche(t.layoutFields(ty, el.Data), t.FieldOrder(ty, el.Data), ".f")
			case EnumNiche:
				if el.rest == nil || el.rest.Avail == 0 {
					return nil
				}
				m := *el.rest
				return &m
			}
		}
	}
	return nil
}

// fieldOffsets returns the byte offset of each field laid out in order.
func (t *Tcx) fieldOffsets(fts []Ty, order []int) []uint64 {
	offs := make([]uint64, len(fts))
	var off uint64
	for _, i := range order {
		s, a, ok := t.cLayout(fts[i], 1)
		if !ok {
			return offs
		}
		off = alignUp(off, a)
		offs[i] = off
		off += s
	}
	return offs
}

func (t *Tcx) fieldsNiche(fts []Ty, order []int, prefix string) *Niche {
	offs := t.fieldOffsets(fts, order)
	var best *Niche
	for _, i := range order {
		n := t.NicheOf(fts[i])
		if n != nil && (best == nil || n.Avail > best.Avail) {
			m := *n
			m.Path = fmt.Sprintf("%s%d", prefix, i) + n.Path
			m.Offset = offs[i] + n.Offset
			best = &m
		}
	}
	return best
}

// nonZeroScalar is the niche of a never-zero pointer or integer field.
func nonZeroScalar(f Ty, path string) *Niche {
	switch f.Kind {
	case TPtr, TRef:
		if f.Elem.IsUnsized() {
			return ptrNiche(path + ".ptr")
		}
		return ptrNiche(path)
	case TFnPtr:
		return ptrNiche(path)
	case TInt:
		return &Niche{Path: path, Bytes: uint64(f.IntKind().Bits() / 8), Start: 0, Avail: 1}
	}
	return nil
}

// isNonNullStruct: Box, NonNull, NonZero* and types marked with
// #[rustc_nonnull_optimization_guaranteed] wrap a never-zero scalar.
func isNonNullStruct(d *Def) bool {
	return d.LangItem == "owned_box" || d.HasAttr("rustc_nonnull_optimization_guaranteed") ||
		(d.Crate != nil && d.Crate.Name == "std" && (d.Name == "NonNull" || strings.HasPrefix(d.Name, "NonZero")))
}

func isStdNamed(d *Def, name string) bool {
	return d.Crate != nil && d.Crate.Name == "std" && d.Name == name
}

func fieldHasAttr(f *FieldDef, name string) bool {
	if f.Ast == nil {
		return false
	}
	for _, a := range f.Ast.Attrs {
		if a.Path == name {
			return true
		}
	}
	return false
}
