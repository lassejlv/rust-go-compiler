//! Integer types that are known not to equal zero.

use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::ops::{BitOr, BitOrAssign};
use crate::str::FromStr;

macro_rules! __std_nonzero {
    ($($name:ident: $t:ty),* $(,)?) => {
        $(
            /// An integer that is known not to equal zero.
            #[derive(Copy, Clone, PartialEq, Eq)]
            #[repr(transparent)]
            pub struct $name($t);

            impl $name {
                /// The smallest value that can be represented by this non-zero integer type.
                pub const MIN: $name = $name(if <$t>::MIN == 0 { 1 } else { <$t>::MIN });
                /// The largest value that can be represented by this non-zero integer type.
                pub const MAX: $name = $name(<$t>::MAX);
                /// The size of this non-zero integer type in bits.
                pub const BITS: u32 = <$t>::BITS;

                /// Creates a non-zero if the given value is not zero.
                #[inline]
                pub const fn new(n: $t) -> Option<$name> {
                    if n != 0 { Some($name(n)) } else { None }
                }
                /// Creates a non-zero without checking whether the value is non-zero.
                #[inline]
                pub const unsafe fn new_unchecked(n: $t) -> $name {
                    $name(n)
                }
                /// Returns the contained value as a primitive type.
                #[inline]
                pub const fn get(self) -> $t {
                    self.0
                }
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
                pub fn count_ones(self) -> u32 {
                    self.0.count_ones()
                }
                /// Multiplies two non-zero integers together, checking for overflow.
                #[inline]
                pub fn checked_mul(self, other: $name) -> Option<$name> {
                    match self.0.checked_mul(other.0) {
                        Some(v) => Some($name(v)),
                        None => None,
                    }
                }
                /// Multiplies two non-zero integers together, saturating at the numeric bounds.
                #[inline]
                pub fn saturating_mul(self, other: $name) -> $name {
                    $name(self.0.saturating_mul(other.0))
                }
                /// Raises non-zero value to an integer power, checking for overflow.
                #[inline]
                pub fn checked_pow(self, exp: u32) -> Option<$name> {
                    match self.0.checked_pow(exp) {
                        Some(v) => Some($name(v)),
                        None => None,
                    }
                }
            }

            impl From<$name> for $t {
                #[inline]
                fn from(n: $name) -> $t {
                    n.0
                }
            }

            impl crate::convert::TryFrom<$t> for $name {
                type Error = crate::num::TryFromIntError;
                fn try_from(n: $t) -> Result<$name, crate::num::TryFromIntError> {
                    match $name::new(n) {
                        Some(v) => Ok(v),
                        None => Err(crate::convert::TryFromIntError(crate::num::IntErrorKind::Zero)),
                    }
                }
            }

            impl PartialOrd for $name {
                #[inline]
                fn partial_cmp(&self, other: &$name) -> Option<Ordering> {
                    self.0.partial_cmp(&other.0)
                }
            }

            impl Ord for $name {
                #[inline]
                fn cmp(&self, other: &$name) -> Ordering {
                    self.0.cmp(&other.0)
                }
            }

            impl Hash for $name {
                fn hash<H: Hasher>(&self, state: &mut H) {
                    self.0.hash(state)
                }
            }

            impl BitOr for $name {
                type Output = $name;
                #[inline]
                fn bitor(self, rhs: $name) -> $name {
                    $name(self.0 | rhs.0)
                }
            }

            impl BitOr<$t> for $name {
                type Output = $name;
                #[inline]
                fn bitor(self, rhs: $t) -> $name {
                    $name(self.0 | rhs)
                }
            }

            impl BitOrAssign for $name {
                #[inline]
                fn bitor_assign(&mut self, rhs: $name) {
                    self.0 = self.0 | rhs.0;
                }
            }

            impl FromStr for $name {
                type Err = crate::num::ParseIntError;
                fn from_str(src: &str) -> Result<$name, crate::num::ParseIntError> {
                    match <$t>::from_str(src) {
                        Ok(v) => match $name::new(v) {
                            Some(v) => Ok(v),
                            None => Err(crate::num::ParseIntError { kind: crate::num::IntErrorKind::Zero }),
                        },
                        Err(e) => Err(e),
                    }
                }
            }

            impl fmt::Debug for $name {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    fmt::Debug::fmt(&self.0, f)
                }
            }

            impl fmt::Display for $name {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    fmt::Display::fmt(&self.0, f)
                }
            }

            impl fmt::Binary for $name {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    fmt::Binary::fmt(&self.0, f)
                }
            }

            impl fmt::LowerHex for $name {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    fmt::LowerHex::fmt(&self.0, f)
                }
            }

            impl fmt::UpperHex for $name {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    fmt::UpperHex::fmt(&self.0, f)
                }
            }
        )*
    };
}

__std_nonzero! {
    NonZeroU8: u8, NonZeroU16: u16, NonZeroU32: u32, NonZeroU64: u64, NonZeroU128: u128, NonZeroUsize: usize,
    NonZeroI8: i8, NonZeroI16: i16, NonZeroI32: i32, NonZeroI64: i64, NonZeroI128: i128, NonZeroIsize: isize,
}
