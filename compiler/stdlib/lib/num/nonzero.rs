//! Integer types that are known not to equal zero.

use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::ops::{BitOr, BitOrAssign, Div, Neg, Rem};
use crate::str::FromStr;

/// A marker trait for primitive types which can be zero (the integer types).
#[doc(hidden)]
pub trait ZeroablePrimitive: Copy + PartialEq + PartialOrd + Ord + Hash + fmt::Debug + fmt::Display {
    #[doc(hidden)]
    const ZERO: Self;
}

/// A value that is known not to equal zero.
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct NonZero<T: ZeroablePrimitive>(T);

impl<T: ZeroablePrimitive> NonZero<T> {
    /// Creates a non-zero if the given value is not zero.
    #[inline]
    pub const fn new(n: T) -> Option<Self> {
        if n != T::ZERO { Some(NonZero(n)) } else { None }
    }

    /// Creates a non-zero without checking whether the value is non-zero.
    #[inline]
    pub const unsafe fn new_unchecked(n: T) -> Self {
        NonZero(n)
    }

    /// Returns the contained value as a primitive type.
    #[inline]
    pub const fn get(self) -> T {
        self.0
    }

    /// Converts a reference to a non-zero mutable reference if the referenced value is not zero.
    #[inline]
    pub fn from_mut(n: &mut T) -> Option<&mut Self> {
        if *n != T::ZERO { Some(unsafe { &mut *(n as *mut T as *mut NonZero<T>) }) } else { None }
    }
}

impl<T: ZeroablePrimitive> From<NonZero<T>> for T {
    #[inline]
    fn from(n: NonZero<T>) -> T {
        n.0
    }
}

impl<T: ZeroablePrimitive> PartialOrd for NonZero<T> {
    #[inline]
    fn partial_cmp(&self, other: &NonZero<T>) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<T: ZeroablePrimitive> Ord for NonZero<T> {
    #[inline]
    fn cmp(&self, other: &NonZero<T>) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: ZeroablePrimitive> Hash for NonZero<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<T: ZeroablePrimitive> fmt::Debug for NonZero<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl<T: ZeroablePrimitive> fmt::Display for NonZero<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl<T: ZeroablePrimitive + fmt::Binary> fmt::Binary for NonZero<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Binary::fmt(&self.0, f)
    }
}

impl<T: ZeroablePrimitive + fmt::Octal> fmt::Octal for NonZero<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Octal::fmt(&self.0, f)
    }
}

impl<T: ZeroablePrimitive + fmt::LowerHex> fmt::LowerHex for NonZero<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl<T: ZeroablePrimitive + fmt::UpperHex> fmt::UpperHex for NonZero<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(&self.0, f)
    }
}

macro_rules! __std_nonzero_common {
    ($($t:ident),*) => {
        $(
            impl ZeroablePrimitive for $t {
                const ZERO: $t = 0;
            }

            impl NonZero<$t> {
                /// The smallest value that can be represented by this non-zero integer type.
                pub const MIN: NonZero<$t> = NonZero(if <$t>::MIN == 0 { 1 } else { <$t>::MIN });
                /// The largest value that can be represented by this non-zero integer type.
                pub const MAX: NonZero<$t> = NonZero(<$t>::MAX);
                /// The size of this non-zero integer type in bits.
                pub const BITS: u32 = <$t>::BITS;

                /// Returns the number of leading zeros in the binary representation of `self`.
                #[inline]
                pub fn leading_zeros(self) -> u32 {
                    self.0.leading_zeros()
                }
                /// Returns the number of trailing zeros in the binary representation of `self`.
                #[inline]
                pub fn trailing_zeros(self) -> u32 {
                    self.0.trailing_zeros()
                }
                /// Returns the number of ones in the binary representation of `self`.
                #[inline]
                pub fn count_ones(self) -> NonZero<u32> {
                    NonZero(self.0.count_ones())
                }
                /// Multiplies two non-zero integers together, checking for overflow.
                #[inline]
                pub fn checked_mul(self, other: NonZero<$t>) -> Option<NonZero<$t>> {
                    match self.0.checked_mul(other.0) {
                        Some(v) => Some(NonZero(v)),
                        None => None,
                    }
                }
                /// Multiplies two non-zero integers together, saturating at the numeric bounds.
                #[inline]
                pub fn saturating_mul(self, other: NonZero<$t>) -> NonZero<$t> {
                    NonZero(self.0.saturating_mul(other.0))
                }
                /// Raises non-zero value to an integer power, checking for overflow.
                #[inline]
                pub fn checked_pow(self, exp: u32) -> Option<NonZero<$t>> {
                    match self.0.checked_pow(exp) {
                        Some(v) => Some(NonZero(v)),
                        None => None,
                    }
                }
                /// Raises non-zero value to an integer power, saturating at the numeric bounds.
                #[inline]
                pub fn saturating_pow(self, exp: u32) -> NonZero<$t> {
                    NonZero(self.0.saturating_pow(exp))
                }
            }

            impl crate::convert::TryFrom<$t> for NonZero<$t> {
                type Error = crate::num::TryFromIntError;
                fn try_from(n: $t) -> Result<NonZero<$t>, crate::num::TryFromIntError> {
                    match NonZero::new(n) {
                        Some(v) => Ok(v),
                        None => Err(crate::convert::TryFromIntError(crate::num::IntErrorKind::Zero)),
                    }
                }
            }

            impl BitOr for NonZero<$t> {
                type Output = NonZero<$t>;
                #[inline]
                fn bitor(self, rhs: NonZero<$t>) -> NonZero<$t> {
                    NonZero(self.0 | rhs.0)
                }
            }

            impl BitOr<$t> for NonZero<$t> {
                type Output = NonZero<$t>;
                #[inline]
                fn bitor(self, rhs: $t) -> NonZero<$t> {
                    NonZero(self.0 | rhs)
                }
            }

            impl BitOr<NonZero<$t>> for $t {
                type Output = NonZero<$t>;
                #[inline]
                fn bitor(self, rhs: NonZero<$t>) -> NonZero<$t> {
                    NonZero(self | rhs.0)
                }
            }

            impl BitOrAssign for NonZero<$t> {
                #[inline]
                fn bitor_assign(&mut self, rhs: NonZero<$t>) {
                    self.0 = self.0 | rhs.0;
                }
            }

            impl BitOrAssign<$t> for NonZero<$t> {
                #[inline]
                fn bitor_assign(&mut self, rhs: $t) {
                    self.0 = self.0 | rhs;
                }
            }

            impl FromStr for NonZero<$t> {
                type Err = crate::num::ParseIntError;
                fn from_str(src: &str) -> Result<NonZero<$t>, crate::num::ParseIntError> {
                    match <$t>::from_str(src) {
                        Ok(v) => match NonZero::new(v) {
                            Some(v) => Ok(v),
                            None => Err(crate::num::ParseIntError { kind: crate::num::IntErrorKind::Zero }),
                        },
                        Err(e) => Err(e),
                    }
                }
            }
        )*
    };
}

__std_nonzero_common! { u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize }

macro_rules! __std_nonzero_unsigned {
    ($($t:ident),*) => {
        $(
            impl NonZero<$t> {
                /// Adds an unsigned integer to a non-zero value, checking for overflow.
                #[inline]
                pub fn checked_add(self, other: $t) -> Option<NonZero<$t>> {
                    match self.0.checked_add(other) {
                        Some(v) => Some(NonZero(v)),
                        None => None,
                    }
                }
                /// Adds an unsigned integer to a non-zero value, saturating at the numeric bounds.
                #[inline]
                pub fn saturating_add(self, other: $t) -> NonZero<$t> {
                    NonZero(self.0.saturating_add(other))
                }
                /// Returns the smallest power of two greater than or equal to `self`, checking for overflow.
                #[inline]
                pub fn checked_next_power_of_two(self) -> Option<NonZero<$t>> {
                    match self.0.checked_next_power_of_two() {
                        Some(v) => Some(NonZero(v)),
                        None => None,
                    }
                }
                /// Returns the base 2 logarithm of the number, rounded down.
                #[inline]
                pub fn ilog2(self) -> u32 {
                    <$t>::BITS - 1 - self.0.leading_zeros()
                }
                /// Returns the base 10 logarithm of the number, rounded down.
                #[inline]
                pub fn ilog10(self) -> u32 {
                    self.0.ilog10()
                }
                /// Returns `true` if and only if `self == (1 << k)` for some `k`.
                #[inline]
                pub fn is_power_of_two(self) -> bool {
                    self.0.is_power_of_two()
                }
            }

            impl Div<NonZero<$t>> for $t {
                type Output = $t;
                #[inline]
                fn div(self, other: NonZero<$t>) -> $t {
                    self / other.0
                }
            }

            impl Rem<NonZero<$t>> for $t {
                type Output = $t;
                #[inline]
                fn rem(self, other: NonZero<$t>) -> $t {
                    self % other.0
                }
            }
        )*
    };
}

__std_nonzero_unsigned! { u8, u16, u32, u64, u128, usize }

macro_rules! __std_nonzero_signed {
    ($($t:ident $ut:ident),*) => {
        $(
            impl NonZero<$t> {
                /// Computes the absolute value of self.
                #[inline]
                pub fn abs(self) -> NonZero<$t> {
                    NonZero(self.0.abs())
                }
                /// Checked absolute value.
                #[inline]
                pub fn checked_abs(self) -> Option<NonZero<$t>> {
                    match self.0.checked_abs() {
                        Some(v) => Some(NonZero(v)),
                        None => None,
                    }
                }
                /// Wrapping absolute value.
                #[inline]
                pub fn wrapping_abs(self) -> NonZero<$t> {
                    NonZero(self.0.wrapping_abs())
                }
                /// Saturating absolute value.
                #[inline]
                pub fn saturating_abs(self) -> NonZero<$t> {
                    NonZero(self.0.saturating_abs())
                }
                /// Computes the absolute value of self without any wrapping or panicking.
                #[inline]
                pub fn unsigned_abs(self) -> NonZero<$ut> {
                    NonZero(self.0.unsigned_abs())
                }
                /// Returns `true` if `self` is positive.
                #[inline]
                pub fn is_positive(self) -> bool {
                    self.0 > 0
                }
                /// Returns `true` if `self` is negative.
                #[inline]
                pub fn is_negative(self) -> bool {
                    self.0 < 0
                }
                /// Checked negation.
                #[inline]
                pub fn checked_neg(self) -> Option<NonZero<$t>> {
                    match self.0.checked_neg() {
                        Some(v) => Some(NonZero(v)),
                        None => None,
                    }
                }
                /// Wrapping negation.
                #[inline]
                pub fn wrapping_neg(self) -> NonZero<$t> {
                    NonZero(self.0.wrapping_neg())
                }
                /// Saturating negation.
                #[inline]
                pub fn saturating_neg(self) -> NonZero<$t> {
                    NonZero(self.0.saturating_neg())
                }
            }

            impl Neg for NonZero<$t> {
                type Output = NonZero<$t>;
                #[inline]
                fn neg(self) -> NonZero<$t> {
                    NonZero(-self.0)
                }
            }
        )*
    };
}

__std_nonzero_signed! { i8 u8, i16 u16, i32 u32, i64 u64, i128 u128, isize usize }

/// An integer that is known not to equal zero.
pub type NonZeroU8 = NonZero<u8>;
/// An integer that is known not to equal zero.
pub type NonZeroU16 = NonZero<u16>;
/// An integer that is known not to equal zero.
pub type NonZeroU32 = NonZero<u32>;
/// An integer that is known not to equal zero.
pub type NonZeroU64 = NonZero<u64>;
/// An integer that is known not to equal zero.
pub type NonZeroU128 = NonZero<u128>;
/// An integer that is known not to equal zero.
pub type NonZeroUsize = NonZero<usize>;
/// An integer that is known not to equal zero.
pub type NonZeroI8 = NonZero<i8>;
/// An integer that is known not to equal zero.
pub type NonZeroI16 = NonZero<i16>;
/// An integer that is known not to equal zero.
pub type NonZeroI32 = NonZero<i32>;
/// An integer that is known not to equal zero.
pub type NonZeroI64 = NonZero<i64>;
/// An integer that is known not to equal zero.
pub type NonZeroI128 = NonZero<i128>;
/// An integer that is known not to equal zero.
pub type NonZeroIsize = NonZero<isize>;
