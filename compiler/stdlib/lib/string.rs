//! A UTF-8–encoded, growable string.

use crate::borrow::Cow;
use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::iter::{DoubleEndedIterator, Extend, FromIterator, FusedIterator, IntoIterator, Iterator};
use crate::ops::{Add, AddAssign, Deref, DerefMut, Index, IndexMut, RangeBounds};
use crate::slice::SliceIndex;
use crate::str::{self, Chars, Utf8Error};

/// A UTF-8–encoded, growable string.
#[lang = "String"]
pub struct String {
    vec: Vec<u8>,
}

/// A possible error value when converting a `String` from a UTF-8 byte vector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FromUtf8Error {
    bytes: Vec<u8>,
    error: Utf8Error,
}

/// A possible error value when converting a `String` from a UTF-16 byte slice.
#[derive(Debug)]
pub struct FromUtf16Error {
    kind: FromUtf16ErrorKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FromUtf16ErrorKind {
    LoneSurrogate,
    #[allow(dead_code)]
    OddBytes,
}

/// A type alias for `Infallible`.
pub type ParseError = crate::convert::Infallible;

impl String {
    /// Creates a new empty `String`.
    #[inline]
    pub const fn new() -> String {
        String { vec: Vec::new() }
    }

    /// Creates a new empty `String` with at least the specified capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> String {
        String { vec: Vec::with_capacity(capacity) }
    }

    /// Converts a vector of bytes to a `String`.
    #[inline]
    pub fn from_utf8(vec: Vec<u8>) -> Result<String, FromUtf8Error> {
        match str::from_utf8(&vec) {
            Ok(_) => Ok(String { vec }),
            Err(e) => Err(FromUtf8Error { bytes: vec, error: e }),
        }
    }

    /// Converts a slice of bytes to a string, including invalid characters.
    pub fn from_utf8_lossy(v: &[u8]) -> Cow<'_, str> {
        let mut rest = v;
        let mut res: Option<String> = None;
        loop {
            match str::from_utf8(rest) {
                Ok(valid) => {
                    return match res {
                        None => Cow::Borrowed(valid),
                        Some(mut s) => {
                            s.push_str(valid);
                            Cow::Owned(s)
                        }
                    };
                }
                Err(e) => {
                    let mut s = match res.take() {
                        Some(s) => s,
                        None => String::with_capacity(v.len()),
                    };
                    let good = e.valid_up_to();
                    s.push_str(unsafe { str::from_utf8_unchecked(&rest[..good]) });
                    s.push('\u{FFFD}');
                    let skip = match e.error_len() {
                        Some(n) => n,
                        None => rest.len() - good,
                    };
                    rest = &rest[good + skip..];
                    res = Some(s);
                }
            }
        }
    }

    /// Converts a `Vec<u8>` to a `String`, substituting invalid UTF-8 sequences with replacement characters.
    pub fn from_utf8_lossy_owned(v: Vec<u8>) -> String {
        String::from_utf8_lossy(&v).into_owned()
    }

    /// Decode a UTF-16–encoded vector `v` into a `String`.
    pub fn from_utf16(v: &[u16]) -> Result<String, FromUtf16Error> {
        let mut ret = String::with_capacity(v.len());
        let mut it = crate::char::decode_utf16(v.iter().cloned());
        while let Some(c) = it.next() {
            match c {
                Ok(c) => ret.push(c),
                Err(_) => return Err(FromUtf16Error { kind: FromUtf16ErrorKind::LoneSurrogate }),
            }
        }
        Ok(ret)
    }

    /// Decode a UTF-16–encoded slice `v` into a `String`, replacing invalid data with the replacement character.
    pub fn from_utf16_lossy(v: &[u16]) -> String {
        let mut ret = String::with_capacity(v.len());
        let mut it = crate::char::decode_utf16(v.iter().cloned());
        while let Some(c) = it.next() {
            match c {
                Ok(c) => ret.push(c),
                Err(_) => ret.push('\u{FFFD}'),
            }
        }
        ret
    }

    /// Creates a new `String` from a pointer, a length and a capacity.
    #[inline]
    pub unsafe fn from_raw_parts(buf: *mut u8, length: usize, capacity: usize) -> String {
        String { vec: Vec::from_raw_parts(buf, length, capacity) }
    }

    /// Converts a vector of bytes to a `String` without checking that the string contains valid UTF-8.
    #[inline]
    pub unsafe fn from_utf8_unchecked(bytes: Vec<u8>) -> String {
        String { vec: bytes }
    }

    /// Converts a `String` into a byte vector.
    #[inline]
    pub fn into_bytes(self) -> Vec<u8> {
        self.vec
    }

    /// Extracts a string slice containing the entire `String`.
    #[inline]
    pub fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.vec.as_slice()) }
    }

    /// Converts a `String` into a mutable string slice.
    #[inline]
    pub fn as_mut_str(&mut self) -> &mut str {
        unsafe { str::from_utf8_unchecked_mut(self.vec.as_mut_slice()) }
    }

    /// Appends a given string slice onto the end of this `String`.
    #[inline]
    pub fn push_str(&mut self, string: &str) {
        let n = string.len();
        self.vec.reserve(n);
        unsafe {
            let len = self.vec.len();
            crate::intrinsics::copy_nonoverlapping(string.as_ptr(), self.vec.as_mut_ptr().add(len), n);
            self.vec.set_len(len + n);
        }
    }

    /// Copies elements from `src` range to the end of the string.
    #[track_caller]
    pub fn extend_from_within<R: RangeBounds<usize>>(&mut self, src: R) {
        let r = crate::slice::range(src, self.len());
        if !self.is_char_boundary(r.start) {
            crate::rt::panic_str("assertion failed: self.is_char_boundary(start)", file!(), line!(), column!());
        }
        if !self.is_char_boundary(r.end) {
            crate::rt::panic_str("assertion failed: self.is_char_boundary(end)", file!(), line!(), column!());
        }
        self.vec.extend_from_within(r);
    }

    /// Returns this `String`'s capacity, in bytes.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.vec.capacity()
    }

    /// Reserves capacity for at least `additional` bytes more than the current length.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.vec.reserve(additional)
    }

    /// Reserves the minimum capacity for at least `additional` bytes more than the current length.
    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        self.vec.reserve_exact(additional)
    }

    /// Tries to reserve capacity for at least `additional` bytes more than the current length.
    #[inline]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), crate::collections::TryReserveError> {
        self.vec.try_reserve(additional)
    }

    /// Tries to reserve the minimum capacity for at least `additional` bytes more than the current length.
    #[inline]
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), crate::collections::TryReserveError> {
        self.vec.try_reserve_exact(additional)
    }

    /// Shrinks the capacity of this `String` to match its length.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.vec.shrink_to_fit()
    }

    /// Shrinks the capacity of this `String` with a lower bound.
    #[inline]
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.vec.shrink_to(min_capacity)
    }

    /// Appends the given `char` to the end of this `String`.
    #[inline]
    pub fn push(&mut self, ch: char) {
        let code = ch as u32;
        if code < 0x80 {
            self.vec.push(code as u8);
        } else {
            let n = ch.len_utf8();
            self.vec.reserve(n);
            unsafe {
                let len = self.vec.len();
                crate::char::encode_utf8_raw(code, self.vec.as_mut_ptr().add(len));
                self.vec.set_len(len + n);
            }
        }
    }

    /// Returns a byte slice of this `String`'s contents.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.vec.as_slice()
    }

    /// Shortens this `String` to the specified length.
    #[inline]
    #[track_caller]
    pub fn truncate(&mut self, new_len: usize) {
        if new_len <= self.len() {
            if !self.is_char_boundary(new_len) {
                crate::rt::panic_str("assertion failed: self.is_char_boundary(new_len)", file!(), line!(), column!());
            }
            self.vec.truncate(new_len)
        }
    }

    /// Removes the last character from the string buffer and returns it.
    #[inline]
    pub fn pop(&mut self) -> Option<char> {
        let len = self.len();
        if len == 0 {
            return None;
        }
        let (ch, n) = str::decode_before(self.as_str(), len);
        unsafe {
            self.vec.set_len(len - n);
        }
        Some(ch)
    }

    /// Removes a `char` from this `String` at a byte position and returns it.
    #[inline]
    #[track_caller]
    pub fn remove(&mut self, idx: usize) -> char {
        let len = self.len();
        if idx > len || !self.is_char_boundary(idx) {
            str::slice_error_fail(self.as_str(), idx, len);
        }
        if idx == len {
            crate::rt::panic_str("cannot remove a char from the end of a string", file!(), line!(), column!());
        }
        let (ch, n) = str::decode_at(self.as_str(), idx);
        unsafe {
            let p = self.vec.as_mut_ptr();
            crate::intrinsics::copy(p.add(idx + n) as *const u8, p.add(idx), len - idx - n);
            self.vec.set_len(len - n);
        }
        ch
    }

    /// Retains only the characters specified by the predicate.
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(char) -> bool,
    {
        // In place (keeps the capacity, like the real implementation).
        let mut f = f;
        let len = self.len();
        let mut i = 0;
        let mut w = 0;
        while i < len {
            let (c, n) = str::decode_at(self.as_str(), i);
            if f(c) {
                if w != i {
                    let p = self.vec.as_mut_ptr();
                    unsafe { crate::ptr::copy(p.add(i), p.add(w), n) };
                }
                w += n;
            }
            i += n;
        }
        unsafe { self.vec.set_len(w) };
    }

    /// Inserts a character into this `String` at a byte position.
    #[inline]
    #[track_caller]
    pub fn insert(&mut self, idx: usize, ch: char) {
        if !self.is_char_boundary(idx) {
            crate::rt::panic_str("assertion failed: self.is_char_boundary(idx)", file!(), line!(), column!());
        }
        let mut bits = [0u8; 4];
        let s = ch.encode_utf8(&mut bits);
        self.insert_bytes(idx, s.as_bytes());
    }

    fn insert_bytes(&mut self, idx: usize, bytes: &[u8]) {
        let len = self.len();
        let amt = bytes.len();
        self.vec.reserve(amt);
        unsafe {
            let p = self.vec.as_mut_ptr();
            crate::intrinsics::copy(p.add(idx) as *const u8, p.add(idx + amt), len - idx);
            crate::intrinsics::copy_nonoverlapping(bytes.as_ptr(), p.add(idx), amt);
            self.vec.set_len(len + amt);
        }
    }

    /// Inserts a string slice into this `String` at a byte position.
    #[inline]
    #[track_caller]
    pub fn insert_str(&mut self, idx: usize, string: &str) {
        if !self.is_char_boundary(idx) {
            crate::rt::panic_str("assertion failed: self.is_char_boundary(idx)", file!(), line!(), column!());
        }
        self.insert_bytes(idx, string.as_bytes());
    }

    /// Returns a mutable reference to the contents of this `String`.
    #[inline]
    pub unsafe fn as_mut_vec(&mut self) -> &mut Vec<u8> {
        &mut self.vec
    }

    /// Returns the length of this `String`, in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    /// Returns `true` if this `String` has a length of zero.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Splits the string into two at the given byte index.
    #[inline]
    #[track_caller]
    pub fn split_off(&mut self, at: usize) -> String {
        if !self.is_char_boundary(at) {
            crate::rt::panic_str("assertion failed: self.is_char_boundary(at)", file!(), line!(), column!());
        }
        let other = self.vec.split_off(at);
        unsafe { String::from_utf8_unchecked(other) }
    }

    /// Truncates this `String`, removing all contents.
    #[inline]
    pub fn clear(&mut self) {
        self.vec.clear()
    }

    /// Removes the specified range from the string in bulk, returning all removed characters as an iterator.
    #[track_caller]
    pub fn drain<R>(&mut self, range: R) -> Drain<'_>
    where
        R: RangeBounds<usize>,
    {
        let r = crate::slice::range(range, self.len());
        if !self.is_char_boundary(r.start) {
            crate::rt::panic_str("assertion failed: self.is_char_boundary(start)", file!(), line!(), column!());
        }
        if !self.is_char_boundary(r.end) {
            crate::rt::panic_str("assertion failed: self.is_char_boundary(end)", file!(), line!(), column!());
        }
        let start = r.start;
        let end = r.end;
        let self_ptr = self as *mut String;
        let chars_iter = unsafe { (*self_ptr).get_unchecked(start..end) }.chars();
        Drain { string: self_ptr, start, end, iter: chars_iter }
    }

    /// Removes the specified range in the string, and replaces it with the given string.
    #[track_caller]
    pub fn replace_range<R>(&mut self, range: R, replace_with: &str)
    where
        R: RangeBounds<usize>,
    {
        let r = crate::slice::range(range, self.len());
        if !self.is_char_boundary(r.start) {
            crate::rt::panic_str("start of range should be a character boundary", file!(), line!(), column!());
        }
        if !self.is_char_boundary(r.end) {
            crate::rt::panic_str("end of range should be a character boundary", file!(), line!(), column!());
        }
        let mut out = String::with_capacity(self.len() - (r.end - r.start) + replace_with.len());
        out.push_str(unsafe { self.get_unchecked(0..r.start) });
        out.push_str(replace_with);
        out.push_str(unsafe { self.get_unchecked(r.end..self.len()) });
        *self = out;
    }

    /// Converts this `String` into a `Box<str>`.
    pub fn into_boxed_str(self) -> Box<str> {
        let b = self.vec.into_boxed_slice();
        let len = b.len();
        let p = Box::into_raw(b);
        unsafe {
            let sp = crate::intrinsics::str_from_raw_parts(crate::intrinsics::slice_ptr(p as *const [u8]), len) as *mut str;
            Box::from_raw(sp)
        }
    }

    /// Consumes and leaks the `String`, returning a mutable reference to the contents.
    pub fn leak<'a>(self) -> &'a mut str {
        let slice = self.vec.leak();
        unsafe { str::from_utf8_unchecked_mut(slice) }
    }
}

impl FromUtf8Error {
    /// Returns a slice of bytes that were attempted to convert to a `String`.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..]
    }

    /// Returns the bytes that were attempted to convert to a `String`.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Fetch a `Utf8Error` to get more details about the conversion failure.
    pub fn utf8_error(&self) -> Utf8Error {
        self.error
    }
}

impl fmt::Display for FromUtf8Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}

impl fmt::Display for FromUtf16Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(match self.kind {
            FromUtf16ErrorKind::LoneSurrogate => "invalid utf-16: lone surrogate found",
            FromUtf16ErrorKind::OddBytes => "invalid utf-16: odd number of bytes",
        }, f)
    }
}

impl crate::error::Error for FromUtf8Error {}
impl crate::error::Error for FromUtf16Error {}

impl Clone for String {
    fn clone(&self) -> Self {
        String { vec: self.vec.clone() }
    }

    fn clone_from(&mut self, source: &Self) {
        self.vec.clone_from(&source.vec);
    }
}

impl PartialEq for String {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        PartialEq::eq(self.as_str(), other.as_str())
    }
}

impl Eq for String {}

impl PartialOrd for String {
    #[inline]
    fn partial_cmp(&self, other: &String) -> Option<Ordering> {
        Some(Ord::cmp(self.as_str(), other.as_str()))
    }
}

impl Ord for String {
    #[inline]
    fn cmp(&self, other: &String) -> Ordering {
        Ord::cmp(self.as_str(), other.as_str())
    }
}

impl Default for String {
    #[inline]
    fn default() -> String {
        String::new()
    }
}

impl fmt::Display for String {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl fmt::Debug for String {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl Hash for String {
    #[inline]
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        Hash::hash(self.as_str(), hasher)
    }
}

impl Deref for String {
    type Target = str;
    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl DerefMut for String {
    #[inline]
    fn deref_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<I: SliceIndex<str>> Index<I> for String {
    type Output = I::Output;
    #[inline]
    #[track_caller]
    fn index(&self, index: I) -> &I::Output {
        index.index(self.as_str())
    }
}

impl<I: SliceIndex<str>> IndexMut<I> for String {
    #[inline]
    #[track_caller]
    fn index_mut(&mut self, index: I) -> &mut I::Output {
        index.index_mut(self.as_mut_str())
    }
}

impl Add<&str> for String {
    type Output = String;
    #[inline]
    fn add(mut self, other: &str) -> String {
        self.push_str(other);
        self
    }
}

impl AddAssign<&str> for String {
    #[inline]
    fn add_assign(&mut self, other: &str) {
        self.push_str(other);
    }
}

impl AsRef<str> for String {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsMut<str> for String {
    #[inline]
    fn as_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl AsRef<[u8]> for String {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl fmt::Write for String {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
    }

    #[inline]
    fn write_char(&mut self, c: char) -> fmt::Result {
        self.push(c);
        Ok(())
    }
}

impl crate::str::FromStr for String {
    type Err = crate::convert::Infallible;
    #[inline]
    fn from_str(s: &str) -> Result<String, Self::Err> {
        Ok(String::from(s))
    }
}

// ---------------------------------------------------------------------------
// Comparisons with str / &str / Cow<str>

macro_rules! __std_string_eq {
    ($lhs:ty, $rhs:ty) => {
        impl<'a, 'b> PartialEq<$rhs> for $lhs {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                PartialEq::eq(&self[..], &other[..])
            }
            #[inline]
            fn ne(&self, other: &$rhs) -> bool {
                PartialEq::ne(&self[..], &other[..])
            }
        }

        impl<'a, 'b> PartialEq<$lhs> for $rhs {
            #[inline]
            fn eq(&self, other: &$lhs) -> bool {
                PartialEq::eq(&self[..], &other[..])
            }
            #[inline]
            fn ne(&self, other: &$lhs) -> bool {
                PartialEq::ne(&self[..], &other[..])
            }
        }
    };
}

__std_string_eq! { String, str }
__std_string_eq! { String, &'a str }
__std_string_eq! { Cow<'a, str>, str }
__std_string_eq! { Cow<'a, str>, &'b str }
__std_string_eq! { Cow<'a, str>, String }

// ---------------------------------------------------------------------------
// Conversions

impl From<&str> for String {
    #[inline]
    fn from(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        out.push_str(s);
        out
    }
}

impl From<&mut str> for String {
    #[inline]
    fn from(s: &mut str) -> String {
        String::from(&*s)
    }
}

impl From<&String> for String {
    #[inline]
    fn from(s: &String) -> String {
        s.clone()
    }
}

impl From<char> for String {
    #[inline]
    fn from(c: char) -> String {
        let mut s = String::new();
        s.push(c);
        s
    }
}

impl From<Box<str>> for String {
    fn from(s: Box<str>) -> String {
        let len = s.len();
        let p = Box::into_raw(s);
        unsafe {
            let bytes = crate::intrinsics::str_ptr(p as *const str) as *mut u8;
            String { vec: Vec::from_raw_parts(bytes, len, len) }
        }
    }
}

impl<'a> From<Cow<'a, str>> for String {
    fn from(s: Cow<'a, str>) -> String {
        s.into_owned()
    }
}

impl<'a> From<&'a str> for Cow<'a, str> {
    #[inline]
    fn from(s: &'a str) -> Cow<'a, str> {
        Cow::Borrowed(s)
    }
}

impl<'a> From<String> for Cow<'a, str> {
    #[inline]
    fn from(s: String) -> Cow<'a, str> {
        Cow::Owned(s)
    }
}

impl<'a> From<&'a String> for Cow<'a, str> {
    #[inline]
    fn from(s: &'a String) -> Cow<'a, str> {
        Cow::Borrowed(s.as_str())
    }
}

impl From<String> for Vec<u8> {
    fn from(string: String) -> Vec<u8> {
        string.into_bytes()
    }
}


// ---------------------------------------------------------------------------
// Iteration

impl FromIterator<char> for String {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> String {
        let mut buf = String::new();
        buf.extend(iter);
        buf
    }
}

impl<'a> FromIterator<&'a char> for String {
    fn from_iter<I: IntoIterator<Item = &'a char>>(iter: I) -> String {
        let mut buf = String::new();
        buf.extend(iter);
        buf
    }
}

impl<'a> FromIterator<&'a str> for String {
    fn from_iter<I: IntoIterator<Item = &'a str>>(iter: I) -> String {
        let mut buf = String::new();
        buf.extend(iter);
        buf
    }
}

impl FromIterator<String> for String {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> String {
        let mut buf = String::new();
        buf.extend(iter);
        buf
    }
}

impl<'a> FromIterator<&'a String> for String {
    fn from_iter<I: IntoIterator<Item = &'a String>>(iter: I) -> String {
        let mut buf = String::new();
        let mut it = iter.into_iter();
        while let Some(s) = it.next() {
            buf.push_str(s.as_str());
        }
        buf
    }
}

impl FromIterator<Box<str>> for String {
    fn from_iter<I: IntoIterator<Item = Box<str>>>(iter: I) -> String {
        let mut buf = String::new();
        let mut it = iter.into_iter();
        while let Some(s) = it.next() {
            buf.push_str(&*s);
        }
        buf
    }
}

impl<'a> FromIterator<Cow<'a, str>> for String {
    fn from_iter<I: IntoIterator<Item = Cow<'a, str>>>(iter: I) -> String {
        let mut buf = String::new();
        let mut it = iter.into_iter();
        while let Some(s) = it.next() {
            buf.push_str(&*s);
        }
        buf
    }
}

impl<'a> FromIterator<char> for Cow<'a, str> {
    fn from_iter<I: IntoIterator<Item = char>>(it: I) -> Cow<'a, str> {
        Cow::Owned(FromIterator::from_iter(it))
    }
}

impl Extend<char> for String {
    fn extend<I: IntoIterator<Item = char>>(&mut self, iter: I) {
        let mut it = iter.into_iter();
        let (lower, _) = it.size_hint();
        self.reserve(lower);
        while let Some(c) = it.next() {
            self.push(c);
        }
    }
    #[inline]
    fn extend_one(&mut self, c: char) {
        self.push(c);
    }
}

impl<'a> Extend<&'a char> for String {
    fn extend<I: IntoIterator<Item = &'a char>>(&mut self, iter: I) {
        let mut it = iter.into_iter();
        while let Some(c) = it.next() {
            self.push(*c);
        }
    }
    #[inline]
    fn extend_one(&mut self, c: &'a char) {
        self.push(*c);
    }
}

impl<'a> Extend<&'a str> for String {
    fn extend<I: IntoIterator<Item = &'a str>>(&mut self, iter: I) {
        let mut it = iter.into_iter();
        while let Some(s) = it.next() {
            self.push_str(s);
        }
    }
    #[inline]
    fn extend_one(&mut self, s: &'a str) {
        self.push_str(s);
    }
}

impl Extend<String> for String {
    fn extend<I: IntoIterator<Item = String>>(&mut self, iter: I) {
        let mut it = iter.into_iter();
        while let Some(s) = it.next() {
            self.push_str(s.as_str());
        }
    }
    #[inline]
    fn extend_one(&mut self, s: String) {
        self.push_str(s.as_str());
    }
}

impl<'a> Extend<&'a String> for String {
    fn extend<I: IntoIterator<Item = &'a String>>(&mut self, iter: I) {
        let mut it = iter.into_iter();
        while let Some(s) = it.next() {
            self.push_str(s.as_str());
        }
    }
}

impl Extend<Box<str>> for String {
    fn extend<I: IntoIterator<Item = Box<str>>>(&mut self, iter: I) {
        let mut it = iter.into_iter();
        while let Some(s) = it.next() {
            self.push_str(&*s);
        }
    }
}

impl<'a> Extend<Cow<'a, str>> for String {
    fn extend<I: IntoIterator<Item = Cow<'a, str>>>(&mut self, iter: I) {
        let mut it = iter.into_iter();
        while let Some(s) = it.next() {
            self.push_str(&*s);
        }
    }
}

/// A draining iterator for `String`.
pub struct Drain<'a> {
    string: *mut String,
    start: usize,
    end: usize,
    iter: Chars<'a>,
}

impl<'a> Drain<'a> {
    /// Returns the remaining (sub)string of this iterator as a slice.
    pub fn as_str(&self) -> &str {
        self.iter.as_str()
    }
}

impl<'a> Drop for Drain<'a> {
    fn drop(&mut self) {
        unsafe {
            let s = &mut *self.string;
            if self.start <= self.end && self.end <= s.len() {
                let _ = s.vec.drain(self.start..self.end);
            }
        }
    }
}

impl<'a> Iterator for Drain<'a> {
    type Item = char;
    #[inline]
    fn next(&mut self) -> Option<char> {
        self.iter.next()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
    #[inline]
    fn last(self) -> Option<char> {
        let mut s = self;
        s.next_back()
    }
}

impl<'a> DoubleEndedIterator for Drain<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<char> {
        self.iter.next_back()
    }
}

impl<'a> FusedIterator for Drain<'a> {}

impl<'a> fmt::Debug for Drain<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain").field(&self.as_str()).finish()
    }
}

// ---------------------------------------------------------------------------
// ToString

/// A trait for converting a value to a `String`.
pub trait ToString {
    /// Converts the given value to a `String`.
    fn to_string(&self) -> String;
}

impl<T: fmt::Display + ?Sized> ToString for T {
    #[inline]
    fn to_string(&self) -> String {
        let mut f = fmt::Formatter::new();
        let _ = fmt::Display::fmt(self, &mut f);
        f.into_string()
    }
}
