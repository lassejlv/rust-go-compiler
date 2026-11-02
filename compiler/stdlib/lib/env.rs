//! Inspection and manipulation of the process's environment.

use crate::ffi::{OsStr, OsString};
use crate::path::{Path, PathBuf};
use crate::fmt;
use crate::intrinsics;
use crate::iter::{DoubleEndedIterator, ExactSizeIterator, Iterator};


fn collect_args() -> Vec<String> {
    let mut v = Vec::new();
    for a in collect_args_os() {
        match a.into_string() {
            Ok(s) => v.push(s),
            Err(a) => panic!("called `Option::unwrap()` on a `None` value: {:?}", a),
        }
    }
    v
}

fn collect_args_os() -> Vec<OsString> {
    let n = unsafe { intrinsics::args_count() };
    let mut v = Vec::with_capacity(n);
    let mut i = 0;
    while i < n {
        unsafe {
            let p = intrinsics::arg_ptr(i);
            let len = intrinsics::arg_len(i);
            v.push(OsString::from_vec(crate::slice::from_raw_parts(p, len).to_vec()));
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
    inner: crate::vec::IntoIter<OsString>,
}

/// Returns the arguments that this program was started with (normally passed via the command line).
pub fn args() -> Args {
    Args { inner: collect_args().into_iter() }
}

/// Returns the arguments that this program was started with, as `OsString`s.
pub fn args_os() -> ArgsOs {
    ArgsOs { inner: collect_args_os().into_iter() }
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
        self.inner.next()
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
        self.inner.next_back()
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

fn getenv(key: &OsStr) -> Option<Vec<u8>> {
    let key = key.as_encoded_bytes();
    if key.is_empty() || key.contains(&b'=') || key.contains(&0) {
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
    match getenv(key.as_ref()) {
        None => Err(VarError::NotPresent),
        Some(bytes) => match String::from_utf8(bytes) {
            Ok(s) => Ok(s),
            Err(e) => Err(VarError::NotUnicode(OsString::from_vec(e.into_bytes()))),
        },
    }
}

/// Fetches the environment variable `key` from the current process, returning `None` if it is not set.
pub fn var_os<K: AsRef<OsStr>>(key: K) -> Option<OsString> {
    match getenv(key.as_ref()) {
        None => None,
        Some(bytes) => Some(OsString::from_vec(bytes)),
    }
}

fn collect_vars() -> Vec<(OsString, OsString)> {
    let n = unsafe { intrinsics::env_count() };
    let mut v = Vec::with_capacity(n);
    let mut i = 0;
    while i < n {
        let entry = unsafe { crate::sys::from_cstr(intrinsics::env_entry(i)) };
        // a leading '=' belongs to the key (like the real implementation)
        if !entry.is_empty() {
            if let Some(pos) = entry[1..].iter().position(|&b| b == b'=') {
                let pos = pos + 1;
                v.push((OsString::from_vec(entry[..pos].to_vec()), OsString::from_vec(entry[pos + 1..].to_vec())));
            }
        }
        i += 1;
    }
    v
}

/// An iterator over a snapshot of the environment variables of this process.
pub struct Vars {
    inner: crate::vec::IntoIter<(OsString, OsString)>,
}

/// An iterator over a snapshot of the environment variables of this process (as `OsString`s).
pub struct VarsOs {
    inner: crate::vec::IntoIter<(OsString, OsString)>,
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
        match self.inner.next() {
            Some((k, v)) => Some((k.into_string().unwrap(), v.into_string().unwrap())),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl Iterator for VarsOs {
    type Item = (OsString, OsString);
    fn next(&mut self) -> Option<(OsString, OsString)> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
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

/// Sets the environment variable `key` to the value `value` for the currently running process.
pub fn set_var<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) {
    let key = key.as_ref();
    let value = value.as_ref();
    let r = (|| -> crate::io::Result<()> {
        let k = crate::sys::cstr(key.as_encoded_bytes())?;
        let v = crate::sys::cstr(value.as_encoded_bytes())?;
        unsafe { crate::sys::cvt(crate::sys::c_setenv(k.as_ptr(), v.as_ptr(), 1))? };
        Ok(())
    })();
    if let Err(e) = r {
        panic!("failed to set environment variable `{:?}` to `{:?}`: {}", key, value, e);
    }
}

/// Removes an environment variable from the environment of the currently running process.
pub fn remove_var<K: AsRef<OsStr>>(key: K) {
    let key = key.as_ref();
    let r = (|| -> crate::io::Result<()> {
        let k = crate::sys::cstr(key.as_encoded_bytes())?;
        unsafe { crate::sys::cvt(crate::sys::c_unsetenv(k.as_ptr()))? };
        Ok(())
    })();
    if let Err(e) = r {
        panic!("failed to remove environment variable `{:?}`: {}", key, e);
    }
}

/// Returns the current working directory as a `PathBuf`.
pub fn current_dir() -> crate::io::Result<PathBuf> {
    let mut buf: Vec<u8> = Vec::with_capacity(512);
    loop {
        let cap = buf.capacity();
        unsafe {
            let p = crate::sys::c_getcwd(buf.as_mut_ptr(), cap);
            if !p.is_null() {
                let len = intrinsics::cstr_len(buf.as_ptr());
                buf.set_len(len);
                buf.shrink_to_fit();
                return Ok(PathBuf::from(OsString::from_vec(buf)));
            }
            let e = crate::io::Error::last_os_error();
            if e.raw_os_error() != Some(34) {
                return Err(e);
            }
        }
        buf.reserve(cap * 2);
    }
}

/// Changes the current working directory to the specified path.
pub fn set_current_dir<P: AsRef<Path>>(path: P) -> crate::io::Result<()> {
    let p = crate::sys::cstr(path.as_ref().as_os_str().as_encoded_bytes())?;
    unsafe { crate::sys::cvt(crate::sys::c_chdir(p.as_ptr()))? };
    Ok(())
}

/// Returns the path of the current user's home directory if known.
pub fn home_dir() -> Option<PathBuf> {
    match var_os("HOME") {
        Some(h) => Some(PathBuf::from(h)),
        None => None,
    }
}

/// Returns the path of a temporary directory.
pub fn temp_dir() -> PathBuf {
    match var_os("TMPDIR") {
        Some(t) => PathBuf::from(t),
        None => PathBuf::from("/tmp"),
    }
}

/// Returns the full filesystem path of the current running executable.
pub fn current_exe() -> crate::io::Result<PathBuf> {
    match crate::fs::read_link("/proc/self/exe") {
        Err(ref e) if e.kind() == crate::io::ErrorKind::NotFound => Err(crate::io::const_io_error(
            crate::io::ErrorKind::Uncategorized,
            "no /proc/self/exe available. Is /proc mounted?",
        )),
        other => other,
    }
}

/// An iterator that splits an environment variable into paths according to platform-specific conventions.
pub struct SplitPaths<'a> {
    rest: Option<&'a [u8]>,
}

/// Parses input according to platform conventions for the `PATH` environment variable.
pub fn split_paths<T: AsRef<OsStr> + ?Sized>(unparsed: &T) -> SplitPaths<'_> {
    SplitPaths { rest: Some(unparsed.as_ref().as_encoded_bytes()) }
}

impl<'a> Iterator for SplitPaths<'a> {
    type Item = PathBuf;
    fn next(&mut self) -> Option<PathBuf> {
        let rest = self.rest?;
        match rest.iter().position(|&b| b == b':') {
            Some(i) => {
                self.rest = Some(&rest[i + 1..]);
                Some(PathBuf::from(OsString::from_vec(rest[..i].to_vec())))
            }
            None => {
                self.rest = None;
                Some(PathBuf::from(OsString::from_vec(rest.to_vec())))
            }
        }
    }
}

impl<'a> fmt::Debug for SplitPaths<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SplitPaths").finish_non_exhaustive()
    }
}

/// The error type for operations on the `PATH` variable.
#[derive(Debug)]
pub struct JoinPathsError {
    _priv: (),
}

impl fmt::Display for JoinPathsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "path segment contains separator `:`")
    }
}

impl crate::error::Error for JoinPathsError {}

/// Joins a collection of `Path`s appropriately for the `PATH` environment variable.
pub fn join_paths<I, T>(paths: I) -> Result<OsString, JoinPathsError>
where
    I: IntoIterator<Item = T>,
    T: AsRef<OsStr>,
{
    let mut joined: Vec<u8> = Vec::new();
    let mut first = true;
    for path in paths {
        let path = path.as_ref().as_encoded_bytes();
        if !first {
            joined.push(b':');
        }
        first = false;
        if path.contains(&b':') {
            return Err(JoinPathsError { _priv: () });
        }
        joined.extend_from_slice(path);
    }
    Ok(OsString::from_vec(joined))
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
