//! `Debug`/`Display` implementations for primitive types and references.

use super::{Binary, Debug, Display, Formatter, LowerExp, LowerHex, Octal, Pointer, Result, UpperExp, UpperHex};

// ---------------------------------------------------------------------------
// References forward every formatting trait.

macro_rules! __std_fmt_ref_forward {
    ($($tr:ident)*) => {
        $(
            impl<T: ?Sized + $tr> $tr for &T {
                #[inline]
                fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                    $tr::fmt(&**self, f)
                }
            }
            impl<T: ?Sized + $tr> $tr for &mut T {
                #[inline]
                fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                    $tr::fmt(&**self, f)
                }
            }
        )*
    };
}

__std_fmt_ref_forward! { Debug Display Octal Binary LowerHex UpperHex LowerExp UpperExp }

impl<T: ?Sized> Pointer for &T {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        super::pointer_fmt_inner(*self as *const T as *const u8 as usize, f)
    }
}

impl<T: ?Sized> Pointer for &mut T {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        super::pointer_fmt_inner(&**self as *const T as *const u8 as usize, f)
    }
}

// ---------------------------------------------------------------------------
// bool, str, char, (), !

impl Display for bool {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Display::fmt(if *self { "true" } else { "false" }, f)
    }
}

impl Debug for bool {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Display::fmt(self, f)
    }
}

impl Display for str {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.pad(self)
    }
}

impl Debug for str {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_char('"')?;
        let bytes = self.as_bytes();
        let len = bytes.len();
        let mut from = 0;
        let mut i = 0;
        while i < len {
            let b = bytes[i];
            if b >= 0x20 && b < 0x7F && b != b'"' && b != b'\\' {
                // printable ASCII: no escape needed
                i += 1;
                continue;
            }
            let (c, clen) = crate::str::decode_at(self, i);
            if crate::char::needs_debug_escape(c, false, true) {
                f.write_str(unsafe { self.get_unchecked(from..i) })?;
                crate::char::write_escape_debug(f, c, false, true)?;
                from = i + clen;
            }
            i += clen;
        }
        f.write_str(unsafe { self.get_unchecked(from..len) })?;
        f.write_char('"')
    }
}

impl Display for char {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if f.width().is_none() && f.precision().is_none() {
            f.write_char(*self)
        } else {
            let mut b = [0u8; 4];
            let s = self.encode_utf8(&mut b);
            f.pad(&*s)
        }
    }
}

impl Debug for char {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_char('\'')?;
        crate::char::write_escape_debug(f, *self, true, false)?;
        f.write_char('\'')
    }
}

impl Debug for ! {
    fn fmt(&self, _: &mut Formatter<'_>) -> Result {
        *self
    }
}

impl Display for ! {
    fn fmt(&self, _: &mut Formatter<'_>) -> Result {
        *self
    }
}

impl Debug for () {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.pad("()")
    }
}

// ---------------------------------------------------------------------------
// Slices

impl<T: Debug> Debug for [T] {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_list().entries(self.iter()).finish()
    }
}
