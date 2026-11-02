//! Bitwise operator traits.

/// The unary logical negation operator `!`.
#[lang = "not"]
pub trait Not {
    type Output;
    fn not(self) -> Self::Output;
}

/// The bitwise AND operator `&`.
#[lang = "bitand"]
pub trait BitAnd<Rhs = Self> {
    type Output;
    fn bitand(self, rhs: Rhs) -> Self::Output;
}

/// The bitwise OR operator `|`.
#[lang = "bitor"]
pub trait BitOr<Rhs = Self> {
    type Output;
    fn bitor(self, rhs: Rhs) -> Self::Output;
}

/// The bitwise XOR operator `^`.
#[lang = "bitxor"]
pub trait BitXor<Rhs = Self> {
    type Output;
    fn bitxor(self, rhs: Rhs) -> Self::Output;
}

/// The left shift operator `<<`.
#[lang = "shl"]
pub trait Shl<Rhs = Self> {
    type Output;
    fn shl(self, rhs: Rhs) -> Self::Output;
}

/// The right shift operator `>>`.
#[lang = "shr"]
pub trait Shr<Rhs = Self> {
    type Output;
    fn shr(self, rhs: Rhs) -> Self::Output;
}

/// The bitwise AND assignment operator `&=`.
#[lang = "bitand_assign"]
pub trait BitAndAssign<Rhs = Self> {
    fn bitand_assign(&mut self, rhs: Rhs);
}

/// The bitwise OR assignment operator `|=`.
#[lang = "bitor_assign"]
pub trait BitOrAssign<Rhs = Self> {
    fn bitor_assign(&mut self, rhs: Rhs);
}

/// The bitwise XOR assignment operator `^=`.
#[lang = "bitxor_assign"]
pub trait BitXorAssign<Rhs = Self> {
    fn bitxor_assign(&mut self, rhs: Rhs);
}

/// The left shift assignment operator `<<=`.
#[lang = "shl_assign"]
pub trait ShlAssign<Rhs = Self> {
    fn shl_assign(&mut self, rhs: Rhs);
}

/// The right shift assignment operator `>>=`.
#[lang = "shr_assign"]
pub trait ShrAssign<Rhs = Self> {
    fn shr_assign(&mut self, rhs: Rhs);
}

macro_rules! __std_bit_not {
    ($($t:ty)*) => {
        $(
            impl Not for $t {
                type Output = $t;
                #[inline]
                fn not(self) -> $t {
                    !self
                }
            }
            impl<'a> Not for &'a $t {
                type Output = $t;
                #[inline]
                fn not(self) -> $t {
                    !(*self)
                }
            }
        )*
    };
}

__std_bit_not! { bool i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }

macro_rules! __std_bit_binop {
    ($Trait:ident $method:ident $op:tt; $($t:ty)*) => {
        $(
            impl $Trait for $t {
                type Output = $t;
                #[inline]
                fn $method(self, other: $t) -> $t {
                    self $op other
                }
            }
            impl<'a> $Trait<$t> for &'a $t {
                type Output = $t;
                #[inline]
                fn $method(self, other: $t) -> $t {
                    (*self) $op other
                }
            }
            impl<'a> $Trait<&'a $t> for $t {
                type Output = $t;
                #[inline]
                fn $method(self, other: &'a $t) -> $t {
                    self $op (*other)
                }
            }
            impl<'a, 'b> $Trait<&'a $t> for &'b $t {
                type Output = $t;
                #[inline]
                fn $method(self, other: &'a $t) -> $t {
                    (*self) $op (*other)
                }
            }
        )*
    };
}

macro_rules! __std_bit_assignop {
    ($Trait:ident $method:ident $op:tt; $($t:ty)*) => {
        $(
            impl $Trait for $t {
                #[inline]
                fn $method(&mut self, other: $t) {
                    *self = (*self) $op other;
                }
            }
            impl<'a> $Trait<&'a $t> for $t {
                #[inline]
                fn $method(&mut self, other: &'a $t) {
                    *self = (*self) $op (*other);
                }
            }
        )*
    };
}

__std_bit_binop! { BitAnd bitand &; bool i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }
__std_bit_binop! { BitOr bitor |; bool i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }
__std_bit_binop! { BitXor bitxor ^; bool i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }
__std_bit_assignop! { BitAndAssign bitand_assign &; bool i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }
__std_bit_assignop! { BitOrAssign bitor_assign |; bool i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }
__std_bit_assignop! { BitXorAssign bitxor_assign ^; bool i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }

// Shifts: same-type right-hand side (with reference forms) for every
// integer type.
__std_bit_binop! { Shl shl <<; i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }
__std_bit_binop! { Shr shr >>; i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }
__std_bit_assignop! { ShlAssign shl_assign <<; i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }
__std_bit_assignop! { ShrAssign shr_assign >>; i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }

// Shifts with a different right-hand side integer type.
macro_rules! __std_bit_shift_mixed {
    ($t:ty; $($r:ty)*) => {
        $(
            impl Shl<$r> for $t {
                type Output = $t;
                #[inline]
                fn shl(self, other: $r) -> $t {
                    self << other
                }
            }
            impl<'a> Shl<$r> for &'a $t {
                type Output = $t;
                #[inline]
                fn shl(self, other: $r) -> $t {
                    (*self) << other
                }
            }
            impl Shr<$r> for $t {
                type Output = $t;
                #[inline]
                fn shr(self, other: $r) -> $t {
                    self >> other
                }
            }
            impl<'a> Shr<$r> for &'a $t {
                type Output = $t;
                #[inline]
                fn shr(self, other: $r) -> $t {
                    (*self) >> other
                }
            }
            impl ShlAssign<$r> for $t {
                #[inline]
                fn shl_assign(&mut self, other: $r) {
                    *self = (*self) << other;
                }
            }
            impl ShrAssign<$r> for $t {
                #[inline]
                fn shr_assign(&mut self, other: $r) {
                    *self = (*self) >> other;
                }
            }
        )*
    };
}

__std_bit_shift_mixed! { i8; i32 u32 usize }
__std_bit_shift_mixed! { i16; i32 u32 usize }
__std_bit_shift_mixed! { i32; u32 usize }
__std_bit_shift_mixed! { i64; i32 u32 usize }
__std_bit_shift_mixed! { i128; i32 u32 usize }
__std_bit_shift_mixed! { isize; i32 u32 usize }
__std_bit_shift_mixed! { u8; i32 u32 usize }
__std_bit_shift_mixed! { u16; i32 u32 usize }
__std_bit_shift_mixed! { u32; i32 usize }
__std_bit_shift_mixed! { u64; i32 u32 usize }
__std_bit_shift_mixed! { u128; i32 u32 usize }
__std_bit_shift_mixed! { usize; i32 u32 }
