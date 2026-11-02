//! Operations on ASCII strings and characters.

use crate::fmt;
use crate::iter::FusedIterator;

/// An iterator over the escaped version of a byte.
#[derive(Clone)]
pub struct EscapeDefault {
    data: [u8; 4],
    start: u8,
    end: u8,
}

/// Returns an iterator that produces an escaped version of a `u8`.
pub fn escape_default(c: u8) -> EscapeDefault {
    let hex = b"0123456789abcdef";
    let (data, len): ([u8; 4], u8) = match c {
        b'\t' => ([b'\\', b't', 0, 0], 2),
        b'\r' => ([b'\\', b'r', 0, 0], 2),
        b'\n' => ([b'\\', b'n', 0, 0], 2),
        b'\\' => ([b'\\', b'\\', 0, 0], 2),
        b'\'' => ([b'\\', b'\'', 0, 0], 2),
        b'"' => ([b'\\', b'"', 0, 0], 2),
        0x20..=0x7e => ([c, 0, 0, 0], 1),
        _ => ([b'\\', b'x', hex[(c >> 4) as usize], hex[(c & 0xf) as usize]], 4),
    };
    EscapeDefault { data, start: 0, end: len }
}

impl EscapeDefault {
    /// Returns the escaped bytes as a string slice.
    pub fn as_str(&self) -> &str {
        unsafe { crate::str::from_utf8_unchecked(&self.data[self.start as usize..self.end as usize]) }
    }
}

impl Iterator for EscapeDefault {
    type Item = u8;
    #[inline]
    fn next(&mut self) -> Option<u8> {
        if self.start < self.end {
            let b = self.data[self.start as usize];
            self.start += 1;
            Some(b)
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.end - self.start) as usize;
        (n, Some(n))
    }
}

impl DoubleEndedIterator for EscapeDefault {
    fn next_back(&mut self) -> Option<u8> {
        if self.start < self.end {
            self.end -= 1;
            Some(self.data[self.end as usize])
        } else {
            None
        }
    }
}

impl ExactSizeIterator for EscapeDefault {}
impl FusedIterator for EscapeDefault {}

impl fmt::Display for EscapeDefault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for EscapeDefault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EscapeDefault").finish_non_exhaustive()
    }
}

/// An iterator over the escaped version of a byte slice.
#[derive(Clone)]
pub struct EscapeAscii<'a> {
    bytes: &'a [u8],
    cur: Option<EscapeDefault>,
    back: Option<EscapeDefault>,
}

pub(crate) fn escape_ascii(bytes: &[u8]) -> EscapeAscii<'_> {
    EscapeAscii { bytes, cur: None, back: None }
}

impl<'a> Iterator for EscapeAscii<'a> {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        loop {
            if let Some(ref mut e) = self.cur {
                if let Some(b) = e.next() {
                    return Some(b);
                }
            }
            match self.bytes.split_first() {
                Some((&b, rest)) => {
                    self.bytes = rest;
                    self.cur = Some(escape_default(b));
                }
                None => {
                    return match self.back {
                        Some(ref mut e) => e.next(),
                        None => None,
                    };
                }
            }
        }
    }
}

impl<'a> DoubleEndedIterator for EscapeAscii<'a> {
    fn next_back(&mut self) -> Option<u8> {
        loop {
            if let Some(ref mut e) = self.back {
                if let Some(b) = e.next_back() {
                    return Some(b);
                }
            }
            match self.bytes.split_last() {
                Some((&b, rest)) => {
                    self.bytes = rest;
                    self.back = Some(escape_default(b));
                }
                None => {
                    return match self.cur {
                        Some(ref mut e) => e.next_back(),
                        None => None,
                    };
                }
            }
        }
    }
}

impl<'a> FusedIterator for EscapeAscii<'a> {}

impl<'a> fmt::Display for EscapeAscii<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref e) = self.cur {
            f.write_str(e.as_str())?;
        }
        for &b in self.bytes {
            f.write_str(escape_default(b).as_str())?;
        }
        if let Some(ref e) = self.back {
            f.write_str(e.as_str())?;
        }
        Ok(())
    }
}

impl<'a> fmt::Debug for EscapeAscii<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EscapeAscii").finish_non_exhaustive()
    }
}
