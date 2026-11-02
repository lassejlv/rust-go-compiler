//! The `OsStr` and `OsString` types (arbitrary bytes on Unix).

use crate::borrow::{Borrow, Cow, ToOwned};
use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::ops::{Deref, DerefMut};
use crate::str::FromStr;

/// Borrowed reference to an OS string.
pub struct OsStr {
    _z: [u8; 0],
    inner: [u8],
}

/// A type that can represent owned, mutable platform-native strings.
#[derive(Clone, Default)]
pub struct OsString {
    inner: Vec<u8>,
}

impl OsStr {
    /// Coerces into an `OsStr` slice.
    #[inline]
    pub fn new<S: AsRef<OsStr> + ?Sized>(s: &S) -> &OsStr {
        s.as_ref()
    }

    #[inline]
    pub(crate) fn from_inner(s: &[u8]) -> &OsStr {
        unsafe { &*(s as *const [u8] as *const OsStr) }
    }

    #[inline]
    pub(crate) fn from_inner_mut(s: &mut [u8]) -> &mut OsStr {
        unsafe { &mut *(s as *mut [u8] as *mut OsStr) }
    }

    #[inline]
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.inner
    }

    /// Converts a slice of bytes to an OS string slice without checking.
    #[inline]
    pub unsafe fn from_encoded_bytes_unchecked(bytes: &[u8]) -> &OsStr {
        OsStr::from_inner(bytes)
    }

    /// Yields a `&str` slice if the `OsStr` is valid Unicode.
    #[inline]
    pub fn to_str(&self) -> Option<&str> {
        crate::str::from_utf8(&self.inner).ok()
    }

    /// Converts an `OsStr` to a `Cow<str>` (invalid sequences are replaced by U+FFFD).
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.inner)
    }

    /// Copies the slice into an owned `OsString`.
    pub fn to_os_string(&self) -> OsString {
        OsString { inner: self.inner.to_vec() }
    }

    /// Checks whether the `OsStr` is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the length of this `OsStr` (in bytes).
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Converts a `Box<OsStr>` into an `OsString` without copying or allocating.
    pub fn into_os_string(self: Box<OsStr>) -> OsString {
        self.to_os_string()
    }

    /// Converts an OS string slice to a byte slice.
    #[inline]
    pub fn as_encoded_bytes(&self) -> &[u8] {
        &self.inner
    }

    /// Converts this string to its ASCII lower case equivalent in-place.
    pub fn make_ascii_lowercase(&mut self) {
        self.inner.make_ascii_lowercase()
    }

    /// Converts this string to its ASCII upper case equivalent in-place.
    pub fn make_ascii_uppercase(&mut self) {
        self.inner.make_ascii_uppercase()
    }

    /// Returns a copy of this string where each character is mapped to its ASCII lower case equivalent.
    pub fn to_ascii_lowercase(&self) -> OsString {
        OsString { inner: self.inner.to_ascii_lowercase() }
    }

    /// Returns a copy of this string where each character is mapped to its ASCII upper case equivalent.
    pub fn to_ascii_uppercase(&self) -> OsString {
        OsString { inner: self.inner.to_ascii_uppercase() }
    }

    /// Checks if all characters in this string are within the ASCII range.
    pub fn is_ascii(&self) -> bool {
        self.inner.is_ascii()
    }

    /// Checks that two strings are an ASCII case-insensitive match.
    pub fn eq_ignore_ascii_case<S: AsRef<OsStr>>(&self, other: S) -> bool {
        self.inner.eq_ignore_ascii_case(&other.as_ref().inner)
    }

    /// Returns an object that implements `Display` for safely printing an
    /// `OsStr` that may contain non-Unicode data.
    pub fn display(&self) -> Display<'_> {
        Display { os_str: self }
    }
}

/// Writes `bytes` like the real library's `Utf8Chunks` debug output.
pub(crate) fn debug_os_bytes(bytes: &[u8], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    use crate::fmt::Write;
    f.write_char('"')?;
    let mut rest = bytes;
    loop {
        let (valid, invalid, next): (&str, &[u8], &[u8]) = match crate::str::from_utf8(rest) {
            Ok(s) => (s, &[], &[]),
            Err(e) => {
                let up = e.valid_up_to();
                let bad = match e.error_len() {
                    Some(n) => n,
                    None => rest.len() - up,
                };
                (
                    unsafe { crate::str::from_utf8_unchecked(&rest[..up]) },
                    &rest[up..up + bad],
                    &rest[up + bad..],
                )
            }
        };
        let mut from = 0;
        for (i, c) in valid.char_indices() {
            let esc = c.escape_debug();
            if esc.len() != 1 && c != '\'' {
                f.write_str(&valid[from..i])?;
                for e in esc {
                    f.write_char(e)?;
                }
                from = i + c.len_utf8();
            }
        }
        f.write_str(&valid[from..])?;
        for &b in invalid {
            write!(f, "\\x{:02X}", b)?;
        }
        if next.is_empty() {
            break;
        }
        rest = next;
    }
    f.write_char('"')
}

/// Writes `bytes` lossily (invalid sequences as U+FFFD), honoring padding.
pub(crate) fn display_os_bytes(bytes: &[u8], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match crate::str::from_utf8(bytes) {
        Ok(s) => fmt::Display::fmt(s, f),
        Err(_) => {
            let s = String::from_utf8_lossy(bytes);
            fmt::Display::fmt(&*s, f)
        }
    }
}

/// Helper struct for safely printing an `OsStr` with `format!` and `{}`.
pub struct Display<'a> {
    os_str: &'a OsStr,
}

impl<'a> fmt::Debug for Display<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.os_str, f)
    }
}

impl<'a> fmt::Display for Display<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        display_os_bytes(&self.os_str.inner, f)
    }
}

impl OsString {
    /// Constructs a new empty `OsString`.
    #[inline]
    pub const fn new() -> OsString {
        OsString { inner: Vec::new() }
    }

    #[inline]
    pub(crate) fn from_vec(v: Vec<u8>) -> OsString {
        OsString { inner: v }
    }

    #[inline]
    pub(crate) fn as_vec(&self) -> &Vec<u8> {
        &self.inner
    }

    #[inline]
    pub(crate) fn as_mut_vec(&mut self) -> &mut Vec<u8> {
        &mut self.inner
    }

    /// Converts bytes to an `OsString` without checking that the bytes contain valid `OsStr`-encoded data.
    #[inline]
    pub unsafe fn from_encoded_bytes_unchecked(bytes: Vec<u8>) -> OsString {
        OsString { inner: bytes }
    }

    /// Creates a new `OsString` with at least the given capacity.
    pub fn with_capacity(capacity: usize) -> OsString {
        OsString { inner: Vec::with_capacity(capacity) }
    }

    /// Converts to an `OsStr` slice.
    #[inline]
    pub fn as_os_str(&self) -> &OsStr {
        OsStr::from_inner(&self.inner)
    }

    /// Converts to a mutable `OsStr` slice.
    #[inline]
    pub fn as_mut_os_str(&mut self) -> &mut OsStr {
        OsStr::from_inner_mut(&mut self.inner)
    }

    /// Converts the `OsString` into a `String` if it contains valid Unicode data.
    pub fn into_string(self) -> Result<String, OsString> {
        match String::from_utf8(self.inner) {
            Ok(s) => Ok(s),
            Err(e) => Err(OsString { inner: e.into_bytes() }),
        }
    }

    /// Extends the string with the given `&OsStr` slice.
    pub fn push<T: AsRef<OsStr>>(&mut self, s: T) {
        self.inner.extend_from_slice(&s.as_ref().inner)
    }

    /// Truncates the `OsString` to zero length.
    pub fn clear(&mut self) {
        self.inner.clear()
    }

    /// Returns the capacity this `OsString` can hold without reallocating.
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Reserves capacity for at least `additional` more bytes.
    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional)
    }

    /// Reserves the minimum capacity for at least `additional` more bytes.
    pub fn reserve_exact(&mut self, additional: usize) {
        self.inner.reserve_exact(additional)
    }

    /// Shrinks the capacity of the `OsString` to match its length.
    pub fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit()
    }

    /// Converts this `OsString` into a boxed `OsStr`.
    pub fn into_boxed_os_str(self) -> Box<OsStr> {
        let b: Box<[u8]> = self.inner.into_boxed_slice();
        unsafe { Box::from_raw(Box::into_raw(b) as *mut OsStr) }
    }

    /// Converts the `OsString` into a byte vector.
    pub fn into_encoded_bytes(self) -> Vec<u8> {
        self.inner
    }

    /// Truncates the `OsString` to the given length (in bytes).
    pub fn truncate(&mut self, len: usize) {
        self.inner.truncate(len)
    }
}

impl Deref for OsString {
    type Target = OsStr;
    #[inline]
    fn deref(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl DerefMut for OsString {
    #[inline]
    fn deref_mut(&mut self) -> &mut OsStr {
        self.as_mut_os_str()
    }
}

impl crate::ops::Index<crate::ops::RangeFull> for OsString {
    type Output = OsStr;
    #[inline]
    fn index(&self, _index: crate::ops::RangeFull) -> &OsStr {
        self.as_os_str()
    }
}

impl AsRef<OsStr> for OsStr {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        self
    }
}

impl AsRef<OsStr> for OsString {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl AsRef<OsStr> for str {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        OsStr::from_inner(self.as_bytes())
    }
}

impl AsRef<OsStr> for String {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        OsStr::from_inner(self.as_bytes())
    }
}

impl Borrow<OsStr> for OsString {
    #[inline]
    fn borrow(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl ToOwned for OsStr {
    type Owned = OsString;
    fn to_owned(&self) -> OsString {
        self.to_os_string()
    }
}

impl Clone for Box<OsStr> {
    fn clone(&self) -> Self {
        self.to_os_string().into_boxed_os_str()
    }
}

impl From<String> for OsString {
    #[inline]
    fn from(s: String) -> OsString {
        OsString { inner: s.into_bytes() }
    }
}

impl<T: ?Sized + AsRef<OsStr>> From<&T> for OsString {
    fn from(s: &T) -> OsString {
        s.as_ref().to_os_string()
    }
}

impl From<Box<OsStr>> for OsString {
    fn from(boxed: Box<OsStr>) -> OsString {
        boxed.to_os_string()
    }
}

impl From<OsString> for Box<OsStr> {
    fn from(s: OsString) -> Box<OsStr> {
        s.into_boxed_os_str()
    }
}

impl From<&OsStr> for Box<OsStr> {
    fn from(s: &OsStr) -> Box<OsStr> {
        s.to_os_string().into_boxed_os_str()
    }
}

impl<'a> From<Cow<'a, OsStr>> for OsString {
    fn from(s: Cow<'a, OsStr>) -> OsString {
        s.into_owned()
    }
}

impl<'a> From<OsString> for Cow<'a, OsStr> {
    fn from(s: OsString) -> Cow<'a, OsStr> {
        Cow::Owned(s)
    }
}

impl<'a> From<&'a OsStr> for Cow<'a, OsStr> {
    fn from(s: &'a OsStr) -> Cow<'a, OsStr> {
        Cow::Borrowed(s)
    }
}

impl<'a> From<&'a OsString> for Cow<'a, OsStr> {
    fn from(s: &'a OsString) -> Cow<'a, OsStr> {
        Cow::Borrowed(s.as_os_str())
    }
}

impl FromStr for OsString {
    type Err = crate::convert::Infallible;
    fn from_str(s: &str) -> Result<OsString, Self::Err> {
        Ok(OsString::from(s))
    }
}

impl Default for &OsStr {
    fn default() -> Self {
        OsStr::from_inner(&[])
    }
}

impl fmt::Write for OsString {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.inner.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

impl<T: AsRef<OsStr>> Extend<T> for OsString {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for s in iter {
            self.push(s);
        }
    }
}

impl<T: AsRef<OsStr>> crate::iter::FromIterator<T> for OsString {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> OsString {
        let mut buf = OsString::new();
        for s in iter {
            buf.push(s);
        }
        buf
    }
}

// ---- comparisons

impl PartialEq for OsStr {
    #[inline]
    fn eq(&self, other: &OsStr) -> bool {
        self.inner == other.inner
    }
}

impl Eq for OsStr {}

impl PartialEq<str> for OsStr {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        &self.inner == other.as_bytes()
    }
}

impl PartialEq<OsStr> for str {
    #[inline]
    fn eq(&self, other: &OsStr) -> bool {
        self.as_bytes() == &other.inner
    }
}

impl PartialEq for OsString {
    #[inline]
    fn eq(&self, other: &OsString) -> bool {
        self.inner == other.inner
    }
}

impl Eq for OsString {}

impl PartialEq<str> for OsString {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        &self.inner[..] == other.as_bytes()
    }
}

impl PartialEq<OsString> for str {
    #[inline]
    fn eq(&self, other: &OsString) -> bool {
        self.as_bytes() == &other.inner[..]
    }
}

impl<'a> PartialEq<&'a str> for OsString {
    #[inline]
    fn eq(&self, other: &&'a str) -> bool {
        &self.inner[..] == other.as_bytes()
    }
}

impl<'a> PartialEq<OsString> for &'a str {
    #[inline]
    fn eq(&self, other: &OsString) -> bool {
        self.as_bytes() == &other.inner[..]
    }
}

impl PartialEq<OsStr> for OsString {
    #[inline]
    fn eq(&self, other: &OsStr) -> bool {
        &self.inner[..] == &other.inner
    }
}

impl PartialEq<OsString> for OsStr {
    #[inline]
    fn eq(&self, other: &OsString) -> bool {
        &self.inner == &other.inner[..]
    }
}

impl<'a> PartialEq<&'a OsStr> for OsString {
    #[inline]
    fn eq(&self, other: &&'a OsStr) -> bool {
        &self.inner[..] == &other.inner
    }
}

impl<'a> PartialEq<OsString> for &'a OsStr {
    #[inline]
    fn eq(&self, other: &OsString) -> bool {
        &self.inner == &other.inner[..]
    }
}

impl PartialOrd for OsStr {
    #[inline]
    fn partial_cmp(&self, other: &OsStr) -> Option<Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl Ord for OsStr {
    #[inline]
    fn cmp(&self, other: &OsStr) -> Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl PartialOrd<str> for OsStr {
    #[inline]
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        self.inner.partial_cmp(other.as_bytes())
    }
}

impl PartialOrd for OsString {
    #[inline]
    fn partial_cmp(&self, other: &OsString) -> Option<Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl Ord for OsString {
    #[inline]
    fn cmp(&self, other: &OsString) -> Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl PartialOrd<str> for OsString {
    #[inline]
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        self.inner[..].partial_cmp(other.as_bytes())
    }
}

impl Hash for OsStr {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state)
    }
}

impl Hash for OsString {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner[..].hash(state)
    }
}

impl fmt::Debug for OsStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_os_bytes(&self.inner, f)
    }
}

impl fmt::Debug for OsString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_os_bytes(&self.inner, f)
    }
}
