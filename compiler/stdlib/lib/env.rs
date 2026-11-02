//! Inspection and manipulation of the process's environment.

use crate::ffi::{OsStr, OsString};
use crate::fmt;
use crate::intrinsics;
use crate::iter::{DoubleEndedIterator, ExactSizeIterator, Iterator};

// Copies a NUL-terminated C string into a `String` (invalid UTF-8 is replaced).
unsafe fn c_string(p: *const u8) -> String {
    let n = intrinsics::cstr_len(p);
    String::from_utf8_lossy(crate::slice::from_raw_parts(p, n)).into_owned()
}

fn collect_args() -> Vec<String> {
    let n = unsafe { intrinsics::args_count() };
    let mut v = Vec::with_capacity(n);
    let mut i = 0;
    while i < n {
        unsafe {
            let p = intrinsics::arg_ptr(i);
            let len = intrinsics::arg_len(i);
            v.push(String::from_utf8_lossy(crate::slice::from_raw_parts(p, len)).into_owned());
        }
        i += 1;
    }
    v
}

/// An iterator over the arguments of a process, yielding a `String` value for each argument.
pub struct Args {
    inner: crate::vec::IntoIter<String>,
}

/// An iterator over the arguments of a process, yielding an `OsString` value for each argument.
pub struct ArgsOs {
    inner: crate::vec::IntoIter<String>,
}

/// Returns the arguments that this program was started with (normally passed via the command line).
pub fn args() -> Args {
    Args { inner: collect_args().into_iter() }
}

/// Returns the arguments that this program was started with, as `OsString`s.
pub fn args_os() -> ArgsOs {
    ArgsOs { inner: collect_args().into_iter() }
}

impl Iterator for Args {
    type Item = String;
    fn next(&mut self) -> Option<String> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for Args {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl DoubleEndedIterator for Args {
    fn next_back(&mut self) -> Option<String> {
        self.inner.next_back()
    }
}

impl fmt::Debug for Args {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Args").field("args", &self.inner.as_slice()).finish()
    }
}

impl Iterator for ArgsOs {
    type Item = OsString;
    fn next(&mut self) -> Option<OsString> {
        match self.inner.next() {
            Some(s) => Some(OsString::from(s)),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for ArgsOs {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl DoubleEndedIterator for ArgsOs {
    fn next_back(&mut self) -> Option<OsString> {
        match self.inner.next_back() {
            Some(s) => Some(OsString::from(s)),
            None => None,
        }
    }
}

impl fmt::Debug for ArgsOs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArgsOs").field("args", &self.inner.as_slice()).finish()
    }
}

/// The error type for operations interacting with environment variables.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum VarError {
    /// The specified environment variable was not present in the current process's environment.
    NotPresent,
    /// The specified environment variable was found, but it did not contain valid unicode data.
    NotUnicode(OsString),
}

impl fmt::Display for VarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            VarError::NotPresent => f.write_str("environment variable not found"),
            VarError::NotUnicode(ref s) => write!(f, "environment variable was not valid unicode: {:?}", s),
        }
    }
}

impl crate::error::Error for VarError {}

fn getenv(key: &str) -> Option<Vec<u8>> {
    if key.is_empty() || key.as_bytes().contains(&b'=') || key.as_bytes().contains(&0) {
        return None;
    }
    unsafe {
        let p = intrinsics::env_get(key.as_ptr(), key.len());
        if (p as usize) == 0 {
            None
        } else {
            let n = intrinsics::cstr_len(p);
            Some(crate::slice::from_raw_parts(p, n).to_vec())
        }
    }
}

/// Fetches the environment variable `key` from the current process.
pub fn var<K: AsRef<OsStr>>(key: K) -> Result<String, VarError> {
    let key = key.as_ref();
    match getenv(key.to_str().unwrap_or("")) {
        None => Err(VarError::NotPresent),
        Some(bytes) => match String::from_utf8(bytes) {
            Ok(s) => Ok(s),
            Err(e) => Err(VarError::NotUnicode(OsString::from(String::from_utf8_lossy(e.as_bytes()).into_owned()))),
        },
    }
}

/// Fetches the environment variable `key` from the current process, returning `None` if it is not set.
pub fn var_os<K: AsRef<OsStr>>(key: K) -> Option<OsString> {
    let key = key.as_ref();
    match getenv(key.to_str().unwrap_or("")) {
        None => None,
        Some(bytes) => Some(OsString::from(String::from_utf8_lossy(&bytes).into_owned())),
    }
}

fn collect_vars() -> Vec<(String, String)> {
    let n = unsafe { intrinsics::env_count() };
    let mut v = Vec::with_capacity(n);
    let mut i = 0;
    while i < n {
        let entry = unsafe { c_string(intrinsics::env_entry(i)) };
        // a leading '=' belongs to the key (like the real implementation)
        let search_from = if entry.starts_with('=') { 1 } else { 0 };
        if let Some(pos) = entry[search_from..].find('=') {
            let pos = pos + search_from;
            v.push((String::from(&entry[..pos]), String::from(&entry[pos + 1..])));
        }
        i += 1;
    }
    v
}

/// An iterator over a snapshot of the environment variables of this process.
pub struct Vars {
    inner: crate::vec::IntoIter<(String, String)>,
}

/// An iterator over a snapshot of the environment variables of this process (as `OsString`s).
pub struct VarsOs {
    inner: crate::vec::IntoIter<(String, String)>,
}

/// Returns an iterator of (variable, value) pairs of strings, for all the environment variables of the current process.
pub fn vars() -> Vars {
    Vars { inner: collect_vars().into_iter() }
}

/// Returns an iterator of (variable, value) pairs of OS strings, for all the environment variables of the current process.
pub fn vars_os() -> VarsOs {
    VarsOs { inner: collect_vars().into_iter() }
}

impl Iterator for Vars {
    type Item = (String, String);
    fn next(&mut self) -> Option<(String, String)> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl Iterator for VarsOs {
    type Item = (OsString, OsString);
    fn next(&mut self) -> Option<(OsString, OsString)> {
        match self.inner.next() {
            Some((k, v)) => Some((OsString::from(k), OsString::from(v))),
            None => None,
        }
    }
}

impl fmt::Debug for Vars {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vars").finish_non_exhaustive()
    }
}

impl fmt::Debug for VarsOs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VarsOs").finish_non_exhaustive()
    }
}

/// Constants associated with the current target.
pub mod consts {
    /// A string describing the architecture of the CPU that is currently in use.
    pub const ARCH: &str = "x86_64";
    /// The family of the operating system.
    pub const FAMILY: &str = "unix";
    /// A string describing the specific operating system in use.
    pub const OS: &str = "linux";
    /// Specifies the filename prefix used for shared libraries on this platform.
    pub const DLL_PREFIX: &str = "lib";
    /// Specifies the filename suffix used for shared libraries on this platform.
    pub const DLL_SUFFIX: &str = ".so";
    /// Specifies the file extension used for shared libraries on this platform.
    pub const DLL_EXTENSION: &str = "so";
    /// Specifies the filename suffix used for executable binaries on this platform.
    pub const EXE_SUFFIX: &str = "";
    /// Specifies the file extension, if any, used for executable binaries on this platform.
    pub const EXE_EXTENSION: &str = "";
}
