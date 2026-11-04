package typeck

import (
	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

// checkDiscRange checks that the discriminants of an enum fit its
// representation type (isize by default): an explicit literal out of range
// is the overflowing_literals lint (a negative one for an unsigned repr is
// E0600), an implicit discriminant after the type's maximum is E0370.
// It returns false when it reported an error.
func (t *Tcx) checkDiscRange(d *Def, k *ast.EnumItem) bool {
	repr := Isize
	if d.Adt != nil && d.Adt.Repr != "" {
		it, ok := IntTyByName(d.Adt.Repr)
		if !ok {
			return true
		}
		repr = it
	}
	bits := repr.Bits()
	if bits > 64 || (bits == 64 && !repr.Signed()) {
		return true // values beyond int64 are not modeled
	}
	var lo, hi int64
	switch {
	case bits == 64:
		lo, hi = -1<<63, 1<<63-1
	case repr.Signed():
		lo, hi = -(1 << (bits - 1)), 1<<(bits-1)-1
	default:
		lo, hi = 0, 1<<bits-1
	}
	for i, va := range k.Variants {
		if i >= len(d.Adt.Variants) {
			break
		}
		v := d.Adt.Variants[i].Disc
		if va.Disc != nil {
			e := unparen(va.Disc)
			neg := false
			if u, ok := e.(*ast.UnaryExpr); ok && u.Op == ast.UnNeg {
				neg = true
				e = unparen(u.X)
			}
			lit, ok := e.(*ast.LitExpr)
			if !ok || lit.Kind != ast.LitInt || lit.Suffix != "" || attrsAllowLint(d.Attrs, "overflowing_literals") || lintAllowedAt(d, "overflowing_literals") {
				continue
			}
			if neg && !repr.Signed() {
				t.ErrorCode(va.Disc.GetSpan(), "E0600", "cannot apply unary operator `-` to type `%s`", repr)
				return false
			}
			if lit.Big != "" || (!neg && lit.IntVal > uint64(hi)) || (neg && lit.IntVal > uint64(-(lo+1))+1) {
				t.Errorf(lit.Span, "literal out of range for `%s` (#[deny(overflowing_literals)] on by default)", repr)
				return false
			}
			continue
		}
		if i > 0 {
			prev := d.Adt.Variants[i-1].Disc
			if prev == hi || v < lo || v > hi {
				t.ErrorCode(va.Name.Span, "E0370", "enum discriminant overflowed: `%s` overflowed on value after %d", va.Name.Name, prev)
				return false
			}
		}
	}
	return true
}
