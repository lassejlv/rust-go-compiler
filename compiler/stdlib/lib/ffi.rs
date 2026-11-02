//! Platform-specific types, as defined by C, and owned/borrowed C and OS strings.

use crate::borrow::{Borrow, Cow, ToOwned};
use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::intrinsics;
use crate::ops::Deref;
use crate::str::Utf8Error;

// ---------------------------------------------------------------------------
// C types (x86-64 Linux)

/// Equivalent to C's `char` type.
pub type c_char = i8;
/// Equivalent to C's `signed char` type.
pub type c_schar = i8;
/// Equivalent to C's `unsigned char` type.
pub type c_uchar = u8;
/// Equivalent to C's `signed short` type.
pub type c_short = i16;
/// Equivalent to C's `unsigned short` type.
pub type c_ushort = u16;
/// Equivalent to C's `signed int` type.
pub type c_int = i32;
/// Equivalent to C's `unsigned int` type.
pub type c_uint = u32;
/// Equivalent to C's `signed long` type.
pub type c_long = i64;
/// Equivalent to C's `unsigned long` type.
pub type c_ulong = u64;
/// Equivalent to C's `signed long long` type.
pub type c_longlong = i64;
/// Equivalent to C's `unsigned long long` type.
pub type c_ulonglong = u64;
/// Equivalent to C's `float` type.
pub type c_float = f32;
/// Equivalent to C's `double` type.
pub type c_double = f64;
/// Equivalent to C's `size_t` type.
pub type c_size_t = usize;
/// Equivalent to C's `ssize_t` type.
pub type c_ssize_t = isize;
/// Equivalent to C's `ptrdiff_t` type.
pub type c_ptrdiff_t = isize;

/// Equivalent to C's `void` type when used as a pointer.
#[repr(u8)]
pub enum c_void {
    #[doc(hidden)]
    __variant1,
    #[doc(hidden)]
    __variant2,
}

impl fmt::Debug for c_void {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("c_void").finish()
    }
}

// Writes the bytes like `<[u8]>::escape_ascii` inside double quotes.
fn debug_bytes(bytes: &[u8], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("\"")?;
    for &b in bytes {
        match b {
            b'\t' => f.write_str("\\t")?,
            b'\r' => f.write_str("\\r")?,
            b'\n' => f.write_str("\\n")?,
            b'\\' => f.write_str("\\\\")?,
            b'\'' => f.write_str("\\'")?,
            b'"' => f.write_str("\\\"")?,
            0x20..=0x7e => f.write_char(b as char)?,
            _ => {
                let hex = b"0123456789abcdef";
                f.write_str("\\x")?;
                f.write_char(hex[(b >> 4) as usize] as char)?;
                f.write_char(hex[(b & 15) as usize] as char)?;
            }
        }
    }
    f.write_str("\"")
}

// ---------------------------------------------------------------------------
// CStr

/// Representation of a borrowed C string (the bytes include the trailing nul).
pub struct CStr {
    // (a zero-sized prefix: the C backend cannot express a struct holding only a slice)
    _z: [u8; 0],
    inner: [u8],
}

/// An error indicating that a nul byte was not in the expected position.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FromBytesWithNulError {
    /// Data provided contains an interior nul byte at byte `position`.
    InteriorNul {
        /// The position of the interior nul byte.
        position: usize,
    },
    /// Data provided is not nul terminated.
    NotNulTerminated,
}

impl fmt::Display for FromBytesWithNulError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            FromBytesWithNulError::InteriorNul { position } => {
                write!(f, "data provided contains an interior nul byte at byte pos {}", position)
            }
            FromBytesWithNulError::NotNulTerminated => f.write_str("data provided is not nul terminated"),
        }
    }
}

impl crate::error::Error for FromBytesWithNulError {}

/// An error indicating that no nul byte was present.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FromBytesUntilNulError(());

impl fmt::Display for FromBytesUntilNulError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("data provided does not contain a nul")
    }
}

impl crate::error::Error for FromBytesUntilNulError {}

impl CStr {
    /// Wraps a raw C string with a safe C string wrapper.
    pub unsafe fn from_ptr<'a>(ptr: *const c_char) -> &'a CStr {
        let len = intrinsics::cstr_len(ptr as *const u8);
        let bytes = crate::slice::from_raw_parts(ptr as *const u8, len + 1);
        CStr::from_bytes_with_nul_unchecked(bytes)
    }

    /// Creates a C string wrapper from a byte slice with any number of nuls (up to the first one).
    pub fn from_bytes_until_nul(bytes: &[u8]) -> Result<&CStr, FromBytesUntilNulError> {
        match bytes.iter().position(|&b| b == 0) {
            Some(i) => Ok(unsafe { CStr::from_bytes_with_nul_unchecked(&bytes[..i + 1]) }),
            None => Err(FromBytesUntilNulError(())),
        }
    }

    /// Creates a C string wrapper from a byte slice with exactly one nul terminator.
    pub fn from_bytes_with_nul(bytes: &[u8]) -> Result<&CStr, FromBytesWithNulError> {
        match bytes.iter().position(|&b| b == 0) {
            Some(i) if i + 1 == bytes.len() => Ok(unsafe { CStr::from_bytes_with_nul_unchecked(bytes) }),
            Some(i) => Err(FromBytesWithNulError::InteriorNul { position: i }),
            None => Err(FromBytesWithNulError::NotNulTerminated),
        }
    }

    /// Unsafely creates a C string wrapper from a byte slice (which must end with the only nul).
    #[inline]
    pub const unsafe fn from_bytes_with_nul_unchecked(bytes: &[u8]) -> &CStr {
        &*(bytes as *const [u8] as *const CStr)
    }

    /// Returns the inner pointer to this C string.
    #[inline]
    pub const fn as_ptr(&self) -> *const c_char {
        self.inner.as_ptr() as *const c_char
    }

    /// Returns the length of `self` (without the nul terminator).
    #[inline]
    pub fn count_bytes(&self) -> usize {
        self.inner.len() - 1
    }

    /// Returns `true` if `self.to_bytes()` has a length of 0.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.len() <= 1
    }

    /// Converts this C string to a byte slice (without the trailing nul).
    #[inline]
    pub fn to_bytes(&self) -> &[u8] {
        &self.inner[..self.inner.len() - 1]
    }

    /// Converts this C string to a byte slice containing the trailing 0 byte.
    #[inline]
    pub fn to_bytes_with_nul(&self) -> &[u8] {
        &self.inner
    }

    /// Yields a `&str` slice if the `CStr` contains valid UTF-8.
    pub fn to_str(&self) -> Result<&str, Utf8Error> {
        crate::str::from_utf8(self.to_bytes())
    }

    /// Converts a `CStr` into a `Cow<str>` (invalid UTF-8 sequences are replaced).
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.to_bytes())
    }
}

impl PartialEq for CStr {
    fn eq(&self, other: &CStr) -> bool {
        self.to_bytes() == other.to_bytes()
    }
}

impl Eq for CStr {}

impl PartialOrd for CStr {
    fn partial_cmp(&self, other: &CStr) -> Option<Ordering> {
        self.to_bytes().partial_cmp(other.to_bytes())
    }
}

impl Ord for CStr {
    fn cmp(&self, other: &CStr) -> Ordering {
        self.to_bytes().cmp(other.to_bytes())
    }
}

impl Hash for CStr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.to_bytes_with_nul().hash(state);
    }
}

impl fmt::Debug for CStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_bytes(self.to_bytes(), f)
    }
}

impl AsRef<CStr> for CStr {
    fn as_ref(&self) -> &CStr {
        self
    }
}

impl ToOwned for CStr {
    type Owned = CString;
    fn to_owned(&self) -> CString {
        CString { inner: self.inner.to_vec() }
    }
}

// ---------------------------------------------------------------------------
// CString

/// A type representing an owned, C-compatible, nul-terminated string with no nul bytes in the middle.
#[derive(PartialEq, PartialOrd, Eq, Ord, Hash, Clone)]
pub struct CString {
    // the bytes including the trailing nul
    inner: Vec<u8>,
}

/// An error indicating that an interior nul byte was found.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NulError(usize, Vec<u8>);

impl NulError {
    /// Returns the position of the nul byte in the slice that caused `CString::new` to fail.
    pub fn nul_position(&self) -> usize {
        self.0
    }

    /// Consumes this error, returning the underlying vector of bytes which generated the error.
    pub fn into_vec(self) -> Vec<u8> {
        self.1
    }
}

impl fmt::Display for NulError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "nul byte found in provided data at position: {}", self.0)
    }
}

impl crate::error::Error for NulError {}

/// An error indicating invalid UTF-8 when converting a `CString` into a `String`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IntoStringError {
    inner: CString,
    error: Utf8Error,
}

impl IntoStringError {
    /// Consumes this error, returning original `CString` which generated the error.
    pub fn into_cstring(self) -> CString {
        self.inner
    }

    /// Access the underlying UTF-8 error that was the cause of this error.
    pub fn utf8_error(&self) -> Utf8Error {
        self.error.clone()
    }
}

impl fmt::Display for IntoStringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("C string contained non-utf8 bytes")
    }
}

impl crate::error::Error for IntoStringError {}

impl CString {
    /// Creates a new C-compatible string from a container of bytes.
    pub fn new<T: Into<Vec<u8>>>(t: T) -> Result<CString, NulError> {
        let bytes: Vec<u8> = t.into();
        match bytes.iter().position(|&b| b == 0) {
            Some(i) => Err(NulError(i, bytes)),
            None => Ok(unsafe { CString::from_vec_unchecked(bytes) }),
        }
    }

    /// Creates a C-compatible string by consuming a byte vector, without checking for interior 0 bytes.
    pub unsafe fn from_vec_unchecked(v: Vec<u8>) -> CString {
        let mut v = v;
        v.reserve_exact(1);
        v.push(0);
        CString { inner: v }
    }

    /// Converts a `Vec<u8>` to a `CString` without checking the invariants (the vector must end with its only nul).
    pub unsafe fn from_vec_with_nul_unchecked(v: Vec<u8>) -> CString {
        CString { inner: v }
    }

    /// Attempts to convert a `Vec<u8>` to a `CString` (the vector must end with its only nul).
    pub fn from_vec_with_nul(v: Vec<u8>) -> Result<CString, FromVecWithNulError> {
        match v.iter().position(|&b| b == 0) {
            Some(i) if i + 1 == v.len() => Ok(CString { inner: v }),
            Some(i) => Err(FromVecWithNulError { error_kind: FromBytesWithNulError::InteriorNul { position: i }, bytes: v }),
            None => Err(FromVecWithNulError { error_kind: FromBytesWithNulError::NotNulTerminated, bytes: v }),
        }
    }

    /// Retakes ownership of a `CString` that was transferred to C via `CString::into_raw`.
    pub unsafe fn from_raw(ptr: *mut c_char) -> CString {
        let len = intrinsics::cstr_len(ptr as *const u8) + 1;
        let slice = intrinsics::slice_from_raw_parts_mut(ptr as *mut u8, len);
        let b: Box<[u8]> = Box::from_raw(slice);
        CString { inner: b.into_vec() }
    }

    /// Consumes the `CString` and transfers ownership of the string to a C caller.
    pub fn into_raw(self) -> *mut c_char {
        let b: Box<[u8]> = self.inner.into_boxed_slice();
        Box::into_raw(b) as *mut u8 as *mut c_char
    }

    /// Converts the `CString` into a `String` if it contains valid UTF-8 data.
    pub fn into_string(self) -> Result<String, IntoStringError> {
        match crate::str::from_utf8(self.as_bytes()) {
            Ok(_) => {
                let mut v = self.inner;
                v.pop();
                Ok(unsafe { String::from_utf8_unchecked(v) })
            }
            Err(e) => Err(IntoStringError { inner: self, error: e }),
        }
    }

    /// Consumes the `CString` and returns the underlying byte buffer (without the nul).
    pub fn into_bytes(self) -> Vec<u8> {
        let mut v = self.inner;
        v.pop();
        v
    }

    /// Equivalent to `CString::into_bytes()` except that the returned vector includes the trailing nul terminator.
    pub fn into_bytes_with_nul(self) -> Vec<u8> {
        self.inner
    }

    /// Returns the contents of this `CString` as a slice of bytes (without the nul).
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.inner[..self.inner.len() - 1]
    }

    /// Equivalent to `CString::as_bytes()` except that the returned slice includes the trailing nul terminator.
    #[inline]
    pub fn as_bytes_with_nul(&self) -> &[u8] {
        &self.inner
    }

    /// Extracts a `CStr` slice containing the entire string.
    #[inline]
    pub fn as_c_str(&self) -> &CStr {
        unsafe { CStr::from_bytes_with_nul_unchecked(&self.inner) }
    }

    /// Converts this `CString` into a boxed `CStr` (here: returns itself).
    pub fn into_boxed_c_str(self) -> CString {
        self
    }
}

/// An error indicating that a nul byte was not in the expected position (`CString::from_vec_with_nul`).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FromVecWithNulError {
    error_kind: FromBytesWithNulError,
    bytes: Vec<u8>,
}

impl FromVecWithNulError {
    /// Returns a slice of the bytes that were attempted to convert to a `CString`.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the bytes that were attempted to convert to a `CString`.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl fmt::Display for FromVecWithNulError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.error_kind {
            FromBytesWithNulError::InteriorNul { position } => {
                write!(f, "data provided contains an interior nul byte at pos {}", position)
            }
            FromBytesWithNulError::NotNulTerminated => f.write_str("data provided is not nul terminated"),
        }
    }
}

impl crate::error::Error for FromVecWithNulError {}

impl Deref for CString {
    type Target = CStr;
    #[inline]
    fn deref(&self) -> &CStr {
        self.as_c_str()
    }
}

impl AsRef<CStr> for CString {
    fn as_ref(&self) -> &CStr {
        self.as_c_str()
    }
}

impl Borrow<CStr> for CString {
    fn borrow(&self) -> &CStr {
        self.as_c_str()
    }
}

impl Default for CString {
    fn default() -> CString {
        CString { inner: vec![0u8] }
    }
}

impl fmt::Debug for CString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_bytes(self.as_bytes(), f)
    }
}

impl From<&CStr> for CString {
    fn from(s: &CStr) -> CString {
        CString { inner: s.to_bytes_with_nul().to_vec() }
    }
}

impl From<CString> for Vec<u8> {
    fn from(s: CString) -> Vec<u8> {
        s.into_bytes()
    }
}

// ---------------------------------------------------------------------------
// OsString / OsStr (UTF-8 based in this implementation)

/// Borrowed reference to an OS string.
pub struct OsStr {
    _z: [u8; 0],
    inner: str,
}

/// A type that can represent owned, mutable platform-native strings.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct OsString {
    inner: String,
}

impl OsStr {
    /// Coerces into an `OsStr` slice.
    #[inline]
    pub fn new<S: AsRef<OsStr> + ?Sized>(s: &S) -> &OsStr {
        s.as_ref()
    }

    #[inline]
    fn from_str(s: &str) -> &OsStr {
        unsafe { &*(s as *const str as *const OsStr) }
    }

    /// Yields a `&str` slice if the `OsStr` is valid Unicode.
    #[inline]
    pub fn to_str(&self) -> Option<&str> {
        Some(&self.inner)
    }

    /// Converts an `OsStr` to a `Cow<str>`.
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.inner)
    }

    /// Copies the slice into an owned `OsString`.
    pub fn to_os_string(&self) -> OsString {
        OsString { inner: String::from(&self.inner) }
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

    /// Converts an OS string slice to a byte slice.
    #[inline]
    pub fn as_encoded_bytes(&self) -> &[u8] {
        self.inner.as_bytes()
    }

    /// Converts this string to its ASCII lower case equivalent.
    pub fn to_ascii_lowercase(&self) -> OsString {
        OsString { inner: self.inner.to_ascii_lowercase() }
    }

    /// Converts this string to its ASCII upper case equivalent.
    pub fn to_ascii_uppercase(&self) -> OsString {
        OsString { inner: self.inner.to_ascii_uppercase() }
    }

    /// Checks that two strings are an ASCII case-insensitive match.
    pub fn eq_ignore_ascii_case<S: AsRef<OsStr>>(&self, other: S) -> bool {
        self.inner.eq_ignore_ascii_case(&other.as_ref().inner)
    }

    /// Returns an object that implements `Display` for safely printing an `OsStr`.
    pub fn display(&self) -> &str {
        &self.inner
    }
}

impl OsString {
    /// Constructs a new empty `OsString`.
    #[inline]
    pub fn new() -> OsString {
        OsString { inner: String::new() }
    }

    /// Creates a new `OsString` with at least the given capacity.
    pub fn with_capacity(capacity: usize) -> OsString {
        OsString { inner: String::with_capacity(capacity) }
    }

    /// Converts to an `OsStr` slice.
    #[inline]
    pub fn as_os_str(&self) -> &OsStr {
        OsStr::from_str(&self.inner)
    }

    /// Converts the `OsString` into a `String` if it contains valid Unicode data.
    pub fn into_string(self) -> Result<String, OsString> {
        Ok(self.inner)
    }

    /// Extends the string with the given `&OsStr` slice.
    pub fn push<T: AsRef<OsStr>>(&mut self, s: T) {
        self.inner.push_str(&s.as_ref().inner)
    }

    /// Truncates the `OsString` to zero length.
    pub fn clear(&mut self) {
        self.inner.clear()
    }

    /// Returns the capacity this `OsString` can hold without reallocating.
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Converts the `OsString` into a byte vector.
    pub fn into_encoded_bytes(self) -> Vec<u8> {
        self.inner.into_bytes()
    }
}

impl Deref for OsString {
    type Target = OsStr;
    #[inline]
    fn deref(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl AsRef<OsStr> for OsStr {
    fn as_ref(&self) -> &OsStr {
        self
    }
}

impl AsRef<OsStr> for OsString {
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl AsRef<OsStr> for str {
    fn as_ref(&self) -> &OsStr {
        OsStr::from_str(self)
    }
}

impl AsRef<OsStr> for String {
    fn as_ref(&self) -> &OsStr {
        OsStr::from_str(self.as_str())
    }
}

impl Borrow<OsStr> for OsString {
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

impl From<String> for OsString {
    fn from(s: String) -> OsString {
        OsString { inner: s }
    }
}

impl From<&str> for OsString {
    fn from(s: &str) -> OsString {
        OsString { inner: String::from(s) }
    }
}

impl From<OsString> for String {
    fn from(s: OsString) -> String {
        s.inner
    }
}

impl PartialEq for OsStr {
    fn eq(&self, other: &OsStr) -> bool {
        self.inner == other.inner
    }
}

impl Eq for OsStr {}

impl PartialEq<str> for OsStr {
    fn eq(&self, other: &str) -> bool {
        &self.inner == other
    }
}

impl PartialEq<str> for OsString {
    fn eq(&self, other: &str) -> bool {
        self.inner.as_str() == other
    }
}

impl<'a> PartialEq<&'a str> for OsString {
    fn eq(&self, other: &&'a str) -> bool {
        self.inner.as_str() == *other
    }
}

impl PartialOrd for OsStr {
    fn partial_cmp(&self, other: &OsStr) -> Option<Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl Ord for OsStr {
    fn cmp(&self, other: &OsStr) -> Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl Hash for OsStr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state)
    }
}

impl fmt::Debug for OsStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl fmt::Debug for OsString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.inner.as_str(), f)
    }
}
