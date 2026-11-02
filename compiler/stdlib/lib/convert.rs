//! Traits for conversions between types.

use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};

/// Used to do a cheap reference-to-reference conversion.
pub trait AsRef<T: ?Sized> {
    /// Converts this type into a shared reference of the (usually inferred) input type.
    fn as_ref(&self) -> &T;
}

/// Used to do a cheap mutable-to-mutable reference conversion.
pub trait AsMut<T: ?Sized> {
    /// Converts this type into a mutable reference of the (usually inferred) input type.
    fn as_mut(&mut self) -> &mut T;
}

/// A value-to-value conversion that consumes the input value. The opposite of `From`.
pub trait Into<T>: Sized {
    /// Converts this type into the (usually inferred) input type.
    fn into(self) -> T;
}

/// Used to do value-to-value conversions while consuming the input value.
#[lang = "from"]
pub trait From<T>: Sized {
    /// Converts to this type from the input type.
    fn from(value: T) -> Self;
}

/// An attempted conversion that consumes `self`, which may or may not be expensive.
pub trait TryInto<T>: Sized {
    /// The type returned in the event of a conversion error.
    type Error;
    /// Performs the conversion.
    fn try_into(self) -> Result<T, Self::Error>;
}

/// Simple and safe type conversions that may fail in a controlled way.
pub trait TryFrom<T>: Sized {
    /// The type returned in the event of a conversion error.
    type Error;
    /// Performs the conversion.
    fn try_from(value: T) -> Result<Self, Self::Error>;
}

// From implies Into
impl<T, U> Into<U> for T
where
    U: From<T>,
{
    #[inline]
    fn into(self) -> U {
        U::from(self)
    }
}

// From (and thus Into) is reflexive
impl<T> From<T> for T {
    #[inline]
    fn from(t: T) -> T {
        t
    }
}

// TryFrom implies TryInto
impl<T, U> TryInto<U> for T
where
    U: TryFrom<T>,
{
    type Error = U::Error;
    #[inline]
    fn try_into(self) -> Result<U, U::Error> {
        U::try_from(self)
    }
}

// Infallible conversions are semantically equivalent to fallible conversions
// with an uninhabited error type.
impl<T, U> TryFrom<U> for T
where
    U: Into<T>,
{
    type Error = Infallible;
    #[inline]
    fn try_from(value: U) -> Result<Self, Self::Error> {
        Ok(U::into(value))
    }
}

// AsRef / AsMut lift over references
impl<T: ?Sized, U: ?Sized> AsRef<U> for &T
where
    T: AsRef<U>,
{
    #[inline]
    fn as_ref(&self) -> &U {
        <T as AsRef<U>>::as_ref(*self)
    }
}

impl<T: ?Sized, U: ?Sized> AsRef<U> for &mut T
where
    T: AsRef<U>,
{
    #[inline]
    fn as_ref(&self) -> &U {
        <T as AsRef<U>>::as_ref(&**self)
    }
}

impl<T: ?Sized, U: ?Sized> AsMut<U> for &mut T
where
    T: AsMut<U>,
{
    #[inline]
    fn as_mut(&mut self) -> &mut U {
        (**self).as_mut()
    }
}

impl<T> AsRef<[T]> for [T] {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T> AsMut<[T]> for [T] {
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl AsRef<str> for str {
    #[inline]
    fn as_ref(&self) -> &str {
        self
    }
}

impl AsRef<[u8]> for str {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<T, const N: usize> AsRef<[T]> for [T; N] {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T, const N: usize> AsMut<[T]> for [T; N] {
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

/// The error type for errors that can never happen.
pub enum Infallible {}

impl Clone for Infallible {
    fn clone(&self) -> Infallible {
        match *self {}
    }
}

impl Copy for Infallible {}

impl fmt::Debug for Infallible {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

impl fmt::Display for Infallible {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

impl PartialEq for Infallible {
    fn eq(&self, _: &Infallible) -> bool {
        match *self {}
    }
}

impl Eq for Infallible {}

impl PartialOrd for Infallible {
    fn partial_cmp(&self, _other: &Self) -> Option<Ordering> {
        match *self {}
    }
}

impl Ord for Infallible {
    fn cmp(&self, _other: &Self) -> Ordering {
        match *self {}
    }
}

impl Hash for Infallible {
    fn hash<H: Hasher>(&self, _: &mut H) {
        match *self {}
    }
}

impl crate::error::Error for Infallible {}

/// Identity function.
#[inline]
pub const fn identity<T>(x: T) -> T {
    x
}

// ---------------------------------------------------------------------------
// Numeric conversions

macro_rules! __std_convert_from_lossless {
    ($src:ty => $($dst:ty)*) => {
        $(
            impl From<$src> for $dst {
                #[inline]
                fn from(small: $src) -> $dst {
                    small as $dst
                }
            }
        )*
    };
}

__std_convert_from_lossless! { u8 => u16 u32 u64 u128 usize i16 i32 i64 i128 isize f32 f64 }
__std_convert_from_lossless! { u16 => u32 u64 u128 usize i32 i64 i128 f32 f64 }
__std_convert_from_lossless! { u32 => u64 u128 i64 i128 f64 }
__std_convert_from_lossless! { u64 => u128 i128 }
__std_convert_from_lossless! { i8 => i16 i32 i64 i128 isize f32 f64 }
__std_convert_from_lossless! { i16 => i32 i64 i128 isize f32 f64 }
__std_convert_from_lossless! { i32 => i64 i128 f64 }
__std_convert_from_lossless! { i64 => i128 }
__std_convert_from_lossless! { f32 => f64 }

macro_rules! __std_convert_from_bool {
    ($($dst:ty)*) => {
        $(
            impl From<bool> for $dst {
                #[inline]
                fn from(b: bool) -> $dst {
                    b as u8 as $dst
                }
            }
        )*
    };
}

__std_convert_from_bool! { u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 }

impl From<u8> for char {
    #[inline]
    fn from(i: u8) -> char {
        i as char
    }
}

impl From<char> for u32 {
    #[inline]
    fn from(c: char) -> u32 {
        c as u32
    }
}

impl From<char> for u64 {
    #[inline]
    fn from(c: char) -> u64 {
        c as u64
    }
}

impl From<char> for u128 {
    #[inline]
    fn from(c: char) -> u128 {
        c as u128
    }
}

/// The error type returned when a checked integral type conversion fails.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TryFromIntError(pub(crate) crate::num::IntErrorKind);

impl TryFromIntError {
    /// Outputs the detailed cause of the conversion failing.
    pub const fn kind(&self) -> &crate::num::IntErrorKind {
        &self.0
    }
}

impl fmt::Display for TryFromIntError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("out of range integral type conversion attempted")
    }
}

impl crate::error::Error for TryFromIntError {}

impl From<Infallible> for TryFromIntError {
    fn from(x: Infallible) -> TryFromIntError {
        match x {}
    }
}

macro_rules! __std_convert_try_from_int {
    ($src:ty => $($dst:ty)*) => {
        $(
            impl TryFrom<$src> for $dst {
                type Error = TryFromIntError;
                #[inline]
                fn try_from(u: $src) -> Result<$dst, TryFromIntError> {
                    let x = u as $dst;
                    if (x as $src) == u && ((x < (0 as $dst)) == (u < (0 as $src))) {
                        Ok(x)
                    } else if u < (0 as $src) {
                        Err(TryFromIntError(crate::num::IntErrorKind::NegOverflow))
                    } else {
                        Err(TryFromIntError(crate::num::IntErrorKind::PosOverflow))
                    }
                }
            }
        )*
    };
}

// Every integer pair that is not covered by a lossless `From` impl.
__std_convert_try_from_int! { u8 => i8 }
__std_convert_try_from_int! { u16 => u8 i8 i16 isize }
__std_convert_try_from_int! { u32 => u8 u16 usize i8 i16 i32 isize }
__std_convert_try_from_int! { u64 => u8 u16 u32 usize i8 i16 i32 i64 isize }
__std_convert_try_from_int! { u128 => u8 u16 u32 u64 usize i8 i16 i32 i64 i128 isize }
__std_convert_try_from_int! { usize => u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 isize }
__std_convert_try_from_int! { i8 => u8 u16 u32 u64 u128 usize }
__std_convert_try_from_int! { i16 => i8 u8 u16 u32 u64 u128 usize }
__std_convert_try_from_int! { i32 => i8 i16 isize u8 u16 u32 u64 u128 usize }
__std_convert_try_from_int! { i64 => i8 i16 i32 isize u8 u16 u32 u64 u128 usize }
__std_convert_try_from_int! { i128 => i8 i16 i32 i64 isize u8 u16 u32 u64 u128 usize }
__std_convert_try_from_int! { isize => i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 usize }
