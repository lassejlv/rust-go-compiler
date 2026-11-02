//! `Wrapping<T>` and `Saturating<T>`: intentionally-wrapped/saturating arithmetic.

use crate::fmt;
use crate::ops::{Add, AddAssign, BitAnd, BitOr, BitXor, Mul, MulAssign, Neg, Not, Sub, SubAssign};

/// Provides intentionally-wrapped arithmetic on `T`.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default, Hash)]
#[repr(transparent)]
pub struct Wrapping<T>(pub T);

impl<T: fmt::Debug> fmt::Debug for Wrapping<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: fmt::Display> fmt::Display for Wrapping<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Provides intentionally-saturating arithmetic on `T`.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default, Hash)]
#[repr(transparent)]
pub struct Saturating<T>(pub T);

impl<T: fmt::Debug> fmt::Debug for Saturating<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: fmt::Display> fmt::Display for Saturating<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

macro_rules! __std_wrapping_impl {
    ($($t:ty)*) => {
        $(
            impl Add for Wrapping<$t> {
                type Output = Wrapping<$t>;
                #[inline]
                fn add(self, other: Wrapping<$t>) -> Wrapping<$t> {
                    Wrapping(self.0.wrapping_add(other.0))
                }
            }
            impl AddAssign for Wrapping<$t> {
                #[inline]
                fn add_assign(&mut self, other: Wrapping<$t>) {
                    *self = *self + other;
                }
            }
            impl Sub for Wrapping<$t> {
                type Output = Wrapping<$t>;
                #[inline]
                fn sub(self, other: Wrapping<$t>) -> Wrapping<$t> {
                    Wrapping(self.0.wrapping_sub(other.0))
                }
            }
            impl SubAssign for Wrapping<$t> {
                #[inline]
                fn sub_assign(&mut self, other: Wrapping<$t>) {
                    *self = *self - other;
                }
            }
            impl Mul for Wrapping<$t> {
                type Output = Wrapping<$t>;
                #[inline]
                fn mul(self, other: Wrapping<$t>) -> Wrapping<$t> {
                    Wrapping(self.0.wrapping_mul(other.0))
                }
            }
            impl MulAssign for Wrapping<$t> {
                #[inline]
                fn mul_assign(&mut self, other: Wrapping<$t>) {
                    *self = *self * other;
                }
            }
            impl Neg for Wrapping<$t> {
                type Output = Wrapping<$t>;
                #[inline]
                fn neg(self) -> Wrapping<$t> {
                    Wrapping(self.0.wrapping_neg())
                }
            }
            impl Not for Wrapping<$t> {
                type Output = Wrapping<$t>;
                #[inline]
                fn not(self) -> Wrapping<$t> {
                    Wrapping(!self.0)
                }
            }
            impl BitAnd for Wrapping<$t> {
                type Output = Wrapping<$t>;
                #[inline]
                fn bitand(self, other: Wrapping<$t>) -> Wrapping<$t> {
                    Wrapping(self.0 & other.0)
                }
            }
            impl BitOr for Wrapping<$t> {
                type Output = Wrapping<$t>;
                #[inline]
                fn bitor(self, other: Wrapping<$t>) -> Wrapping<$t> {
                    Wrapping(self.0 | other.0)
                }
            }
            impl BitXor for Wrapping<$t> {
                type Output = Wrapping<$t>;
                #[inline]
                fn bitxor(self, other: Wrapping<$t>) -> Wrapping<$t> {
                    Wrapping(self.0 ^ other.0)
                }
            }
            impl Add for Saturating<$t> {
                type Output = Saturating<$t>;
                #[inline]
                fn add(self, other: Saturating<$t>) -> Saturating<$t> {
                    Saturating(self.0.saturating_add(other.0))
                }
            }
            impl AddAssign for Saturating<$t> {
                #[inline]
                fn add_assign(&mut self, other: Saturating<$t>) {
                    *self = *self + other;
                }
            }
            impl Sub for Saturating<$t> {
                type Output = Saturating<$t>;
                #[inline]
                fn sub(self, other: Saturating<$t>) -> Saturating<$t> {
                    Saturating(self.0.saturating_sub(other.0))
                }
            }
            impl SubAssign for Saturating<$t> {
                #[inline]
                fn sub_assign(&mut self, other: Saturating<$t>) {
                    *self = *self - other;
                }
            }
            impl Mul for Saturating<$t> {
                type Output = Saturating<$t>;
                #[inline]
                fn mul(self, other: Saturating<$t>) -> Saturating<$t> {
                    Saturating(self.0.saturating_mul(other.0))
                }
            }
            impl MulAssign for Saturating<$t> {
                #[inline]
                fn mul_assign(&mut self, other: Saturating<$t>) {
                    *self = *self * other;
                }
            }
        )*
    };
}

__std_wrapping_impl! { i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }
