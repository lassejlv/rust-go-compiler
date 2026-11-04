package typeck

import (
	"math"
	"math/big"
	"math/bits"
	"strings"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

// Numeric values and operations of the const evaluator. Integers are
// arbitrary precision (*big.Int, never mutated once built) wrapped to their
// type's width when the type is known, so all of i8..i128/u8..u128 behave
// exactly; floats are float64 (rounded to f32 when typed so).

var bigOne = big.NewInt(1)

func pow2(n uint) *big.Int { return new(big.Int).Lsh(bigOne, n) }

// wrapInt reduces n to the range of it (two's complement wrapping).
func wrapInt(n *big.Int, it IntTy) *big.Int {
	b := it.Bits()
	if it.Signed() {
		if n.BitLen() < b {
			return n
		}
	} else if n.Sign() >= 0 && n.BitLen() <= b {
		return n
	}
	r := new(big.Int).And(n, new(big.Int).Sub(pow2(uint(b)), bigOne))
	if it.Signed() && r.Bit(b-1) == 1 {
		r.Sub(r, pow2(uint(b)))
	}
	return r
}

func intMaxBig(it IntTy) *big.Int {
	if it.Signed() {
		return new(big.Int).Sub(pow2(uint(it.Bits()-1)), bigOne)
	}
	return new(big.Int).Sub(pow2(uint(it.Bits())), bigOne)
}

func intMinBig(it IntTy) *big.Int {
	if it.Signed() {
		return new(big.Int).Neg(pow2(uint(it.Bits() - 1)))
	}
	return new(big.Int)
}

func inRange(n *big.Int, it IntTy) bool {
	return n.Cmp(intMinBig(it)) >= 0 && n.Cmp(intMaxBig(it)) <= 0
}

func cvI(n *big.Int, it IntTy, typed bool) *cval {
	if typed {
		n = wrapInt(n, it)
	}
	return &cval{k: cvInt, n: n, ity: it, typed: typed}
}

func cvIi(v int64, it IntTy, typed bool) *cval { return cvI(big.NewInt(v), it, typed) }

func cvB(b bool) *cval { return &cval{k: cvBool, b: b} }

func cvF(f float64, is32, typed bool) *cval {
	if is32 {
		f = float64(float32(f))
	}
	return &cval{k: cvFloat, f: f, f32: is32, typed: typed}
}

// intOf returns the integer value of an int or bool.
func intOf(v *cval) (*big.Int, bool) {
	switch v.k {
	case cvInt:
		return v.n, true
	case cvBool:
		if v.b {
			return big.NewInt(1), true
		}
		return new(big.Int), true
	}
	return nil, false
}

// smallInt returns an int/bool value as int64 if it fits.
func smallInt(v *cval) (int64, bool) {
	n, ok := intOf(v)
	if !ok || !n.IsInt64() {
		return 0, false
	}
	return n.Int64(), true
}

func truthy(v *cval) (bool, bool) {
	switch v.k {
	case cvBool:
		return v.b, true
	case cvInt:
		return v.n.Sign() != 0, true
	}
	return false, false
}

// retype converts an integer/float value to the given type (when it is a
// numeric type).
func retype(v *cval, ty Ty) *cval {
	if v == nil || ty == nil {
		return v
	}
	switch {
	case v.k == cvInt && ty.Kind == TInt:
		return cvI(v.n, ty.IntKind(), true)
	case v.k == cvFloat && ty.Kind == TFloat:
		return cvF(v.f, ty.Sub == 32, true)
	}
	return v
}

// castTo implements `v as ty` for numeric/bool/char values.
func castTo(v *cval, ty Ty) (*cval, bool) {
	switch ty.Kind {
	case TInt:
		it := ty.IntKind()
		switch v.k {
		case cvInt, cvBool:
			n, _ := intOf(v)
			return cvI(n, it, true), true
		case cvFloat:
			f := v.f
			if math.IsNaN(f) {
				return cvIi(0, it, true), true
			}
			if math.IsInf(f, 0) {
				if f > 0 {
					return cvI(intMaxBig(it), it, true), true
				}
				return cvI(intMinBig(it), it, true), true
			}
			n, _ := new(big.Float).SetFloat64(math.Trunc(f)).Int(nil)
			if n.Cmp(intMaxBig(it)) > 0 {
				n = intMaxBig(it)
			} else if n.Cmp(intMinBig(it)) < 0 {
				n = intMinBig(it)
			}
			return cvI(n, it, true), true
		}
	case TFloat:
		is32 := ty.Sub == 32
		switch v.k {
		case cvInt, cvBool:
			n, _ := intOf(v)
			f, _ := new(big.Float).SetInt(n).Float64()
			return cvF(f, is32, true), true
		case cvFloat:
			return cvF(v.f, is32, true), true
		}
	case TChar:
		if v.k == cvInt {
			return cvI(v.n, 0, false), true
		}
	case TBool:
		if v.k == cvBool {
			return v, true
		}
	}
	return nil, false
}

// arithPanics reports whether integer arithmetic op on a and b panics at
// run time (and so fails const evaluation): the result overflows a known
// type, or a division or remainder by zero.
func arithPanics(op ast.BinOp, a, b *cval) bool {
	if a == nil || b == nil || a.k != cvInt || b.k != cvInt {
		return false
	}
	it, typed := a.ity, a.typed
	if !typed {
		it, typed = b.ity, b.typed
	}
	x, y := a.n, b.n
	r := new(big.Int)
	switch op {
	case ast.BinAdd:
		r.Add(x, y)
	case ast.BinSub:
		r.Sub(x, y)
	case ast.BinMul:
		r.Mul(x, y)
	case ast.BinDiv, ast.BinRem:
		if y.Sign() == 0 {
			return true
		}
		r.Quo(x, y)
	default:
		return false
	}
	return typed && !inRange(r, it)
}

func binop(op ast.BinOp, a, b *cval) (*cval, bool) {
	if a.k == cvFloat && b.k == cvFloat {
		return floatBinop(op, a, b)
	}
	if a.k != cvInt || b.k != cvInt {
		if a.k == cvBool && b.k == cvBool {
			switch op {
			case ast.BinBitAnd:
				return cvB(a.b && b.b), true
			case ast.BinBitOr:
				return cvB(a.b || b.b), true
			case ast.BinBitXor:
				return cvB(a.b != b.b), true
			}
		}
		switch op {
		case ast.BinEq:
			return cvB(cvEqual(a, b)), true
		case ast.BinNe:
			return cvB(!cvEqual(a, b)), true
		}
		x, ok1 := intOf(a)
		y, ok2 := intOf(b)
		if ok1 && ok2 {
			return cmpResult(op, x.Cmp(y))
		}
		return nil, false
	}
	it, typed := a.ity, a.typed
	if !typed && op != ast.BinShl && op != ast.BinShr {
		it, typed = b.ity, b.typed
	}
	x, y := a.n, b.n
	r := new(big.Int)
	switch op {
	case ast.BinAdd:
		r.Add(x, y)
	case ast.BinSub:
		r.Sub(x, y)
	case ast.BinMul:
		r.Mul(x, y)
	case ast.BinDiv:
		if y.Sign() == 0 {
			return nil, false
		}
		r.Quo(x, y)
	case ast.BinRem:
		if y.Sign() == 0 {
			return nil, false
		}
		r.Rem(x, y)
	case ast.BinShl, ast.BinShr:
		if y.Sign() < 0 || y.Cmp(big.NewInt(128)) >= 0 {
			return nil, false
		}
		if op == ast.BinShl {
			r.Lsh(x, uint(y.Int64()))
		} else {
			r.Rsh(x, uint(y.Int64()))
		}
	case ast.BinBitAnd:
		r.And(x, y)
	case ast.BinBitOr:
		r.Or(x, y)
	case ast.BinBitXor:
		r.Xor(x, y)
	case ast.BinEq, ast.BinNe, ast.BinLt, ast.BinLe, ast.BinGt, ast.BinGe:
		return cmpResult(op, x.Cmp(y))
	default:
		return nil, false
	}
	if typed && op != ast.BinShl && op != ast.BinShr && !inRange(r, it) {
		return nil, false // arithmetic overflow
	}
	return cvI(r, it, typed), true
}

func cmpResult(op ast.BinOp, c int) (*cval, bool) {
	switch op {
	case ast.BinEq:
		return cvB(c == 0), true
	case ast.BinNe:
		return cvB(c != 0), true
	case ast.BinLt:
		return cvB(c < 0), true
	case ast.BinLe:
		return cvB(c <= 0), true
	case ast.BinGt:
		return cvB(c > 0), true
	case ast.BinGe:
		return cvB(c >= 0), true
	}
	return nil, false
}

func floatBinop(op ast.BinOp, a, b *cval) (*cval, bool) {
	is32, typed := a.f32, a.typed
	if !typed {
		is32, typed = b.f32, b.typed
	}
	x, y := a.f, b.f
	switch op {
	case ast.BinAdd:
		return cvF(x+y, is32, typed), true
	case ast.BinSub:
		return cvF(x-y, is32, typed), true
	case ast.BinMul:
		return cvF(x*y, is32, typed), true
	case ast.BinDiv:
		return cvF(x/y, is32, typed), true
	case ast.BinRem:
		return cvF(math.Mod(x, y), is32, typed), true
	case ast.BinEq:
		return cvB(x == y), true
	case ast.BinNe:
		return cvB(x != y), true
	case ast.BinLt:
		return cvB(x < y), true
	case ast.BinLe:
		return cvB(x <= y), true
	case ast.BinGt:
		return cvB(x > y), true
	case ast.BinGe:
		return cvB(x >= y), true
	}
	return nil, false
}

func unop(op ast.UnOp, v *cval) (*cval, bool) {
	switch op {
	case ast.UnNeg:
		switch v.k {
		case cvInt:
			return cvI(new(big.Int).Neg(v.n), v.ity, v.typed), true
		case cvFloat:
			return cvF(-v.f, v.f32, v.typed), true
		}
	case ast.UnNot:
		switch v.k {
		case cvBool:
			return cvB(!v.b), true
		case cvInt:
			return cvI(new(big.Int).Not(v.n), v.ity, v.typed), true
		}
	case ast.UnDeref:
		return v, true
	}
	return nil, false
}

// primConst evaluates the builtin associated consts of primitive numeric
// types (`u8::MAX`, `T::BITS` with T = i128, `f64::EPSILON`, ...).
func primConst(self Ty, name string) (*cval, bool) {
	switch self.Kind {
	case TInt:
		it := self.IntKind()
		switch name {
		case "MAX":
			return cvI(intMaxBig(it), it, true), true
		case "MIN":
			return cvI(intMinBig(it), it, true), true
		case "BITS":
			return cvIi(int64(it.Bits()), U32, true), true
		}
	case TFloat:
		is32 := self.Sub == 32
		var f float64
		switch {
		case name == "INFINITY":
			f = math.Inf(1)
		case name == "NEG_INFINITY":
			f = math.Inf(-1)
		case name == "NAN":
			f = math.NaN()
		case name == "MAX" && is32:
			f = math.MaxFloat32
		case name == "MAX":
			f = math.MaxFloat64
		case name == "MIN" && is32:
			f = -math.MaxFloat32
		case name == "MIN":
			f = -math.MaxFloat64
		case name == "MIN_POSITIVE" && is32:
			f = float64(math.Float32frombits(0x00800000))
		case name == "MIN_POSITIVE":
			f = math.Float64frombits(0x0010000000000000)
		case name == "EPSILON" && is32:
			f = float64(math.Float32frombits(0x34000000))
		case name == "EPSILON":
			f = math.Float64frombits(0x3CB0000000000000)
		default:
			return nil, false
		}
		return cvF(f, is32, true), true
	}
	return nil, false
}

// optionVal builds `Some(v)` (or `None` for v == nil).
func (ev *constEval) optionVal(v *cval) (*cval, bool) {
	opt := ev.t.Lang("Option")
	if opt == nil {
		return nil, false
	}
	ev.t.AdtFields(opt)
	if opt.Adt == nil {
		return nil, false
	}
	for _, va := range opt.Adt.Variants {
		if va.Name == "Some" && v != nil {
			return &cval{k: cvStruct, def: va.Def, names: []string{"0"}, elems: []*cval{v}}, true
		}
		if va.Name == "None" && v == nil {
			return &cval{k: cvStruct, def: va.Def}, true
		}
	}
	return nil, false
}

// optionMethod evaluates Option methods on `Some(..)`/`None` values.
func (ev *constEval) optionMethod(v *cval, name string, args []*cval) (*cval, bool) {
	opt := ev.t.Lang("Option")
	if opt == nil || v.def.Parent != opt {
		return nil, false
	}
	isSome := v.def.Name == "Some" && len(v.elems) == 1
	switch name {
	case "is_some":
		return cvB(isSome), true
	case "is_none":
		return cvB(!isSome), true
	case "unwrap", "expect", "unwrap_unchecked":
		if isSome {
			return v.elems[0], true
		}
	case "unwrap_or":
		if len(args) != 1 {
			return nil, false
		}
		if isSome {
			return v.elems[0], true
		}
		return args[0], true
	}
	return nil, false
}

// numMethod evaluates the builtin methods of integers and floats.
func (ev *constEval) numMethod(v *cval, name string, args []*cval) (*cval, bool) {
	if v.k == cvFloat {
		return floatMethod(v, name, args)
	}
	if v.k == cvBool {
		return nil, false
	}
	it, typed := v.ity, v.typed
	x := v.n
	arg := func(i int) (*big.Int, bool) {
		if i >= len(args) {
			return nil, false
		}
		return intOf(args[i])
	}
	// exact results that may overflow the type
	exact := func(name string) (*big.Int, bool) {
		y, ok := arg(0)
		if !ok {
			return nil, false
		}
		r := new(big.Int)
		switch name {
		case "add":
			r.Add(x, y)
		case "sub":
			r.Sub(x, y)
		case "mul":
			r.Mul(x, y)
		case "div", "rem", "div_euclid", "rem_euclid":
			if y.Sign() == 0 {
				return nil, false
			}
			switch name {
			case "div":
				r.Quo(x, y)
			case "rem":
				r.Rem(x, y)
			case "div_euclid":
				q, m := new(big.Int), new(big.Int)
				q.DivMod(x, y, m) // Euclidean
				r = q
			default:
				q, m := new(big.Int), new(big.Int)
				q.DivMod(x, y, m)
				r = m
			}
		case "pow":
			if y.Sign() < 0 || !y.IsInt64() {
				return nil, false
			}
			if typed {
				nb := uint(it.Bits())
				if x.CmpAbs(bigOne) > 0 && int64(x.BitLen()-1)*y.Int64() > int64(nb) {
					// definitely overflows: return a value outside the
					// type's range with the right residue and sign
					m := pow2(nb)
					u := new(big.Int).And(x, new(big.Int).Sub(m, bigOne))
					r.Exp(u, y, m)
					if x.Sign() < 0 && y.Bit(0) == 1 {
						return r.Sub(r, pow2(nb+1)), true
					}
					return r.Add(r, m), true
				}
				return r.Exp(x, y, nil), true
			}
			if y.Int64() > 4096 && x.CmpAbs(bigOne) > 0 {
				return nil, false
			}
			r.Exp(x, y, nil)
		case "shl", "shr":
			if !typed || y.Sign() < 0 || y.Cmp(big.NewInt(int64(it.Bits()))) >= 0 {
				return nil, false
			}
			if name == "shl" {
				r.Lsh(x, uint(y.Int64()))
			} else {
				r.Rsh(x, uint(y.Int64()))
			}
			return wrapInt(r, it), true
		default:
			return nil, false
		}
		return r, true
	}
	if strings.HasPrefix(name, "checked_") || strings.HasPrefix(name, "wrapping_") ||
		strings.HasPrefix(name, "saturating_") || strings.HasPrefix(name, "overflowing_") ||
		strings.HasPrefix(name, "strict_") || strings.HasPrefix(name, "unchecked_") {
		if !typed {
			return nil, false
		}
		i := strings.IndexByte(name, '_')
		kind, op := name[:i], name[i+1:]
		var r *big.Int
		ok := false
		switch op {
		case "neg":
			r, ok = new(big.Int).Neg(x), len(args) == 0
		case "abs":
			r, ok = new(big.Int).Abs(x), len(args) == 0
		default:
			if len(args) != 1 {
				return nil, false
			}
			r, ok = exact(op)
		}
		if !ok {
			if kind == "checked" && (op == "div" || op == "rem" || op == "div_euclid" || op == "rem_euclid" || op == "shl" || op == "shr") {
				return ev.optionVal(nil)
			}
			return nil, false
		}
		fits := inRange(r, it)
		switch kind {
		case "checked":
			if !fits {
				return ev.optionVal(nil)
			}
			return ev.optionVal(cvI(r, it, true))
		case "wrapping":
			return cvI(r, it, true), true
		case "saturating":
			if r.Cmp(intMaxBig(it)) > 0 {
				r = intMaxBig(it)
			} else if r.Cmp(intMinBig(it)) < 0 {
				r = intMinBig(it)
			}
			return cvI(r, it, true), true
		case "overflowing":
			return &cval{k: cvTuple, elems: []*cval{cvI(r, it, true), cvB(!fits)}}, true
		default: // strict_ / unchecked_
			if !fits {
				return nil, false
			}
			return cvI(r, it, true), true
		}
	}
	switch name {
	case "pow", "div_euclid", "rem_euclid":
		r, ok := exact(name)
		if !ok || len(args) != 1 {
			return nil, false
		}
		return cvI(r, it, typed), true
	case "min", "max":
		if len(args) != 1 || args[0].k != cvInt {
			return nil, false
		}
		c := x.Cmp(args[0].n)
		if (c <= 0) == (name == "min") {
			return v, true
		}
		return cvI(args[0].n, it, typed), true
	case "clamp":
		lo, ok1 := arg(0)
		hi, ok2 := arg(1)
		if !ok1 || !ok2 {
			return nil, false
		}
		r := x
		if r.Cmp(lo) < 0 {
			r = lo
		}
		if r.Cmp(hi) > 0 {
			r = hi
		}
		return cvI(r, it, typed), true
	case "abs":
		return cvI(new(big.Int).Abs(x), it, typed), true
	case "unsigned_abs":
		r := new(big.Int).Abs(x)
		if typed && it.Signed() {
			return cvI(r, it+U8, true), true
		}
		return cvI(r, it, typed), true
	case "abs_diff":
		y, ok := arg(0)
		if !ok {
			return nil, false
		}
		r := new(big.Int).Abs(new(big.Int).Sub(x, y))
		if typed && it.Signed() {
			return cvI(r, it+U8, true), true
		}
		return cvI(r, it, typed), true
	case "signum":
		return cvIi(int64(x.Sign()), it, typed), true
	case "is_positive":
		return cvB(x.Sign() > 0), true
	case "is_negative":
		return cvB(x.Sign() < 0), true
	case "is_power_of_two":
		return cvB(x.Sign() > 0 && new(big.Int).And(x, new(big.Int).Sub(x, bigOne)).Sign() == 0), true
	case "next_power_of_two":
		p := big.NewInt(1)
		for p.Cmp(x) < 0 {
			p.Lsh(p, 1)
		}
		return cvI(p, it, typed), true
	case "ilog2", "ilog10", "checked_ilog2", "checked_ilog10", "ilog", "checked_ilog":
		base := int64(2)
		switch name {
		case "ilog10", "checked_ilog10":
			base = 10
		case "ilog", "checked_ilog":
			b, ok := arg(0)
			if !ok || !b.IsInt64() || b.Int64() < 2 {
				return nil, false
			}
			base = b.Int64()
		}
		if x.Sign() <= 0 {
			if strings.HasPrefix(name, "checked_") {
				return ev.optionVal(nil)
			}
			return nil, false
		}
		var n int64
		if base == 2 {
			n = int64(x.BitLen() - 1)
		} else {
			bb := big.NewInt(base)
			p := new(big.Int).Set(bb)
			for p.Cmp(x) <= 0 {
				p.Mul(p, bb)
				n++
			}
		}
		r := cvIi(n, U32, true)
		if strings.HasPrefix(name, "checked_") {
			return ev.optionVal(r)
		}
		return r, true
	case "isqrt":
		if x.Sign() < 0 {
			return nil, false
		}
		return cvI(new(big.Int).Sqrt(x), it, typed), true
	case "div_ceil":
		y, ok := arg(0)
		if !ok || y.Sign() == 0 {
			return nil, false
		}
		q, m := new(big.Int).QuoRem(x, y, new(big.Int))
		if m.Sign() != 0 && (m.Sign() > 0) == (y.Sign() > 0) {
			q.Add(q, bigOne)
		}
		return cvI(q, it, typed), true
	case "to_le", "from_le":
		return v, true
	}
	if !typed {
		return nil, false
	}
	nb := it.Bits()
	u := new(big.Int).And(x, new(big.Int).Sub(pow2(uint(nb)), bigOne)) // unsigned bit pattern
	ones := func(z *big.Int) int {
		c := 0
		for _, w := range z.Bits() {
			c += bits.OnesCount64(uint64(w))
		}
		return c
	}
	inv := new(big.Int).Xor(u, new(big.Int).Sub(pow2(uint(nb)), bigOne))
	switch name {
	case "count_ones":
		return cvIi(int64(ones(u)), U32, true), true
	case "count_zeros":
		return cvIi(int64(nb-ones(u)), U32, true), true
	case "leading_zeros":
		return cvIi(int64(nb-u.BitLen()), U32, true), true
	case "leading_ones":
		return cvIi(int64(nb-inv.BitLen()), U32, true), true
	case "trailing_zeros", "trailing_ones":
		z := u
		if name == "trailing_ones" {
			z = inv
		}
		if z.Sign() == 0 {
			return cvIi(int64(nb), U32, true), true
		}
		return cvIi(int64(z.TrailingZeroBits()), U32, true), true
	case "rotate_left", "rotate_right":
		s, ok := arg(0)
		if !ok || !s.IsInt64() {
			return nil, false
		}
		k := uint(((s.Int64() % int64(nb)) + int64(nb)) % int64(nb))
		if name == "rotate_right" {
			k = (uint(nb) - k) % uint(nb)
		}
		r := new(big.Int).Or(new(big.Int).Lsh(u, k), new(big.Int).Rsh(u, uint(nb)-k))
		return cvI(r, it, true), true
	case "swap_bytes", "to_be", "from_be", "reverse_bits":
		r := new(big.Int)
		if name == "reverse_bits" {
			for i := 0; i < nb; i++ {
				if u.Bit(i) == 1 {
					r.SetBit(r, nb-1-i, 1)
				}
			}
		} else {
			for i := 0; i < nb/8; i++ {
				byt := new(big.Int).And(new(big.Int).Rsh(u, uint(8*i)), big.NewInt(0xff))
				r.Or(r, byt.Lsh(byt, uint(nb-8-8*i)))
			}
		}
		return cvI(r, it, true), true
	}
	return nil, false
}

func floatMethod(v *cval, name string, args []*cval) (*cval, bool) {
	x := v.f
	f := func(r float64) (*cval, bool) { return cvF(r, v.f32, v.typed), true }
	fa := func(i int) (float64, bool) {
		if i >= len(args) || args[i].k != cvFloat {
			return 0, false
		}
		return args[i].f, true
	}
	switch name {
	case "abs":
		return f(math.Abs(x))
	case "floor":
		return f(math.Floor(x))
	case "ceil":
		return f(math.Ceil(x))
	case "round":
		return f(math.Round(x))
	case "round_ties_even":
		return f(math.RoundToEven(x))
	case "trunc":
		return f(math.Trunc(x))
	case "fract":
		return f(x - math.Trunc(x))
	case "sqrt":
		return f(math.Sqrt(x))
	case "signum":
		if math.IsNaN(x) {
			return f(x)
		}
		return f(math.Copysign(1, x))
	case "recip":
		return f(1 / x)
	case "to_degrees":
		return f(x * 180 / math.Pi)
	case "to_radians":
		return f(x * math.Pi / 180)
	case "min", "max":
		y, ok := fa(0)
		if !ok {
			return nil, false
		}
		if name == "min" {
			return f(math.Min(x, y))
		}
		return f(math.Max(x, y))
	case "powi":
		if len(args) != 1 {
			return nil, false
		}
		n, ok := smallInt(args[0])
		if !ok {
			return nil, false
		}
		return f(math.Pow(x, float64(n)))
	case "powf":
		y, ok := fa(0)
		if !ok {
			return nil, false
		}
		return f(math.Pow(x, y))
	case "mul_add":
		a, ok1 := fa(0)
		b, ok2 := fa(1)
		if !ok1 || !ok2 {
			return nil, false
		}
		return f(math.FMA(x, a, b))
	case "is_nan":
		return cvB(math.IsNaN(x)), true
	case "is_infinite":
		return cvB(math.IsInf(x, 0)), true
	case "is_finite":
		return cvB(!math.IsInf(x, 0) && !math.IsNaN(x)), true
	case "is_sign_negative":
		return cvB(math.Signbit(x)), true
	case "is_sign_positive":
		return cvB(!math.Signbit(x)), true
	case "to_bits":
		if !v.typed {
			return nil, false
		}
		if v.f32 {
			return cvIi(int64(math.Float32bits(float32(x))), U32, true), true
		}
		return cvI(new(big.Int).SetUint64(math.Float64bits(x)), U64, true), true
	}
	return nil, false
}

// primAssocFn evaluates `<prim>::f(args)` for builtin functions of numeric
// primitives (`f64::from_bits(x)`, `u64::from(x)`, `u32::max(a, b)`, ...).
func (ev *constEval) primAssocFn(self Ty, name string, args []*cval) (*cval, bool) {
	switch name {
	case "from_bits":
		if self.Kind != TFloat || len(args) != 1 || args[0].k != cvInt {
			return nil, false
		}
		u := new(big.Int).And(args[0].n, new(big.Int).Sub(pow2(64), bigOne)).Uint64()
		if self.Sub == 32 {
			return cvF(float64(math.Float32frombits(uint32(u))), true, true), true
		}
		return cvF(math.Float64frombits(u), false, true), true
	case "from":
		if len(args) != 1 {
			return nil, false
		}
		return castTo(args[0], self)
	}
	if len(args) == 0 {
		return nil, false
	}
	recv, ok := castTo(args[0], self)
	if !ok {
		return nil, false
	}
	return ev.numMethod(recv, name, args[1:])
}
