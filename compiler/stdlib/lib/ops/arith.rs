//! Arithmetic operator traits.

/// The addition operator `+`.
#[lang = "add"]
pub trait Add<Rhs = Self> {
    /// The resulting type after applying the `+` operator.
    type Output;
    /// Performs the `+` operation.
    fn add(self, rhs: Rhs) -> Self::Output;
}

/// The subtraction operator `-`.
#[lang = "sub"]
pub trait Sub<Rhs = Self> {
    type Output;
    fn sub(self, rhs: Rhs) -> Self::Output;
}

/// The multiplication operator `*`.
#[lang = "mul"]
pub trait Mul<Rhs = Self> {
    type Output;
    fn mul(self, rhs: Rhs) -> Self::Output;
}

/// The division operator `/`.
#[lang = "div"]
pub trait Div<Rhs = Self> {
    type Output;
    fn div(self, rhs: Rhs) -> Self::Output;
}

/// The remainder operator `%`.
#[lang = "rem"]
pub trait Rem<Rhs = Self> {
    type Output;
    fn rem(self, rhs: Rhs) -> Self::Output;
}

/// The unary negation operator `-`.
#[lang = "neg"]
pub trait Neg {
    type Output;
    fn neg(self) -> Self::Output;
}

/// The addition assignment operator `+=`.
#[lang = "add_assign"]
pub trait AddAssign<Rhs = Self> {
    fn add_assign(&mut self, rhs: Rhs);
}

/// The subtraction assignment operator `-=`.
#[lang = "sub_assign"]
pub trait SubAssign<Rhs = Self> {
    fn sub_assign(&mut self, rhs: Rhs);
}

/// The multiplication assignment operator `*=`.
#[lang = "mul_assign"]
pub trait MulAssign<Rhs = Self> {
    fn mul_assign(&mut self, rhs: Rhs);
}

/// The division assignment operator `/=`.
#[lang = "div_assign"]
pub trait DivAssign<Rhs = Self> {
    fn div_assign(&mut self, rhs: Rhs);
}

/// The remainder assignment operator `%=`.
#[lang = "rem_assign"]
pub trait RemAssign<Rhs = Self> {
    fn rem_assign(&mut self, rhs: Rhs);
}

// Implements a binary operator for a primitive type: by value and all
// reference combinations.
macro_rules! __std_ops_binop {
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

// Implements a compound assignment operator for a primitive type (by value
// and by reference right-hand side).
macro_rules! __std_ops_assignop {
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

__std_ops_binop! { Add add +; i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 }
__std_ops_binop! { Sub sub -; i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 }
__std_ops_binop! { Mul mul *; i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 }
__std_ops_binop! { Div div /; i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 }
__std_ops_binop! { Rem rem %; i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 }

__std_ops_assignop! { AddAssign add_assign +; i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 }
__std_ops_assignop! { SubAssign sub_assign -; i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 }
__std_ops_assignop! { MulAssign mul_assign *; i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 }
__std_ops_assignop! { DivAssign div_assign /; i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 }
__std_ops_assignop! { RemAssign rem_assign %; i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 }

macro_rules! __std_ops_neg {
    ($($t:ty)*) => {
        $(
            impl Neg for $t {
                type Output = $t;
                #[inline]
                fn neg(self) -> $t {
                    -self
                }
            }
            impl<'a> Neg for &'a $t {
                type Output = $t;
                #[inline]
                fn neg(self) -> $t {
                    -(*self)
                }
            }
        )*
    };
}

__std_ops_neg! { i8 i16 i32 i64 i128 isize f32 f64 }
