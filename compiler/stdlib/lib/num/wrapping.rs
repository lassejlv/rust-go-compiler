//! `Wrapping<T>` and `Saturating<T>`: intentionally-wrapped/saturating arithmetic.

use crate::fmt;
use crate::iter::{Product, Sum};
use crate::ops::{
    Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div, DivAssign, Mul, MulAssign, Neg,
    Not, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign,
};

macro_rules! __std_fmt_forward {
    ($w:ident: $($tr:ident)*) => {
        $(
            impl<T: fmt::$tr> fmt::$tr for $w<T> {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    fmt::$tr::fmt(&self.0, f)
                }
            }
        )*
    };
}

__std_fmt_forward!(Wrapping: Binary Octal LowerHex UpperHex);
__std_fmt_forward!(Saturating: Binary Octal LowerHex UpperHex);


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


// One binary operator for `$w<$t>` with `$w<$t>` and plain `$t` right-hand sides.
macro_rules! __std_wbin {
    ($w:ident, $t:ty, $tr:ident, $m:ident, $atr:ident, $am:ident, |$a:ident, $b:ident| $e:expr) => {
        impl $tr for $w<$t> {
            type Output = $w<$t>;
            #[inline]
            fn $m(self, other: $w<$t>) -> $w<$t> {
                let $a = self.0;
                let $b = other.0;
                $w($e)
            }
        }
        impl<'a> $tr<&'a $w<$t>> for $w<$t> {
            type Output = $w<$t>;
            #[inline]
            fn $m(self, other: &'a $w<$t>) -> $w<$t> {
                self.$m(*other)
            }
        }
        impl<'a> $tr<$w<$t>> for &'a $w<$t> {
            type Output = $w<$t>;
            #[inline]
            fn $m(self, other: $w<$t>) -> $w<$t> {
                (*self).$m(other)
            }
        }
        impl $atr for $w<$t> {
            #[inline]
            fn $am(&mut self, other: $w<$t>) {
                *self = (*self).$m(other);
            }
        }
        impl<'a> $atr<&'a $w<$t>> for $w<$t> {
            #[inline]
            fn $am(&mut self, other: &'a $w<$t>) {
                *self = (*self).$m(*other);
            }
        }
        impl $atr<$t> for $w<$t> {
            #[inline]
            fn $am(&mut self, other: $t) {
                *self = (*self).$m($w(other));
            }
        }
    };
}

macro_rules! __std_wrapping_impl {
    ($($t:ident)*) => {
        $(
            __std_wbin!(Wrapping, $t, Add, add, AddAssign, add_assign, |a, b| a.wrapping_add(b));
            __std_wbin!(Wrapping, $t, Sub, sub, SubAssign, sub_assign, |a, b| a.wrapping_sub(b));
            __std_wbin!(Wrapping, $t, Mul, mul, MulAssign, mul_assign, |a, b| a.wrapping_mul(b));
            __std_wbin!(Wrapping, $t, Div, div, DivAssign, div_assign, |a, b| a.wrapping_div(b));
            __std_wbin!(Wrapping, $t, Rem, rem, RemAssign, rem_assign, |a, b| a.wrapping_rem(b));
            __std_wbin!(Wrapping, $t, BitAnd, bitand, BitAndAssign, bitand_assign, |a, b| a & b);
            __std_wbin!(Wrapping, $t, BitOr, bitor, BitOrAssign, bitor_assign, |a, b| a | b);
            __std_wbin!(Wrapping, $t, BitXor, bitxor, BitXorAssign, bitxor_assign, |a, b| a ^ b);
            __std_wbin!(Saturating, $t, Add, add, AddAssign, add_assign, |a, b| a.saturating_add(b));
            __std_wbin!(Saturating, $t, Sub, sub, SubAssign, sub_assign, |a, b| a.saturating_sub(b));
            __std_wbin!(Saturating, $t, Mul, mul, MulAssign, mul_assign, |a, b| a.saturating_mul(b));
            __std_wbin!(Saturating, $t, Div, div, DivAssign, div_assign, |a, b| a.saturating_div(b));
            __std_wbin!(Saturating, $t, Rem, rem, RemAssign, rem_assign, |a, b| a % b);
            __std_wbin!(Saturating, $t, BitAnd, bitand, BitAndAssign, bitand_assign, |a, b| a & b);
            __std_wbin!(Saturating, $t, BitOr, bitor, BitOrAssign, bitor_assign, |a, b| a | b);
            __std_wbin!(Saturating, $t, BitXor, bitxor, BitXorAssign, bitxor_assign, |a, b| a ^ b);

            impl Shl<usize> for Wrapping<$t> {
                type Output = Wrapping<$t>;
                #[inline]
                fn shl(self, other: usize) -> Wrapping<$t> {
                    Wrapping(self.0.wrapping_shl(other as u32))
                }
            }
            impl ShlAssign<usize> for Wrapping<$t> {
                #[inline]
                fn shl_assign(&mut self, other: usize) {
                    *self = *self << other;
                }
            }
            impl Shr<usize> for Wrapping<$t> {
                type Output = Wrapping<$t>;
                #[inline]
                fn shr(self, other: usize) -> Wrapping<$t> {
                    Wrapping(self.0.wrapping_shr(other as u32))
                }
            }
            impl ShrAssign<usize> for Wrapping<$t> {
                #[inline]
                fn shr_assign(&mut self, other: usize) {
                    *self = *self >> other;
                }
            }
            impl Neg for Wrapping<$t> {
                type Output = Wrapping<$t>;
                #[inline]
                fn neg(self) -> Wrapping<$t> {
                    Wrapping((0 as $t).wrapping_sub(self.0))
                }
            }
            impl Not for Wrapping<$t> {
                type Output = Wrapping<$t>;
                #[inline]
                fn not(self) -> Wrapping<$t> {
                    Wrapping(!self.0)
                }
            }
            impl Not for Saturating<$t> {
                type Output = Saturating<$t>;
                #[inline]
                fn not(self) -> Saturating<$t> {
                    Saturating(!self.0)
                }
            }
            impl Sum for Wrapping<$t> {
                fn sum<I: Iterator<Item = Wrapping<$t>>>(iter: I) -> Wrapping<$t> {
                    iter.fold(Wrapping(0), |a, b| a + b)
                }
            }
            impl<'a> Sum<&'a Wrapping<$t>> for Wrapping<$t> {
                fn sum<I: Iterator<Item = &'a Wrapping<$t>>>(iter: I) -> Wrapping<$t> {
                    iter.fold(Wrapping(0), |a, b| a + *b)
                }
            }
            impl Product for Wrapping<$t> {
                fn product<I: Iterator<Item = Wrapping<$t>>>(iter: I) -> Wrapping<$t> {
                    iter.fold(Wrapping(1), |a, b| a * b)
                }
            }
            impl<'a> Product<&'a Wrapping<$t>> for Wrapping<$t> {
                fn product<I: Iterator<Item = &'a Wrapping<$t>>>(iter: I) -> Wrapping<$t> {
                    iter.fold(Wrapping(1), |a, b| a * *b)
                }
            }
            impl Sum for Saturating<$t> {
                fn sum<I: Iterator<Item = Saturating<$t>>>(iter: I) -> Saturating<$t> {
                    iter.fold(Saturating(0), |a, b| a + b)
                }
            }

            impl Wrapping<$t> {
                /// Returns the smallest value that can be represented by this integer type.
                pub const MIN: Wrapping<$t> = Wrapping(<$t>::MIN);
                /// Returns the largest value that can be represented by this integer type.
                pub const MAX: Wrapping<$t> = Wrapping(<$t>::MAX);
                /// Returns the size of this integer type in bits.
                pub const BITS: u32 = <$t>::BITS;

                /// Returns the number of ones in the binary representation of `self`.
                #[inline]
                pub fn count_ones(self) -> u32 {
                    self.0.count_ones()
                }
                /// Returns the number of zeros in the binary representation of `self`.
                #[inline]
                pub fn count_zeros(self) -> u32 {
                    self.0.count_zeros()
                }
                /// Returns the number of trailing zeros in the binary representation of `self`.
                #[inline]
                pub fn trailing_zeros(self) -> u32 {
                    self.0.trailing_zeros()
                }
                /// Returns the number of leading zeros in the binary representation of `self`.
                #[inline]
                pub fn leading_zeros(self) -> u32 {
                    self.0.leading_zeros()
                }
                /// Shifts the bits to the left by a specified amount, wrapping the truncated bits to the end.
                #[inline]
                pub fn rotate_left(self, n: u32) -> Wrapping<$t> {
                    Wrapping(self.0.rotate_left(n))
                }
                /// Shifts the bits to the right by a specified amount, wrapping the truncated bits to the beginning.
                #[inline]
                pub fn rotate_right(self, n: u32) -> Wrapping<$t> {
                    Wrapping(self.0.rotate_right(n))
                }
                /// Reverses the byte order of the integer.
                #[inline]
                pub fn swap_bytes(self) -> Wrapping<$t> {
                    Wrapping(self.0.swap_bytes())
                }
                /// Raises self to the power of `exp`, using exponentiation by squaring.
                #[inline]
                pub fn pow(self, exp: u32) -> Wrapping<$t> {
                    Wrapping(self.0.wrapping_pow(exp))
                }
            }

            impl Saturating<$t> {
                /// Returns the smallest value that can be represented by this integer type.
                pub const MIN: Saturating<$t> = Saturating(<$t>::MIN);
                /// Returns the largest value that can be represented by this integer type.
                pub const MAX: Saturating<$t> = Saturating(<$t>::MAX);
                /// Returns the size of this integer type in bits.
                pub const BITS: u32 = <$t>::BITS;

                /// Raises self to the power of `exp`, saturating.
                #[inline]
                pub fn pow(self, exp: u32) -> Saturating<$t> {
                    Saturating(self.0.saturating_pow(exp))
                }
                /// Returns the number of ones in the binary representation of `self`.
                #[inline]
                pub fn count_ones(self) -> u32 {
                    self.0.count_ones()
                }
            }
        )*
    };
}

__std_wrapping_impl! { i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }

macro_rules! __std_wrapping_signed {
    ($($t:ident)*) => {
        $(
            impl Neg for Saturating<$t> {
                type Output = Saturating<$t>;
                #[inline]
                fn neg(self) -> Saturating<$t> {
                    Saturating(self.0.saturating_neg())
                }
            }
            impl Wrapping<$t> {
                /// Computes the absolute value of `self`, wrapping around at the boundary of the type.
                #[inline]
                pub fn abs(self) -> Wrapping<$t> {
                    Wrapping(self.0.wrapping_abs())
                }
                /// Returns a number representing sign of `self`.
                #[inline]
                pub fn signum(self) -> Wrapping<$t> {
                    Wrapping(self.0.signum())
                }
                /// Returns `true` if `self` is positive.
                #[inline]
                pub const fn is_positive(self) -> bool {
                    self.0 > 0
                }
                /// Returns `true` if `self` is negative.
                #[inline]
                pub const fn is_negative(self) -> bool {
                    self.0 < 0
                }
            }
            impl Saturating<$t> {
                /// Saturating absolute value.
                #[inline]
                pub fn abs(self) -> Saturating<$t> {
                    Saturating(self.0.saturating_abs())
                }
                /// Returns `true` if `self` is positive.
                #[inline]
                pub const fn is_positive(self) -> bool {
                    self.0 > 0
                }
                /// Returns `true` if `self` is negative.
                #[inline]
                pub const fn is_negative(self) -> bool {
                    self.0 < 0
                }
            }
        )*
    };
}

__std_wrapping_signed! { i8 i16 i32 i64 i128 isize }

macro_rules! __std_wrapping_unsigned {
    ($($t:ident)*) => {
        $(
            impl Wrapping<$t> {
                /// Returns `true` if and only if `self == 2^k` for some `k`.
                #[inline]
                pub fn is_power_of_two(self) -> bool {
                    self.0.is_power_of_two()
                }
            }
        )*
    };
}

__std_wrapping_unsigned! { u8 u16 u32 u64 u128 usize }
