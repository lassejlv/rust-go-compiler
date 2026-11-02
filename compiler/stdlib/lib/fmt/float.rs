//! Float formatting (the digit generation is done by the `fmt_f64` intrinsic).

use super::{Debug, Display, Formatter, LowerExp, Result, UpperExp};
use crate::intrinsics;

const MODE_DISPLAY: u8 = 0;
const MODE_DEBUG: u8 = 1;
const MODE_LOWER_EXP: u8 = 2;
const MODE_UPPER_EXP: u8 = 3;

/// Largest precision passed to the intrinsic (its buffer holds 1200 bytes);
/// larger precisions are padded with zeros. The caps are exact: an f64 has
/// at most 767 significant decimal digits, a value >= 1 has at most 52
/// fractional digits and a value < 1 at most 1074.
fn max_precision(v: f64, mode: u8) -> usize {
    if mode == MODE_LOWER_EXP || mode == MODE_UPPER_EXP {
        1180
    } else if v < 1.0 && v > -1.0 {
        1190
    } else {
        880
    }
}

fn fmt_float(f: &mut Formatter<'_>, v: f64, is_f32: bool, mode: u8) -> Result {
    let mut extra_zeros = 0usize;
    let cap = max_precision(v, mode);
    let prec: isize = match f.precision() {
        Some(p) => {
            if p > cap {
                extra_zeros = p - cap;
                cap as isize
            } else {
                p as isize
            }
        }
        None => -1,
    };
    let mode = if mode == MODE_DEBUG && prec >= 0 { MODE_DISPLAY } else { mode };
    let mut buf = [0u8; 1200];
    let len = unsafe { intrinsics::fmt_f64(v, is_f32, mode, prec, buf.as_mut_ptr()) };
    let s = unsafe { crate::str::from_raw_parts(buf.as_ptr(), len) };
    let is_nan = v != v;
    let (sign, body) = if len > 0 && buf[0] == b'-' {
        ("-", unsafe { s.get_unchecked(1..len) })
    } else if f.sign_plus() && !is_nan {
        ("+", s)
    } else {
        ("", s)
    };
    if extra_zeros > 0 && !is_nan && v - v == 0.0 {
        // Insert the extra zeros before any exponent part.
        let mut owned = crate::string::String::with_capacity(body.len() + extra_zeros);
        let epos = match body.find(|c: char| c == 'e' || c == 'E') {
            Some(i) => i,
            None => body.len(),
        };
        owned.push_str(unsafe { body.get_unchecked(0..epos) });
        let mut i = 0;
        while i < extra_zeros {
            owned.push('0');
            i += 1;
        }
        owned.push_str(unsafe { body.get_unchecked(epos..body.len()) });
        return f.pad_formatted_parts(sign, owned.as_str());
    }
    f.pad_formatted_parts(sign, body)
}

impl Display for f64 {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        fmt_float(f, *self, false, MODE_DISPLAY)
    }
}

impl Debug for f64 {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        fmt_float(f, *self, false, MODE_DEBUG)
    }
}

impl LowerExp for f64 {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        fmt_float(f, *self, false, MODE_LOWER_EXP)
    }
}

impl UpperExp for f64 {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        fmt_float(f, *self, false, MODE_UPPER_EXP)
    }
}

impl Display for f32 {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        fmt_float(f, *self as f64, true, MODE_DISPLAY)
    }
}

impl Debug for f32 {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        fmt_float(f, *self as f64, true, MODE_DEBUG)
    }
}

impl LowerExp for f32 {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        fmt_float(f, *self as f64, true, MODE_LOWER_EXP)
    }
}

impl UpperExp for f32 {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        fmt_float(f, *self as f64, true, MODE_UPPER_EXP)
    }
}
