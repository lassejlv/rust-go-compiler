//! Constants for the `f64` double-precision floating point type.

/// The radix or base of the internal representation of `f64`.
pub const RADIX: u32 = f64::RADIX;
/// Number of significant digits in base 2.
pub const MANTISSA_DIGITS: u32 = f64::MANTISSA_DIGITS;
/// Approximate number of significant digits in base 10.
pub const DIGITS: u32 = f64::DIGITS;
/// Machine epsilon value for `f64`.
pub const EPSILON: f64 = f64::EPSILON;
/// Smallest finite `f64` value.
pub const MIN: f64 = f64::MIN;
/// Smallest positive normal `f64` value.
pub const MIN_POSITIVE: f64 = f64::MIN_POSITIVE;
/// Largest finite `f64` value.
pub const MAX: f64 = f64::MAX;
/// One greater than the minimum possible normal power of 2 exponent.
pub const MIN_EXP: i32 = f64::MIN_EXP;
/// Maximum possible power of 2 exponent.
pub const MAX_EXP: i32 = f64::MAX_EXP;
/// Minimum possible normal power of 10 exponent.
pub const MIN_10_EXP: i32 = f64::MIN_10_EXP;
/// Maximum possible power of 10 exponent.
pub const MAX_10_EXP: i32 = f64::MAX_10_EXP;
/// Not a Number (NaN).
pub const NAN: f64 = f64::NAN;
/// Infinity (∞).
pub const INFINITY: f64 = f64::INFINITY;
/// Negative infinity (−∞).
pub const NEG_INFINITY: f64 = f64::NEG_INFINITY;

/// Basic mathematical constants.
pub mod consts {
    /// Archimedes' constant (π)
    pub const PI: f64 = 3.14159265358979323846264338327950288;
    /// The full circle constant (τ)
    pub const TAU: f64 = 6.28318530717958647692528676655900577;
    /// The golden ratio (φ)
    pub const PHI: f64 = 1.618033988749894848204586834365638118;
    /// π/2
    pub const FRAC_PI_2: f64 = 1.57079632679489661923132169163975144;
    /// π/3
    pub const FRAC_PI_3: f64 = 1.04719755119659774615421446109316763;
    /// π/4
    pub const FRAC_PI_4: f64 = 0.785398163397448309615660845819875721;
    /// π/6
    pub const FRAC_PI_6: f64 = 0.52359877559829887307710723054658381;
    /// π/8
    pub const FRAC_PI_8: f64 = 0.39269908169872415480783042290993786;
    /// 1/π
    pub const FRAC_1_PI: f64 = 0.318309886183790671537767526745028724;
    /// 2/π
    pub const FRAC_2_PI: f64 = 0.636619772367581343075535053490057448;
    /// 2/sqrt(π)
    pub const FRAC_2_SQRT_PI: f64 = 1.12837916709551257389615890312154517;
    /// sqrt(2)
    pub const SQRT_2: f64 = 1.41421356237309504880168872420969808;
    /// 1/sqrt(2)
    pub const FRAC_1_SQRT_2: f64 = 0.707106781186547524400844362104849039;
    /// Euler's number (e)
    pub const E: f64 = 2.71828182845904523536028747135266250;
    /// log<sub>2</sub>(10)
    pub const LOG2_10: f64 = 3.32192809488736234787031942948939018;
    /// log<sub>2</sub>(e)
    pub const LOG2_E: f64 = 1.44269504088896340735992468100189214;
    /// log<sub>10</sub>(2)
    pub const LOG10_2: f64 = 0.301029995663981195213738894724493027;
    /// log<sub>10</sub>(e)
    pub const LOG10_E: f64 = 0.434294481903251827651128918916605082;
    /// ln(2)
    pub const LN_2: f64 = 0.693147180559945309417232121458176568;
    /// ln(10)
    pub const LN_10: f64 = 2.30258509299404568401799145468436421;
}
