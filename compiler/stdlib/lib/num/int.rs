//! Inherent methods of the integer types.

use super::{ascii_digit_value, from_str_radix_panic, pie, IntErrorKind, ParseIntError};
use crate::intrinsics;
use crate::str::FromStr;

#[inline(never)]
fn overflow_panic(what: &str) -> ! {
    crate::rt::panic_str(what, file!(), line!(), column!())
}

// Methods shared by signed and unsigned integers.
// $t: the type, $ut: unsigned type of same width, $st: signed type of same
// width, $bits: width, $bytes: width in bytes.
macro_rules! __std_int_common {
    ($t:ident, $ut:ident, $st:ident, $bits:expr, $bytes:expr) => {
        impl $t {
            /// The size of this integer type in bits.
            pub const BITS: u32 = $bits;

            /// Returns the smallest value that can be represented by this integer type.
            #[inline]
            pub const fn min_value() -> $t {
                $t::MIN
            }

            /// Returns the largest value that can be represented by this integer type.
            #[inline]
            pub const fn max_value() -> $t {
                $t::MAX
            }

            /// Returns the number of ones in the binary representation of `self`.
            #[inline]
            pub fn count_ones(self) -> u32 {
                unsafe { intrinsics::ctpop(self) }
            }

            /// Returns the number of zeros in the binary representation of `self`.
            #[inline]
            pub fn count_zeros(self) -> u32 {
                $bits - self.count_ones()
            }

            /// Returns the number of leading zeros in the binary representation of `self`.
            #[inline]
            pub fn leading_zeros(self) -> u32 {
                unsafe { intrinsics::ctlz(self as $ut) }
            }

            /// Returns the number of trailing zeros in the binary representation of `self`.
            #[inline]
            pub fn trailing_zeros(self) -> u32 {
                unsafe { intrinsics::cttz(self) }
            }

            /// Returns the number of leading ones in the binary representation of `self`.
            #[inline]
            pub fn leading_ones(self) -> u32 {
                (!self).leading_zeros()
            }

            /// Returns the number of trailing ones in the binary representation of `self`.
            #[inline]
            pub fn trailing_ones(self) -> u32 {
                (!self).trailing_zeros()
            }

            /// Shifts the bits to the left by a specified amount, `n`, wrapping the truncated bits to the end.
            #[inline]
            pub fn rotate_left(self, n: u32) -> $t {
                unsafe { intrinsics::rotate_left(self, n % $bits) }
            }

            /// Shifts the bits to the right by a specified amount, `n`, wrapping the truncated bits to the beginning.
            #[inline]
            pub fn rotate_right(self, n: u32) -> $t {
                unsafe { intrinsics::rotate_right(self, n % $bits) }
            }

            /// Reverses the byte order of the integer.
            #[inline]
            pub fn swap_bytes(self) -> $t {
                unsafe { intrinsics::bswap(self) }
            }

            /// Reverses the order of bits in the integer.
            #[inline]
            pub fn reverse_bits(self) -> $t {
                let mut x = self as $ut;
                let mut r: $ut = 0;
                let mut i = 0;
                while i < $bits {
                    r = (r << 1) | (x & 1);
                    x = x >> 1;
                    i += 1;
                }
                r as $t
            }

            /// Converts an integer from big endian to the target's endianness.
            #[inline]
            pub fn from_be(x: $t) -> $t {
                x.swap_bytes()
            }

            /// Converts an integer from little endian to the target's endianness.
            #[inline]
            pub fn from_le(x: $t) -> $t {
                x
            }

            /// Converts `self` to big endian from the target's endianness.
            #[inline]
            pub fn to_be(self) -> $t {
                self.swap_bytes()
            }

            /// Converts `self` to little endian from the target's endianness.
            #[inline]
            pub fn to_le(self) -> $t {
                self
            }

            /// Returns the memory representation of this integer as a byte array in little-endian byte order.
            #[inline]
            pub fn to_le_bytes(self) -> [u8; $bytes] {
                let mut out = [0u8; $bytes];
                let x = self as $ut;
                let mut i = 0;
                while i < $bytes {
                    out[i] = (x >> ((8 * i) as u32)) as u8;
                    i += 1;
                }
                out
            }

            /// Returns the memory representation of this integer as a byte array in big-endian byte order.
            #[inline]
            pub fn to_be_bytes(self) -> [u8; $bytes] {
                self.swap_bytes().to_le_bytes()
            }

            /// Returns the memory representation of this integer as a byte array in native byte order.
            #[inline]
            pub fn to_ne_bytes(self) -> [u8; $bytes] {
                self.to_le_bytes()
            }

            /// Creates an integer value from its representation as a byte array in little endian.
            #[inline]
            pub fn from_le_bytes(bytes: [u8; $bytes]) -> $t {
                let mut x: $ut = 0;
                let mut i = 0;
                while i < $bytes {
                    x = x | ((bytes[i] as $ut) << ((8 * i) as u32));
                    i += 1;
                }
                x as $t
            }

            /// Creates an integer value from its representation as a byte array in big endian.
            #[inline]
            pub fn from_be_bytes(bytes: [u8; $bytes]) -> $t {
                $t::from_le_bytes(bytes).swap_bytes()
            }

            /// Creates an integer value from its memory representation as a byte array in native endianness.
            #[inline]
            pub fn from_ne_bytes(bytes: [u8; $bytes]) -> $t {
                $t::from_le_bytes(bytes)
            }

            /// Checked integer addition.
            #[inline]
            pub fn checked_add(self, rhs: $t) -> Option<$t> {
                let (a, b) = self.overflowing_add(rhs);
                if b { None } else { Some(a) }
            }

            /// Checked integer subtraction.
            #[inline]
            pub fn checked_sub(self, rhs: $t) -> Option<$t> {
                let (a, b) = self.overflowing_sub(rhs);
                if b { None } else { Some(a) }
            }

            /// Checked integer multiplication.
            #[inline]
            pub fn checked_mul(self, rhs: $t) -> Option<$t> {
                let (a, b) = self.overflowing_mul(rhs);
                if b { None } else { Some(a) }
            }

            /// Checked integer division.
            #[inline]
            pub fn checked_div(self, rhs: $t) -> Option<$t> {
                if rhs == 0 || (self == $t::MIN && $t::MIN != 0 && rhs == (0 as $t).wrapping_sub(1)) {
                    None
                } else {
                    Some(unsafe { intrinsics::unchecked_div(self, rhs) })
                }
            }

            /// Checked integer remainder.
            #[inline]
            pub fn checked_rem(self, rhs: $t) -> Option<$t> {
                if rhs == 0 || (self == $t::MIN && $t::MIN != 0 && rhs == (0 as $t).wrapping_sub(1)) {
                    None
                } else {
                    Some(unsafe { intrinsics::unchecked_rem(self, rhs) })
                }
            }

            /// Checked Euclidean division.
            #[inline]
            pub fn checked_div_euclid(self, rhs: $t) -> Option<$t> {
                if rhs == 0 || (self == $t::MIN && $t::MIN != 0 && rhs == (0 as $t).wrapping_sub(1)) {
                    None
                } else {
                    Some(self.div_euclid(rhs))
                }
            }

            /// Checked Euclidean remainder.
            #[inline]
            pub fn checked_rem_euclid(self, rhs: $t) -> Option<$t> {
                if rhs == 0 || (self == $t::MIN && $t::MIN != 0 && rhs == (0 as $t).wrapping_sub(1)) {
                    None
                } else {
                    Some(self.rem_euclid(rhs))
                }
            }

            /// Checked negation.
            #[inline]
            pub fn checked_neg(self) -> Option<$t> {
                let (a, b) = self.overflowing_neg();
                if b { None } else { Some(a) }
            }

            /// Checked shift left.
            #[inline]
            pub fn checked_shl(self, rhs: u32) -> Option<$t> {
                if rhs < $bits {
                    Some(unsafe { intrinsics::unchecked_shl(self, rhs) })
                } else {
                    None
                }
            }

            /// Checked shift right.
            #[inline]
            pub fn checked_shr(self, rhs: u32) -> Option<$t> {
                if rhs < $bits {
                    Some(unsafe { intrinsics::unchecked_shr(self, rhs) })
                } else {
                    None
                }
            }

            /// Checked exponentiation.
            #[inline]
            pub fn checked_pow(self, exp: u32) -> Option<$t> {
                let (a, b) = self.overflowing_pow(exp);
                if b { None } else { Some(a) }
            }

            /// Saturating integer multiplication.
            #[inline]
            pub fn saturating_mul(self, rhs: $t) -> $t {
                match self.checked_mul(rhs) {
                    Some(x) => x,
                    None => {
                        if (self < (0 as $t)) != (rhs < (0 as $t)) {
                            $t::MIN
                        } else {
                            $t::MAX
                        }
                    }
                }
            }

            /// Saturating integer division.
            #[inline]
            pub fn saturating_div(self, rhs: $t) -> $t {
                match self.overflowing_div(rhs) {
                    (result, false) => result,
                    (_result, true) => $t::MAX,
                }
            }

            /// Saturating integer exponentiation.
            #[inline]
            pub fn saturating_pow(self, exp: u32) -> $t {
                match self.checked_pow(exp) {
                    Some(x) => x,
                    None => {
                        if self < (0 as $t) && exp % 2 == 1 {
                            $t::MIN
                        } else {
                            $t::MAX
                        }
                    }
                }
            }

            /// Wrapping (modular) addition.
            #[inline]
            pub fn wrapping_add(self, rhs: $t) -> $t {
                unsafe { intrinsics::wrapping_add(self, rhs) }
            }

            /// Wrapping (modular) subtraction.
            #[inline]
            pub fn wrapping_sub(self, rhs: $t) -> $t {
                unsafe { intrinsics::wrapping_sub(self, rhs) }
            }

            /// Wrapping (modular) multiplication.
            #[inline]
            pub fn wrapping_mul(self, rhs: $t) -> $t {
                unsafe { intrinsics::wrapping_mul(self, rhs) }
            }

            /// Wrapping (modular) division.
            #[inline]
            pub fn wrapping_div(self, rhs: $t) -> $t {
                self.overflowing_div(rhs).0
            }

            /// Wrapping (modular) remainder.
            #[inline]
            pub fn wrapping_rem(self, rhs: $t) -> $t {
                self.overflowing_rem(rhs).0
            }

            /// Wrapping Euclidean division.
            #[inline]
            pub fn wrapping_div_euclid(self, rhs: $t) -> $t {
                if self == $t::MIN && $t::MIN != 0 && rhs == (0 as $t).wrapping_sub(1) {
                    self
                } else {
                    self.div_euclid(rhs)
                }
            }

            /// Wrapping Euclidean remainder.
            #[inline]
            pub fn wrapping_rem_euclid(self, rhs: $t) -> $t {
                if self == $t::MIN && $t::MIN != 0 && rhs == (0 as $t).wrapping_sub(1) {
                    0
                } else {
                    self.rem_euclid(rhs)
                }
            }

            /// Wrapping (modular) negation.
            #[inline]
            pub fn wrapping_neg(self) -> $t {
                (0 as $t).wrapping_sub(self)
            }

            /// Panic-free bitwise shift-left; yields `self << mask(rhs)`.
            #[inline]
            pub fn wrapping_shl(self, rhs: u32) -> $t {
                unsafe { intrinsics::unchecked_shl(self, rhs & ($bits - 1)) }
            }

            /// Panic-free bitwise shift-right; yields `self >> mask(rhs)`.
            #[inline]
            pub fn wrapping_shr(self, rhs: u32) -> $t {
                unsafe { intrinsics::unchecked_shr(self, rhs & ($bits - 1)) }
            }

            /// Wrapping (modular) exponentiation.
            #[inline]
            pub fn wrapping_pow(self, exp: u32) -> $t {
                self.overflowing_pow(exp).0
            }

            /// Calculates `self + rhs`, returning whether an arithmetic overflow would occur.
            #[inline]
            pub fn overflowing_add(self, rhs: $t) -> ($t, bool) {
                unsafe { intrinsics::add_with_overflow(self, rhs) }
            }

            /// Calculates `self - rhs`, returning whether an arithmetic overflow would occur.
            #[inline]
            pub fn overflowing_sub(self, rhs: $t) -> ($t, bool) {
                unsafe { intrinsics::sub_with_overflow(self, rhs) }
            }

            /// Calculates `self * rhs`, returning whether an arithmetic overflow would occur.
            #[inline]
            pub fn overflowing_mul(self, rhs: $t) -> ($t, bool) {
                unsafe { intrinsics::mul_with_overflow(self, rhs) }
            }

            /// Calculates `self / rhs`, returning whether an arithmetic overflow would occur.
            #[inline]
            pub fn overflowing_div(self, rhs: $t) -> ($t, bool) {
                if self == $t::MIN && $t::MIN != 0 && rhs == (0 as $t).wrapping_sub(1) {
                    (self, true)
                } else {
                    (self / rhs, false)
                }
            }

            /// Calculates the remainder of `self / rhs`, returning whether an arithmetic overflow would occur.
            #[inline]
            pub fn overflowing_rem(self, rhs: $t) -> ($t, bool) {
                if self == $t::MIN && $t::MIN != 0 && rhs == (0 as $t).wrapping_sub(1) {
                    (0, true)
                } else {
                    (self % rhs, false)
                }
            }

            /// Negates self in a wrapping fashion, returning whether an overflow occurred.
            #[inline]
            pub fn overflowing_neg(self) -> ($t, bool) {
                if $t::MIN == 0 {
                    (self.wrapping_neg(), self != 0)
                } else if self == $t::MIN {
                    ($t::MIN, true)
                } else {
                    (self.wrapping_neg(), false)
                }
            }

            /// Shifts self left by `rhs` bits, returning whether the shift amount was too large.
            #[inline]
            pub fn overflowing_shl(self, rhs: u32) -> ($t, bool) {
                (self.wrapping_shl(rhs), rhs >= $bits)
            }

            /// Shifts self right by `rhs` bits, returning whether the shift amount was too large.
            #[inline]
            pub fn overflowing_shr(self, rhs: u32) -> ($t, bool) {
                (self.wrapping_shr(rhs), rhs >= $bits)
            }

            /// Raises self to the power of `exp`, returning whether an overflow occurred.
            #[inline]
            pub fn overflowing_pow(self, mut exp: u32) -> ($t, bool) {
                if exp == 0 {
                    return (1, false);
                }
                let mut base = self;
                let mut acc: $t = 1;
                let mut overflown = false;
                loop {
                    if (exp & 1) == 1 {
                        let r = acc.overflowing_mul(base);
                        acc = r.0;
                        overflown = overflown || r.1;
                        if exp == 1 {
                            return (acc, overflown);
                        }
                    }
                    exp /= 2;
                    let r = base.overflowing_mul(base);
                    base = r.0;
                    overflown = overflown || r.1;
                }
            }

            /// Raises self to the power of `exp`, using exponentiation by squaring.
            #[inline]
            pub fn pow(self, mut exp: u32) -> $t {
                if exp == 0 {
                    return 1;
                }
                let mut base = self;
                let mut acc: $t = 1;
                loop {
                    if (exp & 1) == 1 {
                        acc = match acc.checked_mul(base) {
                            Some(v) => v,
                            None => overflow_panic("attempt to multiply with overflow"),
                        };
                        if exp == 1 {
                            return acc;
                        }
                    }
                    exp /= 2;
                    base = match base.checked_mul(base) {
                        Some(v) => v,
                        None => overflow_panic("attempt to multiply with overflow"),
                    };
                }
            }

            /// Returns the logarithm of the number with respect to an arbitrary base, rounded down.
            #[inline]
            pub fn ilog(self, base: $t) -> u32 {
                match self.checked_ilog(base) {
                    Some(v) => v,
                    None => overflow_panic("argument of integer logarithm must be positive"),
                }
            }

            /// Returns the base 2 logarithm of the number, rounded down.
            #[inline]
            pub fn ilog2(self) -> u32 {
                match self.checked_ilog2() {
                    Some(v) => v,
                    None => overflow_panic("argument of integer logarithm must be positive"),
                }
            }

            /// Returns the base 10 logarithm of the number, rounded down.
            #[inline]
            pub fn ilog10(self) -> u32 {
                match self.checked_ilog10() {
                    Some(v) => v,
                    None => overflow_panic("argument of integer logarithm must be positive"),
                }
            }

            /// Returns the logarithm of the number with respect to an arbitrary base, rounded down.
            #[inline]
            pub fn checked_ilog(self, base: $t) -> Option<u32> {
                if self <= (0 as $t) || base < (2 as $t) {
                    None
                } else {
                    let mut n = 0;
                    let mut r = self;
                    while r >= base {
                        r = r / base;
                        n += 1;
                    }
                    Some(n)
                }
            }

            /// Returns the base 2 logarithm of the number, rounded down.
            #[inline]
            pub fn checked_ilog2(self) -> Option<u32> {
                if self <= (0 as $t) {
                    None
                } else {
                    Some($bits - 1 - self.leading_zeros())
                }
            }

            /// Returns the base 10 logarithm of the number, rounded down.
            #[inline]
            pub fn checked_ilog10(self) -> Option<u32> {
                self.checked_ilog(10)
            }

            /// Calculates the quotient of Euclidean division of `self` by `rhs`.
            #[inline]
            pub fn div_euclid(self, rhs: $t) -> $t {
                let q = self / rhs;
                if self % rhs < (0 as $t) {
                    return if rhs > (0 as $t) { q - 1 } else { q + 1 };
                }
                q
            }

            /// Calculates the least nonnegative remainder of `self (mod rhs)`.
            #[inline]
            pub fn rem_euclid(self, rhs: $t) -> $t {
                let r = self % rhs;
                if r < (0 as $t) {
                    if rhs < (0 as $t) {
                        r.wrapping_sub(rhs)
                    } else {
                        r.wrapping_add(rhs)
                    }
                } else {
                    r
                }
            }

            /// Converts a string slice in a given base to an integer.
            pub fn from_str_radix(src: &str, radix: u32) -> Result<$t, ParseIntError> {
                if radix < 2 || radix > 36 {
                    from_str_radix_panic(radix);
                }
                let bytes = src.as_bytes();
                let len = bytes.len();
                if len == 0 {
                    return Err(pie(IntErrorKind::Empty));
                }
                let is_signed = $t::MIN != 0;
                let mut i = 0;
                let mut negative = false;
                let first = bytes[0];
                if first == b'+' || (first == b'-' && is_signed) {
                    if len == 1 {
                        return Err(pie(IntErrorKind::InvalidDigit));
                    }
                    negative = first == b'-';
                    i = 1;
                }
                let mut result: $t = 0;
                let r = radix as $t;
                while i < len {
                    let d = match ascii_digit_value(bytes[i], radix) {
                        Some(d) => d as $t,
                        None => return Err(pie(IntErrorKind::InvalidDigit)),
                    };
                    let (m, o1) = result.overflowing_mul(r);
                    if o1 {
                        return Err(pie(if negative { IntErrorKind::NegOverflow } else { IntErrorKind::PosOverflow }));
                    }
                    let (s, o2) = if negative { m.overflowing_sub(d) } else { m.overflowing_add(d) };
                    if o2 {
                        return Err(pie(if negative { IntErrorKind::NegOverflow } else { IntErrorKind::PosOverflow }));
                    }
                    result = s;
                    i += 1;
                }
                Ok(result)
            }
        }

        impl FromStr for $t {
            type Err = ParseIntError;
            #[inline]
            fn from_str(src: &str) -> Result<$t, ParseIntError> {
                $t::from_str_radix(src, 10)
            }
        }
    };
}

macro_rules! __std_int_signed {
    ($t:ident, $ut:ident, $bits:expr, $bytes:expr) => {
        __std_int_common! { $t, $ut, $t, $bits, $bytes }

        impl $t {
            /// The smallest value that can be represented by this integer type.
            pub const MIN: $t = !$t::MAX;
            /// The largest value that can be represented by this integer type.
            pub const MAX: $t = ($ut::MAX >> 1) as $t;

            /// Calculates the middle point of `self` and `rhs`, rounding towards zero.
            #[inline]
            pub fn midpoint(self, rhs: $t) -> $t {
                let s = (self & rhs) + ((self ^ rhs) >> 1);
                if s < 0 && ((self ^ rhs) & 1) != 0 {
                    s + 1
                } else {
                    s
                }
            }

            /// Computes the absolute value of `self`.
            #[inline]
            pub fn abs(self) -> $t {
                if self < 0 {
                    if self == $t::MIN {
                        overflow_panic("attempt to negate with overflow");
                    }
                    -self
                } else {
                    self
                }
            }

            /// Computes the absolute value of `self` without any wrapping or panicking.
            #[inline]
            pub fn unsigned_abs(self) -> $ut {
                if self < 0 {
                    (0 as $ut).wrapping_sub(self as $ut)
                } else {
                    self as $ut
                }
            }

            /// Checked absolute value.
            #[inline]
            pub fn checked_abs(self) -> Option<$t> {
                if self == $t::MIN {
                    None
                } else {
                    Some(self.abs())
                }
            }

            /// Wrapping (modular) absolute value.
            #[inline]
            pub fn wrapping_abs(self) -> $t {
                if self < 0 {
                    self.wrapping_neg()
                } else {
                    self
                }
            }

            /// Saturating absolute value.
            #[inline]
            pub fn saturating_abs(self) -> $t {
                if self == $t::MIN {
                    $t::MAX
                } else {
                    self.abs()
                }
            }

            /// Saturating integer negation.
            #[inline]
            pub fn saturating_neg(self) -> $t {
                if self == $t::MIN {
                    $t::MAX
                } else {
                    -self
                }
            }

            /// Computes the absolute value of `self`, returning whether an overflow occurred.
            #[inline]
            pub fn overflowing_abs(self) -> ($t, bool) {
                (self.wrapping_abs(), self == $t::MIN)
            }

            /// Computes the absolute difference between `self` and `other`.
            #[inline]
            pub fn abs_diff(self, other: $t) -> $ut {
                if self < other {
                    (other as $ut).wrapping_sub(self as $ut)
                } else {
                    (self as $ut).wrapping_sub(other as $ut)
                }
            }

            /// Returns a number representing sign of `self`.
            #[inline]
            pub const fn signum(self) -> $t {
                if self < 0 {
                    -1
                } else if self == 0 {
                    0
                } else {
                    1
                }
            }

            /// Returns `true` if `self` is positive.
            #[inline]
            pub const fn is_positive(self) -> bool {
                self > 0
            }

            /// Returns `true` if `self` is negative.
            #[inline]
            pub const fn is_negative(self) -> bool {
                self < 0
            }

            /// Saturating integer addition.
            #[inline]
            pub fn saturating_add(self, rhs: $t) -> $t {
                match self.checked_add(rhs) {
                    Some(x) => x,
                    None => {
                        if rhs > 0 {
                            $t::MAX
                        } else {
                            $t::MIN
                        }
                    }
                }
            }

            /// Saturating integer subtraction.
            #[inline]
            pub fn saturating_sub(self, rhs: $t) -> $t {
                match self.checked_sub(rhs) {
                    Some(x) => x,
                    None => {
                        if rhs > 0 {
                            $t::MIN
                        } else {
                            $t::MAX
                        }
                    }
                }
            }

            /// Checked addition with an unsigned integer.
            #[inline]
            pub fn checked_add_unsigned(self, rhs: $ut) -> Option<$t> {
                let r = self.wrapping_add(rhs as $t);
                if (r < self) != ((rhs as $t) < 0) {
                    None
                } else {
                    Some(r)
                }
            }

            /// Returns the square root of the number, rounded down.
            #[inline]
            pub fn isqrt(self) -> $t {
                if self < 0 {
                    overflow_panic("argument of integer square root cannot be negative");
                }
                (self as $ut).isqrt() as $t
            }

            /// Returns the square root of the number, rounded down, or `None` if negative.
            #[inline]
            pub fn checked_isqrt(self) -> Option<$t> {
                if self < 0 {
                    None
                } else {
                    Some((self as $ut).isqrt() as $t)
                }
            }
        }
    };
}

macro_rules! __std_int_unsigned {
    ($t:ident, $st:ident, $bits:expr, $bytes:expr) => {
        __std_int_common! { $t, $t, $st, $bits, $bytes }

        impl $t {
            /// The smallest value that can be represented by this integer type.
            pub const MIN: $t = 0;
            /// The largest value that can be represented by this integer type.
            pub const MAX: $t = !0;

            /// Calculates the middle point of `self` and `rhs`, rounding down.
            #[inline]
            pub fn midpoint(self, rhs: $t) -> $t {
                (self & rhs) + ((self ^ rhs) >> 1)
            }

            /// Computes the absolute difference between `self` and `other`.
            #[inline]
            pub fn abs_diff(self, other: $t) -> $t {
                if self < other {
                    other - self
                } else {
                    self - other
                }
            }

            /// Saturating integer addition.
            #[inline]
            pub fn saturating_add(self, rhs: $t) -> $t {
                match self.checked_add(rhs) {
                    Some(x) => x,
                    None => $t::MAX,
                }
            }

            /// Saturating integer subtraction.
            #[inline]
            pub fn saturating_sub(self, rhs: $t) -> $t {
                match self.checked_sub(rhs) {
                    Some(x) => x,
                    None => 0,
                }
            }

            /// Checked addition with a signed integer.
            #[inline]
            pub fn checked_add_signed(self, rhs: $st) -> Option<$t> {
                if rhs >= 0 {
                    self.checked_add(rhs as $t)
                } else {
                    self.checked_sub(rhs.unsigned_abs())
                }
            }

            /// Wrapping addition with a signed integer.
            #[inline]
            pub fn wrapping_add_signed(self, rhs: $st) -> $t {
                self.wrapping_add(rhs as $t)
            }

            /// Saturating addition with a signed integer.
            #[inline]
            pub fn saturating_add_signed(self, rhs: $st) -> $t {
                if rhs >= 0 {
                    self.saturating_add(rhs as $t)
                } else {
                    self.saturating_sub(rhs.unsigned_abs())
                }
            }

            /// Checked subtraction with a signed integer.
            #[inline]
            pub fn checked_signed_diff(self, rhs: $t) -> Option<$st> {
                let res = self.wrapping_sub(rhs) as $st;
                let overflow = (self >= rhs) == (res < 0);
                if !overflow {
                    Some(res)
                } else {
                    None
                }
            }

            /// Returns `true` if and only if `self == 2^k` for some `k`.
            #[inline]
            pub const fn is_power_of_two(self) -> bool {
                self != 0 && (self & (self - 1)) == 0
            }

            /// Returns the smallest power of two greater than or equal to `self`.
            #[inline]
            pub fn next_power_of_two(self) -> $t {
                match self.checked_next_power_of_two() {
                    Some(v) => v,
                    None => overflow_panic("attempt to add with overflow"),
                }
            }

            /// Returns the smallest power of two greater than or equal to `self`, or `None` on overflow.
            #[inline]
            pub fn checked_next_power_of_two(self) -> Option<$t> {
                if self <= 1 {
                    return Some(1);
                }
                let p = self - 1;
                let z = p.leading_zeros();
                if z == 0 {
                    None
                } else {
                    Some((1 as $t) << ($bits - z))
                }
            }

            /// Calculates the quotient of `self` and `rhs`, rounding the result towards positive infinity.
            #[inline]
            pub fn div_ceil(self, rhs: $t) -> $t {
                let d = self / rhs;
                let r = self % rhs;
                if r > 0 {
                    d + 1
                } else {
                    d
                }
            }

            /// Calculates the smallest value greater than or equal to `self` that is a multiple of `rhs`.
            #[inline]
            pub fn next_multiple_of(self, rhs: $t) -> $t {
                match self % rhs {
                    0 => self,
                    r => self + (rhs - r),
                }
            }

            /// Returns `true` if `self` is an integer multiple of `rhs`.
            #[inline]
            pub fn is_multiple_of(self, rhs: $t) -> bool {
                if rhs == 0 {
                    self == 0
                } else {
                    self % rhs == 0
                }
            }

            /// Returns the square root of the number, rounded down.
            #[inline]
            pub fn isqrt(self) -> $t {
                if self < 2 {
                    return self;
                }
                // Bitwise digit-by-digit method.
                let mut op = self;
                let mut res: $t = 0;
                let mut one: $t = (1 as $t) << (($bits - 2) as u32);
                while one > op {
                    one = one >> 2;
                }
                while one != 0 {
                    if op >= res + one {
                        op = op - (res + one);
                        res = (res >> 1) + one;
                    } else {
                        res = res >> 1;
                    }
                    one = one >> 2;
                }
                res
            }

            /// Returns the square root of the number, rounded down.
            #[inline]
            pub fn checked_isqrt(self) -> Option<$t> {
                Some(self.isqrt())
            }
        }
    };
}

__std_int_signed! { i8, u8, 8, 1 }
__std_int_signed! { i16, u16, 16, 2 }
__std_int_signed! { i32, u32, 32, 4 }
__std_int_signed! { i64, u64, 64, 8 }
__std_int_signed! { i128, u128, 128, 16 }
__std_int_signed! { isize, usize, 64, 8 }
__std_int_unsigned! { u8, i8, 8, 1 }
__std_int_unsigned! { u16, i16, 16, 2 }
__std_int_unsigned! { u32, i32, 32, 4 }
__std_int_unsigned! { u64, i64, 64, 8 }
__std_int_unsigned! { u128, i128, 128, 16 }
__std_int_unsigned! { usize, isize, 64, 8 }

// ---------------------------------------------------------------------------
// ASCII helpers on u8

impl u8 {
    /// Checks if the value is within the ASCII range.
    #[inline]
    pub const fn is_ascii(&self) -> bool {
        *self < 128
    }

    /// Makes a copy of the value in its ASCII upper case equivalent.
    #[inline]
    pub const fn to_ascii_uppercase(&self) -> u8 {
        if *self >= b'a' && *self <= b'z' {
            *self - 32
        } else {
            *self
        }
    }

    /// Makes a copy of the value in its ASCII lower case equivalent.
    #[inline]
    pub const fn to_ascii_lowercase(&self) -> u8 {
        if *self >= b'A' && *self <= b'Z' {
            *self + 32
        } else {
            *self
        }
    }

    /// Checks that two values are an ASCII case-insensitive match.
    #[inline]
    pub const fn eq_ignore_ascii_case(&self, other: &u8) -> bool {
        self.to_ascii_lowercase() == other.to_ascii_lowercase()
    }

    /// Converts this value to its ASCII upper case equivalent in-place.
    #[inline]
    pub fn make_ascii_uppercase(&mut self) {
        *self = self.to_ascii_uppercase();
    }

    /// Converts this value to its ASCII lower case equivalent in-place.
    #[inline]
    pub fn make_ascii_lowercase(&mut self) {
        *self = self.to_ascii_lowercase();
    }

    /// Checks if the value is an ASCII alphabetic character.
    #[inline]
    pub const fn is_ascii_alphabetic(&self) -> bool {
        (*self >= b'A' && *self <= b'Z') || (*self >= b'a' && *self <= b'z')
    }

    /// Checks if the value is an ASCII uppercase character.
    #[inline]
    pub const fn is_ascii_uppercase(&self) -> bool {
        *self >= b'A' && *self <= b'Z'
    }

    /// Checks if the value is an ASCII lowercase character.
    #[inline]
    pub const fn is_ascii_lowercase(&self) -> bool {
        *self >= b'a' && *self <= b'z'
    }

    /// Checks if the value is an ASCII alphanumeric character.
    #[inline]
    pub const fn is_ascii_alphanumeric(&self) -> bool {
        self.is_ascii_alphabetic() || self.is_ascii_digit()
    }

    /// Checks if the value is an ASCII decimal digit.
    #[inline]
    pub const fn is_ascii_digit(&self) -> bool {
        *self >= b'0' && *self <= b'9'
    }

    /// Checks if the value is an ASCII octal digit.
    #[inline]
    pub const fn is_ascii_octdigit(&self) -> bool {
        *self >= b'0' && *self <= b'7'
    }

    /// Checks if the value is an ASCII hexadecimal digit.
    #[inline]
    pub const fn is_ascii_hexdigit(&self) -> bool {
        self.is_ascii_digit() || (*self >= b'A' && *self <= b'F') || (*self >= b'a' && *self <= b'f')
    }

    /// Checks if the value is an ASCII punctuation character.
    #[inline]
    pub const fn is_ascii_punctuation(&self) -> bool {
        (*self >= b'!' && *self <= b'/')
            || (*self >= b':' && *self <= b'@')
            || (*self >= b'[' && *self <= b'`')
            || (*self >= b'{' && *self <= b'~')
    }

    /// Checks if the value is an ASCII graphic character.
    #[inline]
    pub const fn is_ascii_graphic(&self) -> bool {
        *self >= b'!' && *self <= b'~'
    }

    /// Checks if the value is an ASCII whitespace character.
    #[inline]
    pub const fn is_ascii_whitespace(&self) -> bool {
        *self == b'\t' || *self == b'\n' || *self == b'\x0C' || *self == b'\r' || *self == b' '
    }

    /// Checks if the value is an ASCII control character.
    #[inline]
    pub const fn is_ascii_control(&self) -> bool {
        *self < 32 || *self == 127
    }
}
