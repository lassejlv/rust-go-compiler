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
        tanf64 as tan, tanhf64 as tanh, truncf64 as trunc,
    };
}

mod f32_math {
    pub use crate::intrinsics::{
        acosf32 as acos, asinf32 as asin, atan2f32 as atan2, atanf32 as atan, cbrtf32 as cbrt, ceilf32 as ceil,
        copysignf32 as copysign, cosf32 as cos, coshf32 as cosh, exp2f32 as exp2, expf32 as exp, fabsf32 as fabs,
        floorf32 as floor, fmaf32 as fma, hypotf32 as hypot, log10f32 as log10, log2f32 as log2, logf32 as log,
        powff32 as powf, powif32 as powi, roundf32 as round, sinf32 as sin, sinhf32 as sinh, sqrtf32 as sqrt,
        tanf32 as tan, tanhf32 as tanh, truncf32 as trunc,
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

            /// Calculates the middle point of `self` and `rhs`.
            #[inline]
            pub fn midpoint(self, other: $t) -> $t {
                let s = self + other;
                if s.is_finite() {
                    s / 2.0
                } else {
                    self / 2.0 + other / 2.0
                }
            }

            /// Raw transmutation to the unsigned integer of the same width.
            #[inline]
            pub fn to_bits(self) -> $bits {
                unsafe { intrinsics::transmute::<$t, $bits>(self) }
            }

            /// Raw transmutation from the unsigned integer of the same width.
            #[inline]
            pub const fn from_bits(v: $bits) -> $t {
                unsafe { intrinsics::transmute::<$bits, $t>(v) }
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
                if d < 0.5 {
                    f
                } else if d > 0.5 {
                    f + 1.0
                } else if (f / 2.0).floor() * 2.0 == f {
                    f
                } else {
                    f + 1.0
                }
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
                if self.abs() < 1e-5 {
                    self + self * self / 2.0 + self * self * self / 6.0
                } else {
                    self.exp() - 1.0
                }
            }

            /// Returns the natural logarithm of the number.
            #[inline]
            pub fn ln(self) -> $t {
                unsafe { $m::log(self) }
            }

            /// Returns `ln(1+n)` more accurately than if the operations were performed separately.
            #[inline]
            pub fn ln_1p(self) -> $t {
                if self.abs() < 1e-5 {
                    self - self * self / 2.0 + self * self * self / 3.0
                } else {
                    (1.0 + self).ln()
                }
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
                unsafe { $m::cbrt(self) }
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
                let ax = self.abs();
                let r = (ax + (ax * ax + 1.0).sqrt()).ln();
                r.copysign(self)
            }

            /// Inverse hyperbolic cosine function.
            #[inline]
            pub fn acosh(self) -> $t {
                if self < 1.0 {
                    $t::NAN
                } else {
                    (self + (self * self - 1.0).sqrt()).ln()
                }
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
