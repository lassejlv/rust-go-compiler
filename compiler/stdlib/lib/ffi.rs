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

    /// Converts this `CString` into a boxed `CStr`.
    pub fn into_boxed_c_str(self) -> Box<CStr> {
        let b: Box<[u8]> = self.inner.into_boxed_slice();
        unsafe { Box::from_raw(Box::into_raw(b) as *mut CStr) }
    }

    /// Retakes ownership of a `CString` that was transferred to C via `CString::into_raw`.
    pub unsafe fn from_raw(ptr: *mut c_char) -> CString {
        let len = intrinsics::cstr_len(ptr as *const u8) + 1;
        let v = Vec::from_raw_parts(ptr as *mut u8, len, len);
        CString { inner: v }
    }

    /// Creates a C-compatible string by consuming a byte vector, without checking for interior 0 bytes.
    pub unsafe fn from_vec_unchecked(v: Vec<u8>) -> CString {
        let mut v = v;
        v.push(0);
        CString { inner: v }
    }
}

impl CStr {
    /// Converts a `Box<CStr>` into a `CString` without copying or allocating.
    pub fn into_c_string(self: Box<CStr>) -> CString {
        CString::from(&*self)
    }
}

impl Clone for Box<CStr> {
    fn clone(&self) -> Self {
        CString::from(&**self).into_boxed_c_str()
    }
}

impl From<&CStr> for Box<CStr> {
    fn from(s: &CStr) -> Box<CStr> {
        CString::from(s).into_boxed_c_str()
    }
}

impl From<CString> for Box<CStr> {
    fn from(s: CString) -> Box<CStr> {
        s.into_boxed_c_str()
    }
}

impl From<Box<CStr>> for CString {
    fn from(s: Box<CStr>) -> CString {
        s.into_c_string()
    }
}

impl From<CString> for crate::sync::Arc<CStr> {
    fn from(s: CString) -> crate::sync::Arc<CStr> {
        let arc: crate::sync::Arc<[u8]> = crate::sync::Arc::from(s.inner);
        unsafe { crate::mem::transmute::<crate::sync::Arc<[u8]>, crate::sync::Arc<CStr>>(arc) }
    }
}

impl From<&CStr> for crate::sync::Arc<CStr> {
    fn from(s: &CStr) -> crate::sync::Arc<CStr> {
        crate::sync::Arc::from(CString::from(s))
    }
}

impl From<CString> for crate::rc::Rc<CStr> {
    fn from(s: CString) -> crate::rc::Rc<CStr> {
        let rc: crate::rc::Rc<[u8]> = crate::rc::Rc::from(s.inner);
        unsafe { crate::mem::transmute::<crate::rc::Rc<[u8]>, crate::rc::Rc<CStr>>(rc) }
    }
}

impl From<&CStr> for crate::rc::Rc<CStr> {
    fn from(s: &CStr) -> crate::rc::Rc<CStr> {
        crate::rc::Rc::from(CString::from(s))
    }
}

impl<'a> From<Cow<'a, CStr>> for CString {
    fn from(s: Cow<'a, CStr>) -> CString {
        s.into_owned()
    }
}

impl<'a> From<&'a CStr> for Cow<'a, CStr> {
    fn from(s: &'a CStr) -> Cow<'a, CStr> {
        Cow::Borrowed(s)
    }
}

impl<'a> From<&'a CString> for Cow<'a, CStr> {
    fn from(s: &'a CString) -> Cow<'a, CStr> {
        Cow::Borrowed(s.as_c_str())
    }
}

impl<'a> From<CString> for Cow<'a, CStr> {
    fn from(s: CString) -> Cow<'a, CStr> {
        Cow::Owned(s)
    }
}

impl From<Vec<crate::num::NonZero<u8>>> for CString {
    fn from(v: Vec<crate::num::NonZero<u8>>) -> CString {
        let mut bytes: Vec<u8> = Vec::with_capacity(v.len() + 1);
        for b in v {
            bytes.push(b.get());
        }
        bytes.push(0);
        CString { inner: bytes }
    }
}

impl Default for &CStr {
    fn default() -> Self {
        unsafe { CStr::from_bytes_with_nul_unchecked(b"\0") }
    }
}

impl Default for Box<CStr> {
    fn default() -> Self {
        CString::default().into_boxed_c_str()
    }
}

impl crate::ops::Index<crate::ops::RangeFull> for CString {
    type Output = CStr;
    fn index(&self, _index: crate::ops::RangeFull) -> &CStr {
        self.as_c_str()
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
// OsString / OsStr

pub mod os_str;
pub use self::os_str::{OsStr, OsString};
