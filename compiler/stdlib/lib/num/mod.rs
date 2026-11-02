//! Numeric traits and functions for the built-in numeric types.

use crate::error::Error;
use crate::fmt;
use crate::str::FromStr;

mod float;
mod int;
mod nonzero;
mod wrapping;

pub use self::float::FpCategory;
pub use self::wrapping::{Saturating, Wrapping};
pub use self::nonzero::{
    NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64,
    NonZeroU8, NonZeroUsize, NonZero, ZeroablePrimitive,
};
pub use crate::convert::TryFromIntError;

/// Enum to store the various types of errors that can cause parsing an integer to fail.
#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash)]
#[non_exhaustive]
pub enum IntErrorKind {
    /// Value being parsed is empty.
    Empty,
    /// Contains an invalid digit in its context.
    InvalidDigit,
    /// Integer is too large to store in target integer type.
    PosOverflow,
    /// Integer is too small to store in target integer type.
    NegOverflow,
    /// Value was Zero.
    Zero,
}

/// An error which can be returned when parsing an integer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseIntError {
    pub(crate) kind: IntErrorKind,
}

impl ParseIntError {
    /// Outputs the detailed cause of parsing an integer failing.
    pub const fn kind(&self) -> &IntErrorKind {
        &self.kind
    }

    fn description(&self) -> &str {
        match self.kind {
            IntErrorKind::Empty => "cannot parse integer from empty string",
            IntErrorKind::InvalidDigit => "invalid digit found in string",
            IntErrorKind::PosOverflow => "number too large to fit in target type",
            IntErrorKind::NegOverflow => "number too small to fit in target type",
            IntErrorKind::Zero => "number would be zero for non-zero type",
        }
    }
}

impl fmt::Display for ParseIntError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.description())
    }
}

impl Error for ParseIntError {
    #[allow(deprecated)]
    fn description(&self) -> &str {
        ParseIntError::description(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FloatErrorKind {
    Empty,
    Invalid,
}

/// An error which can be returned when parsing a float.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseFloatError {
    kind: FloatErrorKind,
}

impl fmt::Display for ParseFloatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self.kind {
            FloatErrorKind::Empty => "cannot parse float from empty string",
            FloatErrorKind::Invalid => "invalid float literal",
        };
        f.pad(s)
    }
}

impl Error for ParseFloatError {}

fn parse_float_bits(src: &str) -> Result<f64, ParseFloatError> {
    if src.is_empty() {
        return Err(ParseFloatError { kind: FloatErrorKind::Empty });
    }
    let mut out: f64 = 0.0;
    let ok = unsafe { crate::intrinsics::parse_f64(src.as_ptr(), src.len(), &mut out as *mut f64) };
    if ok {
        Ok(out)
    } else {
        Err(ParseFloatError { kind: FloatErrorKind::Invalid })
    }
}

impl FromStr for f64 {
    type Err = ParseFloatError;
    fn from_str(src: &str) -> Result<f64, ParseFloatError> {
        parse_float_bits(src)
    }
}

impl FromStr for f32 {
    type Err = ParseFloatError;
    fn from_str(src: &str) -> Result<f32, ParseFloatError> {
        match parse_float_bits(src) {
            Ok(v) => Ok(v as f32),
            Err(e) => Err(e),
        }
    }
}

/// Converts an ASCII byte to its digit value in `radix`, if any.
#[inline]
pub(crate) fn ascii_digit_value(b: u8, radix: u32) -> Option<u32> {
    let d = if b >= b'0' && b <= b'9' {
        (b - b'0') as u32
    } else if b >= b'a' && b <= b'z' {
        (b - b'a') as u32 + 10
    } else if b >= b'A' && b <= b'Z' {
        (b - b'A') as u32 + 10
    } else {
        return None;
    };
    if d < radix {
        Some(d)
    } else {
        None
    }
}

#[inline(never)]
pub(crate) fn from_str_radix_panic(radix: u32) -> ! {
    crate::rt::panic_fmt(
        format_args!("from_ascii_bytes_radix: radix must lie in the range `[2, 36]` - found {}", radix),
        file!(),
        line!(),
        column!(),
    )
}

/// The error for parsing ints: shortcut constructor.
#[inline]
pub(crate) fn pie(kind: IntErrorKind) -> ParseIntError {
    ParseIntError { kind }
}
