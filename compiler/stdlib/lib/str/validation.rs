//! UTF-8 validation.

use crate::fmt;

/// Errors which can occur when attempting to interpret a sequence of `u8` as a string.
#[derive(Copy, Eq, PartialEq, Clone, Debug)]
pub struct Utf8Error {
    pub(crate) valid_up_to: usize,
    pub(crate) error_len: Option<u8>,
}

impl Utf8Error {
    /// Returns the index in the given string up to which valid UTF-8 was verified.
    #[inline]
    pub const fn valid_up_to(&self) -> usize {
        self.valid_up_to
    }

    /// Provides more information about the failure.
    #[inline]
    pub fn error_len(&self) -> Option<usize> {
        match self.error_len {
            Some(len) => Some(len as usize),
            None => None,
        }
    }
}

impl fmt::Display for Utf8Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(error_len) = self.error_len {
            write!(f, "invalid utf-8 sequence of {} bytes from index {}", error_len, self.valid_up_to)
        } else {
            write!(f, "incomplete utf-8 byte sequence from index {}", self.valid_up_to)
        }
    }
}

impl crate::error::Error for Utf8Error {}

/// Given a first byte, determines how many bytes are in this UTF-8 character.
#[inline]
pub(crate) const fn utf8_char_width(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b < 0xC2 {
        0
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else if b < 0xF5 {
        4
    } else {
        0
    }
}

#[inline]
fn is_cont(b: u8) -> bool {
    (b & 0xC0) == 0x80
}

/// Validates UTF-8, returning the same errors as the real standard library.
pub(crate) fn run_utf8_validation(v: &[u8]) -> Result<(), Utf8Error> {
    let len = v.len();
    let mut index = 0;
    while index < len {
        let old_offset = index;
        let first = v[index];
        if first >= 128 {
            let w = utf8_char_width(first);
            if w == 2 {
                if index + 1 >= len {
                    return Err(Utf8Error { valid_up_to: old_offset, error_len: None });
                }
                index += 1;
                if !is_cont(v[index]) {
                    return Err(Utf8Error { valid_up_to: old_offset, error_len: Some(1) });
                }
            } else if w == 3 {
                if index + 1 >= len {
                    return Err(Utf8Error { valid_up_to: old_offset, error_len: None });
                }
                index += 1;
                let second = v[index];
                let ok = match first {
                    0xE0 => second >= 0xA0 && second <= 0xBF,
                    0xED => second >= 0x80 && second <= 0x9F,
                    _ => second >= 0x80 && second <= 0xBF,
                };
                if !ok {
                    return Err(Utf8Error { valid_up_to: old_offset, error_len: Some(1) });
                }
                if index + 1 >= len {
                    return Err(Utf8Error { valid_up_to: old_offset, error_len: None });
                }
                index += 1;
                if !is_cont(v[index]) {
                    return Err(Utf8Error { valid_up_to: old_offset, error_len: Some(2) });
                }
            } else if w == 4 {
                if index + 1 >= len {
                    return Err(Utf8Error { valid_up_to: old_offset, error_len: None });
                }
                index += 1;
                let second = v[index];
                let ok = match first {
                    0xF0 => second >= 0x90 && second <= 0xBF,
                    0xF4 => second >= 0x80 && second <= 0x8F,
                    _ => second >= 0x80 && second <= 0xBF,
                };
                if !ok {
                    return Err(Utf8Error { valid_up_to: old_offset, error_len: Some(1) });
                }
                if index + 1 >= len {
                    return Err(Utf8Error { valid_up_to: old_offset, error_len: None });
                }
                index += 1;
                if !is_cont(v[index]) {
                    return Err(Utf8Error { valid_up_to: old_offset, error_len: Some(2) });
                }
                if index + 1 >= len {
                    return Err(Utf8Error { valid_up_to: old_offset, error_len: None });
                }
                index += 1;
                if !is_cont(v[index]) {
                    return Err(Utf8Error { valid_up_to: old_offset, error_len: Some(3) });
                }
            } else {
                return Err(Utf8Error { valid_up_to: old_offset, error_len: Some(1) });
            }
        }
        index += 1;
    }
    Ok(())
}

/// Converts a slice of bytes to a string slice.
pub fn from_utf8(v: &[u8]) -> Result<&str, Utf8Error> {
    match run_utf8_validation(v) {
        Ok(_) => Ok(unsafe { from_utf8_unchecked(v) }),
        Err(err) => Err(err),
    }
}

/// Converts a mutable slice of bytes to a mutable string slice.
pub fn from_utf8_mut(v: &mut [u8]) -> Result<&mut str, Utf8Error> {
    match run_utf8_validation(v) {
        Ok(_) => Ok(unsafe { from_utf8_unchecked_mut(v) }),
        Err(err) => Err(err),
    }
}

/// Converts a slice of bytes to a string slice without checking that it contains valid UTF-8.
#[inline]
pub const unsafe fn from_utf8_unchecked(v: &[u8]) -> &str {
    &*crate::intrinsics::str_from_raw_parts(v.as_ptr(), v.len())
}

/// Converts a slice of bytes to a string slice without checking that it contains valid UTF-8; mutable version.
#[inline]
pub unsafe fn from_utf8_unchecked_mut(v: &mut [u8]) -> &mut str {
    let len = v.len();
    &mut *(crate::intrinsics::str_from_raw_parts(v.as_mut_ptr() as *const u8, len) as *mut str)
}
