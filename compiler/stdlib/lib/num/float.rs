//! Inherent methods of `f32` and `f64`.

use crate::cmp::Ordering;
use crate::intrinsics;

/// A classification of floating point numbers.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FpCategory {
    /// NaN (not a number).
    Nan,
    /// Positive or negative infinity.
    Infinite,
    /// Positive or negative zero.
    Zero,
    /// "Subnormal" or "denormal" floating point representation.
    Subnormal,
    /// A regular floating point number.
    Normal,
}

mod f64_math {
    pub use crate::intrinsics::{
        acosf64 as acos, asinf64 as asin, atan2f64 as atan2, atanf64 as atan, cbrtf64 as cbrt, ceilf64 as ceil,
        copysignf64 as copysign, cosf64 as cos, coshf64 as cosh, exp2f64 as exp2, expf64 as exp, fabsf64 as fabs,
        floorf64 as floor, fmaf64 as fma, hypotf64 as hypot, log10f64 as log10, log2f64 as log2, logf64 as log,
        powff64 as powf, powif64 as powi, roundf64 as round, sinf64 as sin, sinhf64 as sinh, sqrtf64 as sqrt,
        tanf64 as tan, tanhf64 as tanh, truncf64 as trunc, expm1f64 as expm1, ln1pf64 as ln1p, asinhf64 as asinh, acoshf64 as acosh,
    };
}

mod f32_math {
    pub use crate::intrinsics::{
        acosf32 as acos, asinf32 as asin, atan2f32 as atan2, atanf32 as atan, cbrtf32 as cbrt, ceilf32 as ceil,
        copysignf32 as copysign, cosf32 as cos, coshf32 as cosh, exp2f32 as exp2, expf32 as exp, fabsf32 as fabs,
        floorf32 as floor, fmaf32 as fma, hypotf32 as hypot, log10f32 as log10, log2f32 as log2, logf32 as log,
        powff32 as powf, powif32 as powi, roundf32 as round, sinf32 as sin, sinhf32 as sinh, sqrtf32 as sqrt,
        tanf32 as tan, tanhf32 as tanh, truncf32 as trunc, expm1f32 as expm1, ln1pf32 as ln1p, asinhf32 as asinh, acoshf32 as acosh,
    };
}

#[inline(never)]
fn clamp_failed<T: crate::fmt::Debug>(min: T, max: T) -> ! {
    crate::rt::panic_fmt(
        format_args!("min > max, or either was NaN. min = {min:?}, max = {max:?}"),
        file!(),
        line!(),
        column!(),
    )
}

// $t: float type, $bits: unsigned type of same width, $sbits: signed type
// of same width, $m: module with the math intrinsics, $nbits: width,
// $mant: number of explicit mantissa bits, $bytes: width in bytes.
// Reinterprets the bits of a value (usable in const fns, unlike transmute).
union FloatBits<A: Copy, B: Copy> {
    a: A,
    b: B,
}

macro_rules! __std_float_impl {
    ($t:ident, $bits:ident, $sbits:ident, $m:ident, $nbits:expr, $mant:expr, $bytes:expr) => {
        impl $t {
            /// Returns `true` if this value is NaN.
            #[inline]
            pub const fn is_nan(self) -> bool {
                self != self
            }

            /// Returns `true` if this value is positive infinity or negative infinity.
            #[inline]
            pub fn is_infinite(self) -> bool {
                self == $t::INFINITY || self == $t::NEG_INFINITY
            }

            /// Returns `true` if this number is neither infinite nor NaN.
            #[inline]
            pub fn is_finite(self) -> bool {
                self.abs() < $t::INFINITY
            }

            /// Returns `true` if the number is subnormal.
            #[inline]
            pub fn is_subnormal(self) -> bool {
                match self.classify() {
                    FpCategory::Subnormal => true,
                    _ => false,
                }
            }

            /// Returns `true` if the number is neither zero, infinite, subnormal, or NaN.
            #[inline]
            pub fn is_normal(self) -> bool {
                match self.classify() {
                    FpCategory::Normal => true,
                    _ => false,
                }
            }

            /// Returns the floating point category of the number.
            pub fn classify(self) -> FpCategory {
                let bits = self.to_bits();
                let exp_mask: $bits = ((1 as $bits) << (($nbits - $mant - 1) as u32)) - 1;
                let exp = (bits >> ($mant as u32)) & exp_mask;
                let man = bits & (((1 as $bits) << ($mant as u32)) - 1);
                if exp == 0 {
                    if man == 0 {
                        FpCategory::Zero
                    } else {
                        FpCategory::Subnormal
                    }
                } else if exp == exp_mask {
                    if man == 0 {
                        FpCategory::Infinite
                    } else {
                        FpCategory::Nan
                    }
                } else {
                    FpCategory::Normal
                }
            }

            /// Returns `true` if `self` has a positive sign (including `+0.0` and positive NaNs).
            #[inline]
            pub fn is_sign_positive(self) -> bool {
                !self.is_sign_negative()
            }

            /// Returns `true` if `self` has a negative sign (including `-0.0` and negative NaNs).
            #[inline]
            pub fn is_sign_negative(self) -> bool {
                (self.to_bits() >> (($nbits - 1) as u32)) != 0
            }

            /// Takes the reciprocal (inverse) of a number, `1/x`.
            #[inline]
            pub fn recip(self) -> $t {
                1.0 / self
            }

            /// Converts radians to degrees.
            #[inline]
            pub fn to_degrees(self) -> $t {
                if $nbits == 32 {
                    // std uses the correctly rounded literal for f32
                    return self * (57.2957795130823208767981548141051703_f64 as $t);
                }
                self * (180.0 / crate::$t::consts::PI)
            }

            /// Converts degrees to radians.
            #[inline]
            pub fn to_radians(self) -> $t {
                self * (crate::$t::consts::PI / 180.0)
            }

            /// Returns the maximum of the two numbers, ignoring NaN.
            #[inline]
            pub fn max(self, other: $t) -> $t {
                if self.is_nan() {
                    other
                } else if other.is_nan() {
                    self
                } else if self > other {
                    self
                } else if other > self {
                    other
                } else if self.is_sign_negative() {
                    other
                } else {
                    self
                }
            }

            /// Returns the minimum of the two numbers, ignoring NaN.
            #[inline]
            pub fn min(self, other: $t) -> $t {
                if self.is_nan() {
                    other
                } else if other.is_nan() {
                    self
                } else if self < other {
                    self
                } else if other < self {
                    other
                } else if self.is_sign_negative() {
                    self
                } else {
                    other
                }
            }

            /// Returns the maximum of the two numbers, propagating NaN.
            #[inline]
            pub fn maximum(self, other: $t) -> $t {
                if self.is_nan() || other.is_nan() {
                    self + other
                } else {
                    self.max(other)
                }
            }

            /// Returns the minimum of the two numbers, propagating NaN.
            #[inline]
            pub fn minimum(self, other: $t) -> $t {
                if self.is_nan() || other.is_nan() {
                    self + other
                } else {
                    self.min(other)
                }
            }

            /// Float addition that allows optimizations based on algebraic rules.
            #[inline]
            pub fn algebraic_add(self, rhs: $t) -> $t {
                self + rhs
            }

            /// Float subtraction that allows optimizations based on algebraic rules.
            #[inline]
            pub fn algebraic_sub(self, rhs: $t) -> $t {
                self - rhs
            }

            /// Float multiplication that allows optimizations based on algebraic rules.
            #[inline]
            pub fn algebraic_mul(self, rhs: $t) -> $t {
                self * rhs
            }

            /// Float division that allows optimizations based on algebraic rules.
            #[inline]
            pub fn algebraic_div(self, rhs: $t) -> $t {
                self / rhs
            }

            /// Float remainder that allows optimizations based on algebraic rules.
            #[inline]
            pub fn algebraic_rem(self, rhs: $t) -> $t {
                self % rhs
            }

            /// Rounds toward zero and converts to any primitive integer type,
            /// assuming that the value is finite and fits in that type.
            #[inline]
            pub unsafe fn to_int_unchecked<Int>(self) -> Int
            where
                $t: crate::convert::FloatToInt<Int>,
            {
                <$t as crate::convert::FloatToInt<Int>>::to_int_unchecked(self)
            }

            /// Calculates the middle point of `self` and `rhs`.
            #[inline]
            pub fn midpoint(self, other: $t) -> $t {
                // as std on x86_64: f32 through f64, f64 halving only when
                // the sum could overflow
                if $nbits == 32 {
                    return ((self as f64 + other as f64) * 0.5) as $t;
                }
                const HI: $t = $t::MAX * 0.5;
                if self.abs() <= HI && other.abs() <= HI {
                    (self + other) * 0.5
                } else {
                    (self * 0.5) + (other * 0.5)
                }
            }

            /// Returns the least number greater than `self`.
            pub fn next_up(self) -> $t {
                let sign: $bits = 1 << (crate::mem::size_of::<$bits>() * 8 - 1);
                let bits = self.to_bits();
                if self.is_nan() || bits == <$t>::INFINITY.to_bits() {
                    return self;
                }
                let abs = bits & !sign;
                let next = if abs == 0 { 1 } else if bits == abs { bits + 1 } else { bits - 1 };
                <$t>::from_bits(next)
            }

            /// Returns the greatest number less than `self`.
            pub fn next_down(self) -> $t {
                let sign: $bits = 1 << (crate::mem::size_of::<$bits>() * 8 - 1);
                let bits = self.to_bits();
                if self.is_nan() || bits == <$t>::NEG_INFINITY.to_bits() {
                    return self;
                }
                let abs = bits & !sign;
                let next = if abs == 0 { sign | 1 } else if bits == abs { bits - 1 } else { bits + 1 };
                <$t>::from_bits(next)
            }

            /// Raw transmutation to the unsigned integer of the same width.
            #[inline]
            pub fn to_bits(self) -> $bits {
                unsafe { intrinsics::transmute::<$t, $bits>(self) }
            }

            /// Raw transmutation from the unsigned integer of the same width.
            #[inline]
            pub const fn from_bits(v: $bits) -> $t {
                unsafe { FloatBits::<$bits, $t> { a: v }.b }
            }

            /// Returns the memory representation of this number as a little-endian byte array.
            #[inline]
            pub fn to_le_bytes(self) -> [u8; $bytes] {
                self.to_bits().to_le_bytes()
            }

            /// Returns the memory representation of this number as a big-endian byte array.
            #[inline]
            pub fn to_be_bytes(self) -> [u8; $bytes] {
                self.to_bits().to_be_bytes()
            }

            /// Returns the memory representation of this number as a native-endian byte array.
            #[inline]
            pub fn to_ne_bytes(self) -> [u8; $bytes] {
                self.to_bits().to_ne_bytes()
            }

            /// Creates a value from its representation as a little-endian byte array.
            #[inline]
            pub fn from_le_bytes(bytes: [u8; $bytes]) -> $t {
                $t::from_bits($bits::from_le_bytes(bytes))
            }

            /// Creates a value from its representation as a big-endian byte array.
            #[inline]
            pub fn from_be_bytes(bytes: [u8; $bytes]) -> $t {
                $t::from_bits($bits::from_be_bytes(bytes))
            }

            /// Creates a value from its representation as a native-endian byte array.
            #[inline]
            pub fn from_ne_bytes(bytes: [u8; $bytes]) -> $t {
                $t::from_bits($bits::from_ne_bytes(bytes))
            }

            /// Returns the ordering between `self` and `other` (IEEE 754 totalOrder).
            #[inline]
            pub fn total_cmp(&self, other: &$t) -> Ordering {
                let mut left = self.to_bits() as $sbits;
                let mut right = other.to_bits() as $sbits;
                left = left ^ ((((left >> (($nbits - 1) as u32)) as $bits) >> 1) as $sbits);
                right = right ^ ((((right >> (($nbits - 1) as u32)) as $bits) >> 1) as $sbits);
                left.cmp(&right)
            }

            /// Restrict a value to a certain interval unless it is NaN.
            #[inline]
            pub fn clamp(mut self, min: $t, max: $t) -> $t {
                if !(min <= max) {
                    clamp_failed(min, max);
                }
                if self < min {
                    self = min;
                }
                if self > max {
                    self = max;
                }
                self
            }

            /// Returns the largest integer less than or equal to `self`.
            #[inline]
            pub fn floor(self) -> $t {
                unsafe { $m::floor(self) }
            }

            /// Returns the smallest integer greater than or equal to `self`.
            #[inline]
            pub fn ceil(self) -> $t {
                unsafe { $m::ceil(self) }
            }

            /// Returns the nearest integer to `self`. Rounds half-way cases away from `0.0`.
            #[inline]
            pub fn round(self) -> $t {
                unsafe { $m::round(self) }
            }

            /// Returns the nearest integer to a number. Rounds half-way cases to the number with an even least significant digit.
            #[inline]
            pub fn round_ties_even(self) -> $t {
                let f = self.floor();
                let d = self - f;
                let r = if d < 0.5 {
                    f
                } else if d > 0.5 {
                    f + 1.0
                } else if (f / 2.0).floor() * 2.0 == f {
                    f
                } else {
                    f + 1.0
                };
                // rounding to nearest never changes the sign (-0.4 -> -0.0)
                r.copysign(self)
            }

            /// Returns the integer part of `self`.
            #[inline]
            pub fn trunc(self) -> $t {
                unsafe { $m::trunc(self) }
            }

            /// Returns the fractional part of `self`.
            #[inline]
            pub fn fract(self) -> $t {
                self - self.trunc()
            }

            /// Computes the absolute value of `self`.
            #[inline]
            pub fn abs(self) -> $t {
                unsafe { $m::fabs(self) }
            }

            /// Returns a number that represents the sign of `self`.
            #[inline]
            pub fn signum(self) -> $t {
                if self.is_nan() {
                    $t::NAN
                } else {
                    unsafe { $m::copysign(1.0, self) }
                }
            }

            /// Returns a number composed of the magnitude of `self` and the sign of `sign`.
            #[inline]
            pub fn copysign(self, sign: $t) -> $t {
                unsafe { $m::copysign(self, sign) }
            }

            /// Fused multiply-add. Computes `(self * a) + b` with only one rounding error.
            #[inline]
            pub fn mul_add(self, a: $t, b: $t) -> $t {
                unsafe { $m::fma(self, a, b) }
            }

            /// Calculates Euclidean division.
            #[inline]
            pub fn div_euclid(self, rhs: $t) -> $t {
                let q = (self / rhs).trunc();
                if self % rhs < 0.0 {
                    return if rhs > 0.0 { q - 1.0 } else { q + 1.0 };
                }
                q
            }

            /// Calculates the least nonnegative remainder of `self (mod rhs)`.
            #[inline]
            pub fn rem_euclid(self, rhs: $t) -> $t {
                let r = self % rhs;
                if r < 0.0 {
                    r + rhs.abs()
                } else {
                    r
                }
            }

            /// Raises a number to an integer power.
            #[inline]
            pub fn powi(self, n: i32) -> $t {
                unsafe { $m::powi(self, n) }
            }

            /// Raises a number to a floating point power.
            #[inline]
            pub fn powf(self, n: $t) -> $t {
                unsafe { $m::powf(self, n) }
            }

            /// Returns the square root of a number (NaN for negative numbers).
            #[inline]
            pub fn sqrt(self) -> $t {
                unsafe { $m::sqrt(self) }
            }

            /// Returns `e^(self)`.
            #[inline]
            pub fn exp(self) -> $t {
                unsafe { $m::exp(self) }
            }

            /// Returns `2^(self)`.
            #[inline]
            pub fn exp2(self) -> $t {
                unsafe { $m::exp2(self) }
            }

            /// Returns `e^(self) - 1`.
            #[inline]
            pub fn exp_m1(self) -> $t {
                unsafe { $m::expm1(self) }
            }

            /// Returns the natural logarithm of the number.
            #[inline]
            pub fn ln(self) -> $t {
                unsafe { $m::log(self) }
            }

            /// Returns `ln(1+n)` more accurately than if the operations were performed separately.
            #[inline]
            pub fn ln_1p(self) -> $t {
                unsafe { $m::ln1p(self) }
            }

            /// Returns the logarithm of the number with respect to an arbitrary base.
            #[inline]
            pub fn log(self, base: $t) -> $t {
                self.ln() / base.ln()
            }

            /// Returns the base 2 logarithm of the number.
            #[inline]
            pub fn log2(self) -> $t {
                unsafe { $m::log2(self) }
            }

            /// Returns the base 10 logarithm of the number.
            #[inline]
            pub fn log10(self) -> $t {
                unsafe { $m::log10(self) }
            }

            /// Returns the cube root of a number.
            #[inline]
            pub fn cbrt(self) -> $t {
                // Same algorithms as Rust's libm (correctly rounded for f64),
                // so results match rustc bit for bit.
                <$t as CbrtImpl>::cbrt_impl(self)
            }

            /// Compute the distance between the origin and a point (`x`, `y`) on the Euclidean plane.
            #[inline]
            pub fn hypot(self, other: $t) -> $t {
                unsafe { $m::hypot(self, other) }
            }

            /// Computes the sine of a number (in radians).
            #[inline]
            pub fn sin(self) -> $t {
                unsafe { $m::sin(self) }
            }

            /// Computes the cosine of a number (in radians).
            #[inline]
            pub fn cos(self) -> $t {
                unsafe { $m::cos(self) }
            }

            /// Computes the tangent of a number (in radians).
            #[inline]
            pub fn tan(self) -> $t {
                unsafe { $m::tan(self) }
            }

            /// Computes the arcsine of a number.
            #[inline]
            pub fn asin(self) -> $t {
                unsafe { $m::asin(self) }
            }

            /// Computes the arccosine of a number.
            #[inline]
            pub fn acos(self) -> $t {
                unsafe { $m::acos(self) }
            }

            /// Computes the arctangent of a number.
            #[inline]
            pub fn atan(self) -> $t {
                unsafe { $m::atan(self) }
            }

            /// Computes the four quadrant arctangent of `self` (`y`) and `other` (`x`) in radians.
            #[inline]
            pub fn atan2(self, other: $t) -> $t {
                unsafe { $m::atan2(self, other) }
            }

            /// Simultaneously computes the sine and cosine of the number.
            #[inline]
            pub fn sin_cos(self) -> ($t, $t) {
                (self.sin(), self.cos())
            }

            /// Hyperbolic sine function.
            #[inline]
            pub fn sinh(self) -> $t {
                unsafe { $m::sinh(self) }
            }

            /// Hyperbolic cosine function.
            #[inline]
            pub fn cosh(self) -> $t {
                unsafe { $m::cosh(self) }
            }

            /// Hyperbolic tangent function.
            #[inline]
            pub fn tanh(self) -> $t {
                unsafe { $m::tanh(self) }
            }

            /// Inverse hyperbolic sine function.
            #[inline]
            pub fn asinh(self) -> $t {
                // libm's, like std
                unsafe { $m::asinh(self) }
            }

            /// Inverse hyperbolic cosine function.
            #[inline]
            pub fn acosh(self) -> $t {
                unsafe { $m::acosh(self) }
            }

            /// Inverse hyperbolic tangent function.
            #[inline]
            pub fn atanh(self) -> $t {
                0.5 * ((2.0 * self) / (1.0 - self)).ln_1p()
            }

            /// The positive difference of two numbers (deprecated).
            #[inline]
            pub fn abs_sub(self, other: $t) -> $t {
                if self <= other {
                    0.0
                } else {
                    self - other
                }
            }
        }
    };
}

__std_float_impl! { f64, u64, i64, f64_math, 64, 52, 8 }
__std_float_impl! { f32, u32, i32, f32_math, 32, 23, 4 }

impl f64 {
    /// The radix or base of the internal representation of `f64`.
    pub const RADIX: u32 = 2;
    /// Number of significant digits in base 2.
    pub const MANTISSA_DIGITS: u32 = 53;
    /// Approximate number of significant digits in base 10.
    pub const DIGITS: u32 = 15;
    /// Machine epsilon value for `f64`.
    pub const EPSILON: f64 = 2.220446049250313e-16;
    /// Smallest finite `f64` value.
    pub const MIN: f64 = -1.7976931348623157e+308;
    /// Smallest positive normal `f64` value.
    pub const MIN_POSITIVE: f64 = 2.2250738585072014e-308;
    /// Largest finite `f64` value.
    pub const MAX: f64 = 1.7976931348623157e+308;
    /// One greater than the minimum possible normal power of 2 exponent.
    pub const MIN_EXP: i32 = -1021;
    /// Maximum possible power of 2 exponent.
    pub const MAX_EXP: i32 = 1024;
    /// Minimum possible normal power of 10 exponent.
    pub const MIN_10_EXP: i32 = -307;
    /// Maximum possible power of 10 exponent.
    pub const MAX_10_EXP: i32 = 308;
    /// Not a Number (NaN).
    pub const NAN: f64 = f64::from_bits(0x7ff8_0000_0000_0000);
    /// Infinity (∞).
    pub const INFINITY: f64 = 1.0 / 0.0;
    /// Negative infinity (−∞).
    pub const NEG_INFINITY: f64 = -1.0 / 0.0;
}

impl f32 {
    /// The radix or base of the internal representation of `f32`.
    pub const RADIX: u32 = 2;
    /// Number of significant digits in base 2.
    pub const MANTISSA_DIGITS: u32 = 24;
    /// Approximate number of significant digits in base 10.
    pub const DIGITS: u32 = 6;
    /// Machine epsilon value for `f32`.
    pub const EPSILON: f32 = 1.19209290e-07;
    /// Smallest finite `f32` value.
    pub const MIN: f32 = -3.40282347e+38;
    /// Smallest positive normal `f32` value.
    pub const MIN_POSITIVE: f32 = 1.17549435e-38;
    /// Largest finite `f32` value.
    pub const MAX: f32 = 3.40282347e+38;
    /// One greater than the minimum possible normal power of 2 exponent.
    pub const MIN_EXP: i32 = -125;
    /// Maximum possible power of 2 exponent.
    pub const MAX_EXP: i32 = 128;
    /// Minimum possible normal power of 10 exponent.
    pub const MIN_10_EXP: i32 = -37;
    /// Maximum possible power of 10 exponent.
    pub const MAX_10_EXP: i32 = 38;
    /// Not a Number (NaN).
    pub const NAN: f32 = f32::from_bits(0x7fc0_0000);
    /// Infinity (∞).
    pub const INFINITY: f32 = 1.0 / 0.0;
    /// Negative infinity (−∞).
    pub const NEG_INFINITY: f32 = -1.0 / 0.0;
}

trait CbrtImpl {
    fn cbrt_impl(self) -> Self;
}

impl CbrtImpl for f64 {
    /// Correctly rounded cube root (port of CORE-MATH via Rust's libm).
    fn cbrt_impl(self) -> f64 {
        let x = self;
        let hf = f64::from_bits;
        let escale: [f64; 3] = [1.0, hf(0x3ff428a2f98d728b), hf(0x3ff965fea53d6e3d)];
        let c: [f64; 4] = [hf(0x3fe1b0babccfef9c), hf(0x3fe2c9a3e94d1da5), hf(0xbfc4dc30b1a1ddba), hf(0x3f97a8d3e4ec9b07)];
        let u0: f64 = hf(0x3fd5555555555555);
        let u1: f64 = hf(0x3fcc71c71c71c71c);
        let rsc = [1.0, -1.0, 0.5, -0.5, 0.25, -0.25];
        let off0: f64 = hf(0x3ca0000000000000);
        let sig_mask: u64 = (1u64 << 52) - 1;
        let exp_sat: u32 = 0x7ff;

        let hx: u64 = x.to_bits();
        let mut mant: u64 = hx & sig_mask;
        let sign: u64 = hx >> 63;
        let mut e: u32 = (hx >> 52) as u32 & exp_sat;

        if ((e + 1) & exp_sat) < 2 {
            let ix: u64 = hx & !(1u64 << 63);
            if e == exp_sat || ix == 0 {
                return x + x;
            }
            let nz = ix.leading_zeros() - 11;
            mant <<= nz;
            mant &= sig_mask;
            e = e.wrapping_sub(nz - 1);
        }

        e = e.wrapping_add(3072);
        let cvt1: u64 = mant | (0x3ffu64 << 52);
        let mut cvt5: u64 = cvt1;
        let et: u32 = e / 3;
        let it: u32 = e % 3;
        cvt5 += (it as u64) << 52;
        cvt5 |= sign << 63;
        let zz: f64 = f64::from_bits(cvt5);
        let mut isc: u64 = escale[it as usize].to_bits();
        isc |= sign << 63;
        let cvt2: u64 = isc;
        let z: f64 = f64::from_bits(cvt1);

        let r: f64 = 1.0 / z;
        let rr: f64 = r * rsc[((it as usize) << 1) | sign as usize];
        let z2: f64 = z * z;
        let c0: f64 = c[0] + z * c[1];
        let c2: f64 = c[2] + z * c[3];
        let mut y: f64 = c0 + z2 * c2;
        let mut y2: f64 = y * y;
        let mut h: f64 = y2 * (y * r) - 1.0;
        y -= (h * y) * (u0 - u1 * h);
        y *= f64::from_bits(cvt2);

        y2 = y * y;
        let mut y2l: f64 = y.mul_add(y, -y2);
        let mut y3: f64 = y2 * y;
        let mut y3l: f64 = y.mul_add(y2, -y3) + y * y2l;
        h = ((y3 - zz) + y3l) * rr;
        let mut dy: f64 = h * (y * u0);
        let mut y1: f64 = y - dy;
        dy = (y - y1) - dy;

        let mut ady: f64 = dy.abs();
        let mut ady0: f64 = (ady - off0).abs();
        let mut ady1: f64 = (ady - (hf(0x3cb0000000000000) + off0)).abs();

        if ady0 < hf(0x3b40000000000000) || ady1 < hf(0x3b40000000000000) {
            y2 = y1 * y1;
            y2l = y1.mul_add(y1, -y2);
            y3 = y2 * y1;
            y3l = y1.mul_add(y2, -y3) + y1 * y2l;
            h = ((y3 - zz) + y3l) * rr;
            dy = h * (y1 * u0);
            y = y1 - dy;
            dy = (y1 - y) - dy;
            y1 = y;
            ady = dy.abs();
            ady0 = (ady - off0).abs();
            ady1 = (ady - (hf(0x3cb0000000000000) + off0)).abs();

            if ady0 < hf(0x39d0000000000000) || ady1 < hf(0x39d0000000000000) {
                let azz: f64 = zz.abs();
                if azz == hf(0x4009b78223aa307c) {
                    y1 = hf(0x3ff79d15d0e8d59c).copysign(zz);
                }
                if azz == hf(0x401a202bfc89ddff) {
                    y1 = hf(0x3ffde87aa837820f).copysign(zz);
                }
            }
        }

        let mut cvt3: u64 = y1.to_bits();
        cvt3 = cvt3.wrapping_add(((et.wrapping_sub(342).wrapping_sub(1023)) as u64) << 52);
        let m0: u64 = cvt3 << 30;
        let m1 = m0 >> 63;

        if (m0 ^ m1) <= (1u64 << 30) {
            let mut cvt4: u64 = y1.to_bits();
            cvt4 = (cvt4 + (164 << 15)) & 0xffffffffffff0000u64;
            if ((f64::from_bits(cvt4) - y1) - dy).abs() < hf(0x3c30000000000000) || (zz).abs() == 1.0 {
                cvt3 = (cvt3 + (1u64 << 15)) & 0xffffffffffff0000u64;
            }
        }
        f64::from_bits(cvt3)
    }
}

impl CbrtImpl for f32 {
    /// Cube root (port of FreeBSD's s_cbrtf.c via Rust's libm).
    fn cbrt_impl(self) -> f32 {
        let x = self;
        const B1: u32 = 709958130;
        const B2: u32 = 642849266;
        let x1p24 = f32::from_bits(0x4b800000);
        let mut r: f64;
        let mut t: f64;
        let mut ui: u32 = x.to_bits();
        let mut hx: u32 = ui & 0x7fffffff;
        if hx >= 0x7f800000 {
            return x + x;
        }
        if hx < 0x00800000 {
            if hx == 0 {
                return x;
            }
            ui = (x * x1p24).to_bits();
            hx = ui & 0x7fffffff;
            hx = hx / 3 + B2;
        } else {
            hx = hx / 3 + B1;
        }
        ui &= 0x80000000;
        ui |= hx;
        t = f32::from_bits(ui) as f64;
        r = t * t * t;
        t = t * (x as f64 + x as f64 + r) / (x as f64 + r + r);
        r = t * t * t;
        t = t * (x as f64 + x as f64 + r) / (x as f64 + r + r);
        t as f32
    }
}
