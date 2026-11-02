//! Inspection and manipulation of the process's environment (not available
//! in this runtime: there are no arguments and no environment variables).

use crate::fmt;
use crate::iter::{DoubleEndedIterator, ExactSizeIterator, Iterator};

/// An iterator over the arguments of a process, yielding a `String` value for each argument.
pub struct Args {
    inner: crate::vec::IntoIter<String>,
}

/// Returns the arguments that this program was started with.
pub fn args() -> Args {
    Args { inner: Vec::new().into_iter() }
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

/// The error type for operations interacting with environment variables.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum VarError {
    /// The specified environment variable was not present in the current process's environment.
    NotPresent,
    /// The specified environment variable was found, but it did not contain valid unicode data.
    NotUnicode(String),
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

/// Fetches the environment variable `key` from the current process.
pub fn var<K: AsRef<str>>(key: K) -> Result<String, VarError> {
    let _ = key;
    Err(VarError::NotPresent)
}

/// An iterator over a snapshot of the environment variables of this process.
pub struct Vars {
    inner: crate::vec::IntoIter<(String, String)>,
}

/// Returns an iterator of (variable, value) pairs of strings, for all the environment variables of the current process.
pub fn vars() -> Vars {
    Vars { inner: Vec::new().into_iter() }
}

impl Iterator for Vars {
    type Item = (String, String);
    fn next(&mut self) -> Option<(String, String)> {
        self.inner.next()
    }
}
