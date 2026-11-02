//! Integer formatting.

use super::{Binary, Debug, Display, Formatter, LowerExp, LowerHex, Octal, Result, UpperExp, UpperHex};

#[inline]
fn digits_str(buf: &[u8], start: usize, end: usize) -> &str {
    unsafe { crate::str::from_raw_parts(buf.as_ptr().add(start), end - start) }
}

/// Formats an unsigned magnitude in decimal with the given sign.
#[inline]
pub(crate) fn fmt_u64(mut n: u64, is_nonnegative: bool, f: &mut Formatter<'_>) -> Result {
    let mut buf = [0u8; 20];
    let mut curr = 20;
    loop {
        curr -= 1;
        buf[curr] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    f.pad_integral(is_nonnegative, "", digits_str(&buf, curr, 20))
}

/// Formats an unsigned 128-bit magnitude in decimal with the given sign.
pub(crate) fn fmt_u128(mut n: u128, is_nonnegative: bool, f: &mut Formatter<'_>) -> Result {
    if n <= (u64::MAX as u128) {
        return fmt_u64(n as u64, is_nonnegative, f);
    }
    let mut buf = [0u8; 40];
    let mut curr = 40;
    loop {
        curr -= 1;
        buf[curr] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    f.pad_integral(is_nonnegative, "", digits_str(&buf, curr, 40))
}

/// Formats `x` in base 2, 8 or 16 (`shift` = log2 of the base).
fn fmt_radix(mut x: u128, shift: u32, upper: bool, prefix: &str, f: &mut Formatter<'_>) -> Result {
    let mut buf = [0u8; 128];
    let mut curr = 128;
    let mask: u128 = (1u128 << shift) - 1;
    loop {
        let d = (x & mask) as u8;
        x >>= shift;
        curr -= 1;
        buf[curr] = if d < 10 {
            b'0' + d
        } else if upper {
            b'A' + (d - 10)
        } else {
            b'a' + (d - 10)
        };
        if x == 0 {
            break;
        }
    }
    f.pad_integral(true, prefix, digits_str(&buf, curr, 128))
}

/// Formats an integer magnitude in exponential notation (`{:e}`), following
/// the real standard library's algorithm.
fn fmt_exp(mut n: u128, is_nonnegative: bool, upper: bool, f: &mut Formatter<'_>) -> Result {
    let mut exponent: usize = 0;
    // count and remove trailing decimal zeroes
    while n % 10 == 0 && n >= 10 {
        n /= 10;
        exponent += 1;
    }
    let (added_precision, subtracted_precision) = match f.precision() {
        Some(fmt_prec) => {
            // number of decimal digits minus 1
            let mut tmp = n;
            let mut prec = 0;
            while tmp >= 10 {
                tmp /= 10;
                prec += 1;
            }
            (fmt_prec.saturating_sub(prec), prec.saturating_sub(fmt_prec))
        }
        None => (0, 0),
    };
    let mut i = 1;
    while i < subtracted_precision {
        n /= 10;
        exponent += 1;
        i += 1;
    }
    if subtracted_precision != 0 {
        let rem = n % 10;
        n /= 10;
        exponent += 1;
        // round up last digit, round to even on a tie
        if rem > 5 || (rem == 5 && (n % 2 != 0 || subtracted_precision > 1)) {
            let before = n;
            n += 1;
            // if the digit is rounded to the next power instead adjust the exponent
            if n.ilog10() > before.ilog10() {
                n /= 10;
                exponent += 1;
            }
        }
    }
    // 39 digits (worst case u128) + . = 40
    let mut buf = [0u8; 40];
    let mut curr = 40;
    // decode 2 chars at a time
    while n >= 100 {
        let d = (n % 100) as u8;
        curr -= 2;
        buf[curr] = b'0' + d / 10;
        buf[curr + 1] = b'0' + d % 10;
        n /= 100;
        exponent += 2;
    }
    // n is <= 99, so at most 2 chars long
    let mut n = n as u8;
    // decode second-to-last character
    if n >= 10 {
        curr -= 1;
        buf[curr] = (n % 10) + b'0';
        n /= 10;
        exponent += 1;
    }
    // add decimal point iff >1 mantissa digit will be printed
    let has_point = curr != 40 || added_precision != 0;
    if has_point {
        curr -= 1;
        buf[curr] = b'.';
    }
    // decode last character
    curr -= 1;
    buf[curr] = n + b'0';

    let mut s = crate::string::String::with_capacity(48 + added_precision);
    s.push_str(digits_str(&buf, curr, 40));
    let mut k = 0;
    while k < added_precision {
        s.push('0');
        k += 1;
    }
    s.push(if upper { 'E' } else { 'e' });
    // exponent digits
    let mut exp_buf = [0u8; 20];
    let mut ecur = 20;
    let mut e = exponent;
    loop {
        ecur -= 1;
        exp_buf[ecur] = b'0' + (e % 10) as u8;
        e /= 10;
        if e == 0 {
            break;
        }
    }
    s.push_str(digits_str(&exp_buf, ecur, 20));

    let sign = if !is_nonnegative {
        "-"
    } else if f.sign_plus() {
        "+"
    } else {
        ""
    };
    f.pad_formatted_parts(sign, s.as_str())
}

macro_rules! __std_fmt_int_signed {
    ($($t:ty, $ut:ty, $wide:ident);* $(;)?) => {
        $(
            impl Display for $t {
                #[inline]
                fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                    let is_nonnegative = *self >= 0;
                    let n = (*self as $ut) as $wide;
                    let n = if is_nonnegative { n } else { ((!(*self as $ut)) as $wide) + 1 };
                    __std_fmt_wide!($wide, n, is_nonnegative, f)
                }
            }
            impl LowerExp for $t {
                fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                    let is_nonnegative = *self >= 0;
                    let n = if is_nonnegative { *self as $ut as u128 } else { ((!(*self as $ut)) as u128) + 1 };
                    fmt_exp(n, is_nonnegative, false, f)
                }
            }
            impl UpperExp for $t {
                fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                    let is_nonnegative = *self >= 0;
                    let n = if is_nonnegative { *self as $ut as u128 } else { ((!(*self as $ut)) as u128) + 1 };
                    fmt_exp(n, is_nonnegative, true, f)
                }
            }
            __std_fmt_int_radix! { $t, $ut }
        )*
    };
}

macro_rules! __std_fmt_wide {
    (u64, $n:expr, $nn:expr, $f:expr) => {
        fmt_u64($n, $nn, $f)
    };
    (u128, $n:expr, $nn:expr, $f:expr) => {
        fmt_u128($n, $nn, $f)
    };
}

macro_rules! __std_fmt_int_unsigned {
    ($($t:ty, $wide:ident);* $(;)?) => {
        $(
            impl Display for $t {
                #[inline]
                fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                    __std_fmt_wide!($wide, *self as $wide, true, f)
                }
            }
            impl LowerExp for $t {
                fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                    fmt_exp(*self as u128, true, false, f)
                }
            }
            impl UpperExp for $t {
                fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                    fmt_exp(*self as u128, true, true, f)
                }
            }
            __std_fmt_int_radix! { $t, $t }
        )*
    };
}

macro_rules! __std_fmt_int_radix {
    ($t:ty, $ut:ty) => {
        impl Debug for $t {
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                if f.debug_lower_hex() {
                    LowerHex::fmt(self, f)
                } else if f.debug_upper_hex() {
                    UpperHex::fmt(self, f)
                } else {
                    Display::fmt(self, f)
                }
            }
        }
        impl LowerHex for $t {
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                fmt_radix(*self as $ut as u128, 4, false, "0x", f)
            }
        }
        impl UpperHex for $t {
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                fmt_radix(*self as $ut as u128, 4, true, "0x", f)
            }
        }
        impl Octal for $t {
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                fmt_radix(*self as $ut as u128, 3, false, "0o", f)
            }
        }
        impl Binary for $t {
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                fmt_radix(*self as $ut as u128, 1, false, "0b", f)
            }
        }
    };
}

__std_fmt_int_signed! {
    i8, u8, u64;
    i16, u16, u64;
    i32, u32, u64;
    i64, u64, u64;
    isize, usize, u64;
    i128, u128, u128;
}

__std_fmt_int_unsigned! {
    u8, u64;
    u16, u64;
    u32, u64;
    u64, u64;
    usize, u64;
    u128, u128;
}
