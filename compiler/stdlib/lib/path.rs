//! Cross-platform path manipulation (Unix semantics).

use crate::borrow::{Borrow, Cow, ToOwned};
use crate::cmp::Ordering;
use crate::error::Error;
use crate::ffi::{OsStr, OsString};
use crate::fmt;
use crate::fs;
use crate::hash::{Hash, Hasher};
use crate::io;
use crate::iter::{DoubleEndedIterator, FusedIterator, Iterator};
use crate::ops::{Deref, DerefMut};
use crate::str::FromStr;

/// The primary separator of path components for the current platform.
pub const MAIN_SEPARATOR: char = '/';

/// The primary separator of path components for the current platform, as a string.
pub const MAIN_SEPARATOR_STR: &str = "/";

/// Determines whether the character is one of the permitted path separators.
#[inline]
pub fn is_separator(c: char) -> bool {
    c == '/'
}

#[inline]
fn is_sep_byte(b: u8) -> bool {
    b == b'/'
}

#[inline]
fn os(bytes: &[u8]) -> &OsStr {
    OsStr::from_inner(bytes)
}

// ---------------------------------------------------------------------------
// Prefix (Windows only; never produced on Unix)

/// Windows path prefixes (never produced on Unix).
#[derive(Copy, Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum Prefix<'a> {
    Verbatim(&'a OsStr),
    VerbatimUNC(&'a OsStr, &'a OsStr),
    VerbatimDisk(u8),
    DeviceNS(&'a OsStr),
    UNC(&'a OsStr, &'a OsStr),
    Disk(u8),
}

impl<'a> Prefix<'a> {
    /// Determines if the prefix is verbatim, i.e., begins with `\\?\`.
    #[inline]
    pub fn is_verbatim(&self) -> bool {
        match *self {
            Prefix::Verbatim(_) | Prefix::VerbatimDisk(_) | Prefix::VerbatimUNC(..) => true,
            _ => false,
        }
    }
}

/// A structure wrapping a Windows path prefix as well as its unparsed string representation.
#[derive(Copy, Clone, Eq, Debug)]
pub struct PrefixComponent<'a> {
    raw: &'a OsStr,
    parsed: Prefix<'a>,
}

impl<'a> PrefixComponent<'a> {
    /// Returns the parsed prefix data.
    pub fn kind(&self) -> Prefix<'a> {
        self.parsed
    }

    /// Returns the raw `OsStr` slice for this prefix.
    pub fn as_os_str(&self) -> &'a OsStr {
        self.raw
    }
}

impl<'a> PartialEq for PrefixComponent<'a> {
    fn eq(&self, other: &PrefixComponent<'a>) -> bool {
        self.parsed == other.parsed
    }
}

impl<'a> PartialOrd for PrefixComponent<'a> {
    fn partial_cmp(&self, other: &PrefixComponent<'a>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&self.parsed, &other.parsed)
    }
}

impl<'a> Ord for PrefixComponent<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&self.parsed, &other.parsed)
    }
}

impl<'a> Hash for PrefixComponent<'a> {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.parsed.hash(h);
    }
}

// ---------------------------------------------------------------------------
// Component

/// A single component of a path.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Component<'a> {
    /// A Windows path prefix (never produced on Unix).
    Prefix(PrefixComponent<'a>),
    /// The root directory component (`/`).
    RootDir,
    /// A reference to the current directory, i.e., `.`.
    CurDir,
    /// A reference to the parent directory, i.e., `..`.
    ParentDir,
    /// A normal component, e.g., `a` and `b` in `a/b`.
    Normal(&'a OsStr),
}

impl<'a> Component<'a> {
    /// Extracts the underlying `OsStr` slice.
    pub fn as_os_str(self) -> &'a OsStr {
        match self {
            Component::Prefix(p) => p.as_os_str(),
            Component::RootDir => os(b"/"),
            Component::CurDir => os(b"."),
            Component::ParentDir => os(b".."),
            Component::Normal(path) => path,
        }
    }
}

impl<'a> AsRef<OsStr> for Component<'a> {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl<'a> AsRef<Path> for Component<'a> {
    #[inline]
    fn as_ref(&self) -> &Path {
        Path::new(self.as_os_str())
    }
}

// ---------------------------------------------------------------------------
// Components

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
enum State {
    Prefix = 0,
    StartDir = 1,
    Body = 2,
    Done = 3,
}

/// An iterator over the `Component`s of a `Path`.
#[derive(Clone)]
pub struct Components<'a> {
    path: &'a [u8],
    has_physical_root: bool,
    front: State,
    back: State,
}

/// An iterator over the `Component`s of a `Path`, as `OsStr` slices.
#[derive(Clone)]
pub struct Iter<'a> {
    inner: Components<'a>,
}

impl<'a> fmt::Debug for Components<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct DebugHelper<'a>(&'a Path);
        impl<'a> fmt::Debug for DebugHelper<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.0.components()).finish()
            }
        }
        f.debug_tuple("Components").field(&DebugHelper(self.as_path())).finish()
    }
}

impl<'a> Components<'a> {
    #[inline]
    fn len_before_body(&self) -> usize {
        let root = if self.front <= State::StartDir && self.has_physical_root { 1 } else { 0 };
        let cur_dir = if self.front <= State::StartDir && self.include_cur_dir() { 1 } else { 0 };
        root + cur_dir
    }

    #[inline]
    fn finished(&self) -> bool {
        self.front == State::Done || self.back == State::Done || self.front > self.back
    }

    /// Extracts a slice corresponding to the portion of the path remaining for iteration.
    pub fn as_path(&self) -> &'a Path {
        let mut comps = self.clone();
        if comps.front == State::Body {
            comps.trim_left();
        }
        if comps.back == State::Body {
            comps.trim_right();
        }
        Path::from_bytes(comps.path)
    }

    fn include_cur_dir(&self) -> bool {
        if self.has_physical_root {
            return false;
        }
        let p = self.path;
        if p.len() == 1 {
            p[0] == b'.'
        } else if p.len() >= 2 {
            p[0] == b'.' && is_sep_byte(p[1])
        } else {
            false
        }
    }

    fn parse_single_component<'b>(&self, comp: &'b [u8]) -> Option<Component<'b>> {
        if comp.is_empty() {
            None
        } else if comp == b"." {
            None
        } else if comp == b".." {
            Some(Component::ParentDir)
        } else {
            Some(Component::Normal(os(comp)))
        }
    }

    fn parse_next_component(&self) -> (usize, Option<Component<'a>>) {
        let path = self.path;
        let (extra, comp) = match path.iter().position(|b| is_sep_byte(*b)) {
            None => (0, path),
            Some(i) => (1, &path[..i]),
        };
        (comp.len() + extra, self.parse_single_component(comp))
    }

    fn parse_next_component_back(&self) -> (usize, Option<Component<'a>>) {
        let start = self.len_before_body();
        let path = self.path;
        let (extra, comp) = match path[start..].iter().rposition(|b| is_sep_byte(*b)) {
            None => (0, &path[start..]),
            Some(i) => (1, &path[start + i + 1..]),
        };
        (comp.len() + extra, self.parse_single_component(comp))
    }

    fn trim_left(&mut self) {
        while !self.path.is_empty() {
            let (size, comp) = self.parse_next_component();
            if comp.is_some() {
                return;
            } else {
                self.path = &self.path[size..];
            }
        }
    }

    fn trim_right(&mut self) {
        while self.path.len() > self.len_before_body() {
            let (size, comp) = self.parse_next_component_back();
            if comp.is_some() {
                return;
            } else {
                self.path = &self.path[..self.path.len() - size];
            }
        }
    }
}

impl<'a> AsRef<Path> for Components<'a> {
    #[inline]
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl<'a> AsRef<OsStr> for Components<'a> {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        self.as_path().as_os_str()
    }
}

impl<'a> Iterator for Components<'a> {
    type Item = Component<'a>;

    fn next(&mut self) -> Option<Component<'a>> {
        while !self.finished() {
            match self.front {
                State::Prefix => {
                    self.front = State::StartDir;
                }
                State::StartDir => {
                    self.front = State::Body;
                    if self.has_physical_root {
                        self.path = &self.path[1..];
                        return Some(Component::RootDir);
                    } else if self.include_cur_dir() {
                        self.path = &self.path[1..];
                        return Some(Component::CurDir);
                    }
                }
                State::Body => {
                    if !self.path.is_empty() {
                        let (size, comp) = self.parse_next_component();
                        self.path = &self.path[size..];
                        if comp.is_some() {
                            return comp;
                        }
                    } else {
                        self.front = State::Done;
                    }
                }
                State::Done => unreachable!(),
            }
        }
        None
    }
}

impl<'a> DoubleEndedIterator for Components<'a> {
    fn next_back(&mut self) -> Option<Component<'a>> {
        while !self.finished() {
            match self.back {
                State::Body => {
                    if self.path.len() > self.len_before_body() {
                        let (size, comp) = self.parse_next_component_back();
                        self.path = &self.path[..self.path.len() - size];
                        if comp.is_some() {
                            return comp;
                        }
                    } else {
                        self.back = State::StartDir;
                    }
                }
                State::StartDir => {
                    self.back = State::Prefix;
                    if self.has_physical_root {
                        self.path = &self.path[..self.path.len() - 1];
                        return Some(Component::RootDir);
                    } else if self.include_cur_dir() {
                        self.path = &self.path[..self.path.len() - 1];
                        return Some(Component::CurDir);
                    }
                }
                State::Prefix => {
                    self.back = State::Done;
                    return None;
                }
                State::Done => unreachable!(),
            }
        }
        None
    }
}

impl<'a> FusedIterator for Components<'a> {}

impl<'a> PartialEq for Components<'a> {
    #[inline]
    fn eq(&self, other: &Components<'a>) -> bool {
        // Fast path for exact matches, e.g. for hashmap lookups.
        if self.path.len() == other.path.len()
            && self.front == other.front
            && self.back == State::Body
            && other.back == State::Body
        {
            if self.path == other.path {
                return true;
            }
        }
        Iterator::eq(self.clone().rev(), other.clone().rev())
    }
}

impl<'a> Eq for Components<'a> {}

impl<'a> PartialOrd for Components<'a> {
    #[inline]
    fn partial_cmp(&self, other: &Components<'a>) -> Option<Ordering> {
        Some(compare_components(self.clone(), other.clone()))
    }
}

impl<'a> Ord for Components<'a> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        compare_components(self.clone(), other.clone())
    }
}

fn compare_components(mut left: Components<'_>, mut right: Components<'_>) -> Ordering {
    // Fast path for long shared prefixes
    if left.front == right.front {
        let lp = left.path;
        let rp = right.path;
        let n = if lp.len() < rp.len() { lp.len() } else { rp.len() };
        let mut first_difference = n;
        let mut i = 0;
        while i < n {
            if lp[i] != rp[i] {
                first_difference = i;
                break;
            }
            i += 1;
        }
        if first_difference == n && lp.len() == rp.len() {
            return Ordering::Equal;
        }
        if let Some(previous_sep) = lp[..first_difference].iter().rposition(|&b| is_sep_byte(b)) {
            let start = previous_sep + 1;
            left.path = &lp[start..];
            left.front = State::Body;
            right.path = &rp[start..];
            right.front = State::Body;
        }
    }
    Iterator::cmp(left, right)
}

impl<'a> Iter<'a> {
    /// Extracts a slice corresponding to the portion of the path remaining for iteration.
    #[inline]
    pub fn as_path(&self) -> &'a Path {
        self.inner.as_path()
    }
}

impl<'a> fmt::Debug for Iter<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct DebugHelper<'a>(&'a Path);
        impl<'a> fmt::Debug for DebugHelper<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.0.iter()).finish()
            }
        }
        f.debug_tuple("Iter").field(&DebugHelper(self.as_path())).finish()
    }
}

impl<'a> AsRef<Path> for Iter<'a> {
    #[inline]
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl<'a> AsRef<OsStr> for Iter<'a> {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        self.as_path().as_os_str()
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a OsStr;
    #[inline]
    fn next(&mut self) -> Option<&'a OsStr> {
        self.inner.next().map(Component::as_os_str)
    }
}

impl<'a> DoubleEndedIterator for Iter<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a OsStr> {
        self.inner.next_back().map(Component::as_os_str)
    }
}

impl<'a> FusedIterator for Iter<'a> {}

/// An iterator over `Path` and its ancestors.
#[derive(Copy, Clone, Debug)]
pub struct Ancestors<'a> {
    next: Option<&'a Path>,
}

impl<'a> Iterator for Ancestors<'a> {
    type Item = &'a Path;
    #[inline]
    fn next(&mut self) -> Option<&'a Path> {
        let next = self.next;
        self.next = match next {
            Some(p) => p.parent(),
            None => None,
        };
        next
    }
}

impl<'a> FusedIterator for Ancestors<'a> {}

// Iterate through `iter` while it matches `prefix`; return `None` if `prefix`
// is not a prefix of `iter`, otherwise return `Some(iter_after_prefix)`.
fn iter_after<'a, 'b, I, J>(mut iter: I, mut prefix: J) -> Option<I>
where
    I: Iterator<Item = Component<'a>> + Clone,
    J: Iterator<Item = Component<'b>>,
{
    loop {
        let mut iter_next = iter.clone();
        match (iter_next.next(), prefix.next()) {
            (Some(x), Some(y)) => {
                if x.as_os_str().as_encoded_bytes() != y.as_os_str().as_encoded_bytes() || !same_kind(&x, &y) {
                    return None;
                }
            }
            (Some(_), None) => return Some(iter),
            (None, None) => return Some(iter),
            (None, Some(_)) => return None,
        }
        iter = iter_next;
    }
}

fn same_kind(a: &Component<'_>, b: &Component<'_>) -> bool {
    match (a, b) {
        (Component::RootDir, Component::RootDir) => true,
        (Component::CurDir, Component::CurDir) => true,
        (Component::ParentDir, Component::ParentDir) => true,
        (Component::Normal(_), Component::Normal(_)) => true,
        (Component::Prefix(_), Component::Prefix(_)) => true,
        _ => false,
    }
}

// Splits a file name at its last dot ("" before the dot means no extension).
fn rsplit_file_at_dot(file: &OsStr) -> (Option<&OsStr>, Option<&OsStr>) {
    let bytes = file.as_encoded_bytes();
    if bytes == b".." {
        return (Some(file), None);
    }
    match bytes.iter().rposition(|b| *b == b'.') {
        None => (None, Some(file)),
        Some(0) => (Some(file), None),
        Some(i) => (Some(os(&bytes[..i])), Some(os(&bytes[i + 1..]))),
    }
}

// Splits a file name at its first (non-leading) dot.
fn split_file_at_dot(file: &OsStr) -> (&OsStr, Option<&OsStr>) {
    let slice = file.as_encoded_bytes();
    if slice == b".." {
        return (file, None);
    }
    let i = match slice[1..].iter().position(|b| *b == b'.') {
        Some(i) => i + 1,
        None => return (file, None),
    };
    (os(&slice[..i]), Some(os(&slice[i + 1..])))
}

// ---------------------------------------------------------------------------
// PathBuf

/// An owned, mutable path (akin to `String`).
pub struct PathBuf {
    inner: OsString,
}

impl PathBuf {
    #[inline]
    fn as_mut_vec(&mut self) -> &mut Vec<u8> {
        self.inner.as_mut_vec()
    }

    /// Allocates an empty `PathBuf`.
    #[inline]
    pub const fn new() -> PathBuf {
        PathBuf { inner: OsString::new() }
    }

    /// Creates a new `PathBuf` with a given capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> PathBuf {
        PathBuf { inner: OsString::with_capacity(capacity) }
    }

    /// Coerces to a `Path` slice.
    #[inline]
    pub fn as_path(&self) -> &Path {
        Path::from_bytes(self.inner.as_vec())
    }

    /// Consumes and leaks the `PathBuf`, returning a mutable reference to the contents.
    pub fn leak<'a>(self) -> &'a mut Path {
        let b = self.into_boxed_path();
        unsafe { &mut *Box::into_raw(b) }
    }

    /// Extends `self` with `path`.
    pub fn push<P: AsRef<Path>>(&mut self, path: P) {
        self._push(path.as_ref())
    }

    fn _push(&mut self, path: &Path) {
        let need_sep = match self.inner.as_vec().last() {
            Some(c) => !is_sep_byte(*c),
            None => false,
        };
        if path.is_absolute() {
            self.as_mut_vec().truncate(0);
        } else if need_sep {
            self.as_mut_vec().push(b'/');
        }
        self.as_mut_vec().extend_from_slice(path.as_u8_slice());
    }

    /// Truncates `self` to `self.parent`.
    pub fn pop(&mut self) -> bool {
        let len = match self.as_path().parent() {
            Some(p) => p.as_u8_slice().len(),
            None => return false,
        };
        self.as_mut_vec().truncate(len);
        true
    }

    /// Updates `self.file_name()` to `file_name`.
    pub fn set_file_name<S: AsRef<OsStr>>(&mut self, file_name: S) {
        self._set_file_name(file_name.as_ref())
    }

    fn _set_file_name(&mut self, file_name: &OsStr) {
        if self.file_name().is_some() {
            let popped = self.pop();
            debug_assert!(popped);
        }
        self.push(file_name);
    }

    /// Updates `self.extension()` to `Some(extension)` or to `None` if `extension` is empty.
    pub fn set_extension<S: AsRef<OsStr>>(&mut self, extension: S) -> bool {
        self._set_extension(extension.as_ref())
    }

    fn _set_extension(&mut self, extension: &OsStr) -> bool {
        validate_extension(extension);
        let end_file_stem = match self.file_stem() {
            None => return false,
            Some(f) => {
                let b = f.as_encoded_bytes();
                (b.as_ptr() as usize) + b.len() - (self.inner.as_vec().as_ptr() as usize)
            }
        };
        self.as_mut_vec().truncate(end_file_stem);
        let new = extension.as_encoded_bytes();
        if !new.is_empty() {
            let v = self.as_mut_vec();
            v.reserve_exact(new.len() + 1);
            v.push(b'.');
            v.extend_from_slice(new);
        }
        true
    }

    /// Append `self.extension()` with `extension`.
    pub fn add_extension<S: AsRef<OsStr>>(&mut self, extension: S) -> bool {
        let extension = extension.as_ref();
        validate_extension(extension);
        let end_file_name = match self.file_name() {
            None => return false,
            Some(f) => {
                let b = f.as_encoded_bytes();
                (b.as_ptr() as usize) + b.len() - (self.inner.as_vec().as_ptr() as usize)
            }
        };
        let new = extension.as_encoded_bytes();
        if !new.is_empty() {
            let v = self.as_mut_vec();
            v.truncate(end_file_name);
            v.reserve_exact(new.len() + 1);
            v.push(b'.');
            v.extend_from_slice(new);
        }
        true
    }

    /// Yields a mutable reference to the underlying `OsString` instance.
    #[inline]
    pub fn as_mut_os_string(&mut self) -> &mut OsString {
        &mut self.inner
    }

    /// Consumes the `PathBuf`, yielding its internal `OsString` storage.
    #[inline]
    pub fn into_os_string(self) -> OsString {
        self.inner
    }

    /// Converts this `PathBuf` into a boxed `Path`.
    pub fn into_boxed_path(self) -> Box<Path> {
        let b = self.inner.into_boxed_os_str();
        unsafe { Box::from_raw(Box::into_raw(b) as *mut Path) }
    }

    /// Invokes `capacity` on the underlying instance of `OsString`.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Invokes `clear` on the underlying instance of `OsString`.
    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear()
    }

    /// Invokes `reserve` on the underlying instance of `OsString`.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional)
    }

    /// Invokes `reserve_exact` on the underlying instance of `OsString`.
    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        self.inner.reserve_exact(additional)
    }

    /// Invokes `shrink_to_fit` on the underlying instance of `OsString`.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit()
    }

    /// Shrinks the capacity with a lower bound.
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.inner.shrink_to(min_capacity)
    }

    /// Tries to reserve capacity for at least `additional` more elements.
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), crate::collections::TryReserveError> {
        self.inner.try_reserve(additional)
    }

    /// Tries to reserve the minimum capacity for at least `additional` more elements.
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), crate::collections::TryReserveError> {
        self.inner.try_reserve_exact(additional)
    }
}

fn validate_extension(extension: &OsStr) {
    for &b in extension.as_encoded_bytes() {
        if is_sep_byte(b) {
            panic!("extension cannot contain path separators: {:?}", extension);
        }
    }
}

impl Clone for PathBuf {
    #[inline]
    fn clone(&self) -> Self {
        PathBuf { inner: self.inner.clone() }
    }
}

impl Default for PathBuf {
    #[inline]
    fn default() -> Self {
        PathBuf::new()
    }
}

impl fmt::Debug for PathBuf {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, formatter)
    }
}

impl Deref for PathBuf {
    type Target = Path;
    #[inline]
    fn deref(&self) -> &Path {
        Path::from_bytes(self.inner.as_vec())
    }
}

impl DerefMut for PathBuf {
    #[inline]
    fn deref_mut(&mut self) -> &mut Path {
        Path::from_bytes_mut(self.inner.as_mut_vec())
    }
}

impl Borrow<Path> for PathBuf {
    #[inline]
    fn borrow(&self) -> &Path {
        self.deref()
    }
}

impl<T: ?Sized + AsRef<OsStr>> From<&T> for PathBuf {
    #[inline]
    fn from(s: &T) -> PathBuf {
        PathBuf::from(s.as_ref().to_os_string())
    }
}

impl From<OsString> for PathBuf {
    #[inline]
    fn from(s: OsString) -> PathBuf {
        PathBuf { inner: s }
    }
}

impl From<PathBuf> for OsString {
    #[inline]
    fn from(path_buf: PathBuf) -> OsString {
        path_buf.inner
    }
}

impl From<String> for PathBuf {
    #[inline]
    fn from(s: String) -> PathBuf {
        PathBuf::from(OsString::from(s))
    }
}

impl From<Box<Path>> for PathBuf {
    fn from(boxed: Box<Path>) -> PathBuf {
        boxed.to_path_buf()
    }
}

impl From<PathBuf> for Box<Path> {
    fn from(p: PathBuf) -> Box<Path> {
        p.into_boxed_path()
    }
}

impl From<&Path> for Box<Path> {
    fn from(path: &Path) -> Box<Path> {
        path.to_path_buf().into_boxed_path()
    }
}

impl Clone for Box<Path> {
    #[inline]
    fn clone(&self) -> Self {
        self.to_path_buf().into_boxed_path()
    }
}

impl<'a> From<Cow<'a, Path>> for PathBuf {
    #[inline]
    fn from(p: Cow<'a, Path>) -> Self {
        p.into_owned()
    }
}

impl<'a> From<&'a Path> for Cow<'a, Path> {
    #[inline]
    fn from(s: &'a Path) -> Cow<'a, Path> {
        Cow::Borrowed(s)
    }
}

impl<'a> From<PathBuf> for Cow<'a, Path> {
    #[inline]
    fn from(s: PathBuf) -> Cow<'a, Path> {
        Cow::Owned(s)
    }
}

impl<'a> From<&'a PathBuf> for Cow<'a, Path> {
    #[inline]
    fn from(p: &'a PathBuf) -> Cow<'a, Path> {
        Cow::Borrowed(p.as_path())
    }
}

impl From<PathBuf> for crate::sync::Arc<Path> {
    fn from(s: PathBuf) -> crate::sync::Arc<Path> {
        let arc: crate::sync::Arc<[u8]> = crate::sync::Arc::from(s.inner.into_encoded_bytes());
        unsafe { crate::mem::transmute::<crate::sync::Arc<[u8]>, crate::sync::Arc<Path>>(arc) }
    }
}

impl From<&Path> for crate::sync::Arc<Path> {
    fn from(s: &Path) -> crate::sync::Arc<Path> {
        crate::sync::Arc::from(s.to_path_buf())
    }
}

impl From<PathBuf> for crate::rc::Rc<Path> {
    fn from(s: PathBuf) -> crate::rc::Rc<Path> {
        let rc: crate::rc::Rc<[u8]> = crate::rc::Rc::from(s.inner.into_encoded_bytes());
        unsafe { crate::mem::transmute::<crate::rc::Rc<[u8]>, crate::rc::Rc<Path>>(rc) }
    }
}

impl From<&Path> for crate::rc::Rc<Path> {
    fn from(s: &Path) -> crate::rc::Rc<Path> {
        crate::rc::Rc::from(s.to_path_buf())
    }
}

impl FromStr for PathBuf {
    type Err = crate::convert::Infallible;
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(PathBuf::from(s))
    }
}

impl<P: AsRef<Path>> crate::iter::FromIterator<P> for PathBuf {
    fn from_iter<I: IntoIterator<Item = P>>(iter: I) -> PathBuf {
        let mut buf = PathBuf::new();
        for p in iter {
            buf.push(p.as_ref());
        }
        buf
    }
}

impl<P: AsRef<Path>> Extend<P> for PathBuf {
    fn extend<I: IntoIterator<Item = P>>(&mut self, iter: I) {
        for p in iter {
            self.push(p.as_ref());
        }
    }
}

impl PartialEq for PathBuf {
    #[inline]
    fn eq(&self, other: &PathBuf) -> bool {
        self.components() == other.components()
    }
}

impl Eq for PathBuf {}

impl Hash for PathBuf {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.as_path().hash(h)
    }
}

impl PartialOrd for PathBuf {
    #[inline]
    fn partial_cmp(&self, other: &PathBuf) -> Option<Ordering> {
        Some(compare_components(self.components(), other.components()))
    }
}

impl Ord for PathBuf {
    #[inline]
    fn cmp(&self, other: &PathBuf) -> Ordering {
        compare_components(self.components(), other.components())
    }
}

impl AsRef<OsStr> for PathBuf {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        self.inner.as_os_str()
    }
}

impl ToOwned for Path {
    type Owned = PathBuf;
    #[inline]
    fn to_owned(&self) -> PathBuf {
        self.to_path_buf()
    }
}

// ---------------------------------------------------------------------------
// Path

/// A slice of a path (akin to `str`).
pub struct Path {
    _z: [u8; 0],
    inner: [u8],
}

/// An error returned from `Path::strip_prefix` if the prefix was not found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StripPrefixError(());

impl Path {
    #[inline]
    pub(crate) fn from_bytes(s: &[u8]) -> &Path {
        unsafe { &*(s as *const [u8] as *const Path) }
    }

    #[inline]
    fn from_bytes_mut(s: &mut [u8]) -> &mut Path {
        unsafe { &mut *(s as *mut [u8] as *mut Path) }
    }

    #[inline]
    pub(crate) fn as_u8_slice(&self) -> &[u8] {
        &self.inner
    }

    /// Directly wraps a string slice as a `Path` slice.
    #[inline]
    pub fn new<S: AsRef<OsStr> + ?Sized>(s: &S) -> &Path {
        Path::from_bytes(s.as_ref().as_encoded_bytes())
    }

    /// Yields the underlying `OsStr` slice.
    #[inline]
    pub fn as_os_str(&self) -> &OsStr {
        os(&self.inner)
    }

    /// Yields a mutable reference to the underlying `OsStr` slice.
    #[inline]
    pub fn as_mut_os_str(&mut self) -> &mut OsStr {
        unsafe { &mut *(&mut self.inner as *mut [u8] as *mut OsStr) }
    }

    /// Yields a `&str` slice if the `Path` is valid unicode.
    #[inline]
    pub fn to_str(&self) -> Option<&str> {
        self.as_os_str().to_str()
    }

    /// Converts a `Path` to a `Cow<str>`.
    #[inline]
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        self.as_os_str().to_string_lossy()
    }

    /// Converts a `Path` to an owned `PathBuf`.
    pub fn to_path_buf(&self) -> PathBuf {
        PathBuf { inner: OsString::from_vec(self.inner.to_vec()) }
    }

    /// Returns `true` if the `Path` is absolute, i.e., if it is independent of the current directory.
    pub fn is_absolute(&self) -> bool {
        self.has_root()
    }

    /// Returns `true` if the `Path` is relative, i.e., not absolute.
    #[inline]
    pub fn is_relative(&self) -> bool {
        !self.is_absolute()
    }

    /// Returns `true` if the `Path` has a root.
    #[inline]
    pub fn has_root(&self) -> bool {
        !self.inner.is_empty() && is_sep_byte(self.inner[0])
    }

    /// Returns the `Path` without its final component, if there is one.
    pub fn parent(&self) -> Option<&Path> {
        let mut comps = self.components();
        let comp = comps.next_back();
        match comp {
            Some(Component::Normal(_)) | Some(Component::CurDir) | Some(Component::ParentDir) => Some(comps.as_path()),
            _ => None,
        }
    }

    /// Produces an iterator over `Path` and its ancestors.
    #[inline]
    pub fn ancestors(&self) -> Ancestors<'_> {
        Ancestors { next: Some(self) }
    }

    /// Returns the final component of the `Path`, if there is one.
    pub fn file_name(&self) -> Option<&OsStr> {
        match self.components().next_back() {
            Some(Component::Normal(p)) => Some(p),
            _ => None,
        }
    }

    /// Returns a path that, when joined onto `base`, yields `self`.
    pub fn strip_prefix<P>(&self, base: P) -> Result<&Path, StripPrefixError>
    where
        P: AsRef<Path>,
    {
        self._strip_prefix(base.as_ref())
    }

    fn _strip_prefix(&self, base: &Path) -> Result<&Path, StripPrefixError> {
        match iter_after(self.components(), base.components()) {
            Some(c) => Ok(c.as_path()),
            None => Err(StripPrefixError(())),
        }
    }

    /// Determines whether `base` is a prefix of `self`.
    pub fn starts_with<P: AsRef<Path>>(&self, base: P) -> bool {
        iter_after(self.components(), base.as_ref().components()).is_some()
    }

    /// Determines whether `child` is a suffix of `self`.
    pub fn ends_with<P: AsRef<Path>>(&self, child: P) -> bool {
        iter_after(self.components().rev(), child.as_ref().components().rev()).is_some()
    }

    /// Extracts the stem (non-extension) portion of `self.file_name`.
    pub fn file_stem(&self) -> Option<&OsStr> {
        match self.file_name() {
            Some(name) => {
                let (before, after) = rsplit_file_at_dot(name);
                before.or(after)
            }
            None => None,
        }
    }

    /// Extracts the prefix of `self.file_name` (the portion before the first dot).
    pub fn file_prefix(&self) -> Option<&OsStr> {
        match self.file_name() {
            Some(name) => Some(split_file_at_dot(name).0),
            None => None,
        }
    }

    /// Extracts the extension (without the leading dot) of `self.file_name`, if possible.
    pub fn extension(&self) -> Option<&OsStr> {
        match self.file_name() {
            Some(name) => {
                let (before, after) = rsplit_file_at_dot(name);
                before.and(after)
            }
            None => None,
        }
    }

    /// Creates an owned `PathBuf` with `path` adjoined to `self`.
    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        let mut buf = self.to_path_buf();
        buf.push(path);
        buf
    }

    /// Creates an owned `PathBuf` like `self` but with the given file name.
    pub fn with_file_name<S: AsRef<OsStr>>(&self, file_name: S) -> PathBuf {
        let mut buf = self.to_path_buf();
        buf.set_file_name(file_name);
        buf
    }

    /// Creates an owned `PathBuf` like `self` but with the given extension.
    pub fn with_extension<S: AsRef<OsStr>>(&self, extension: S) -> PathBuf {
        let mut buf = self.to_path_buf();
        buf.set_extension(extension);
        buf
    }

    /// Creates an owned `PathBuf` like `self` but with the extension added.
    pub fn with_added_extension<S: AsRef<OsStr>>(&self, extension: S) -> PathBuf {
        let mut buf = self.to_path_buf();
        buf.add_extension(extension);
        buf
    }

    /// Produces an iterator over the `Component`s of the path.
    pub fn components(&self) -> Components<'_> {
        Components { path: &self.inner, has_physical_root: self.has_root(), front: State::Prefix, back: State::Body }
    }

    /// Produces an iterator over the path's components viewed as `OsStr` slices.
    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter { inner: self.components() }
    }

    /// Returns an object that implements `Display` for safely printing paths.
    #[inline]
    pub fn display(&self) -> Display<'_> {
        Display { path: self }
    }

    /// Queries the file system to get information about a file, directory, etc.
    #[inline]
    pub fn metadata(&self) -> io::Result<fs::Metadata> {
        fs::metadata(self)
    }

    /// Queries the metadata about a file without following symlinks.
    #[inline]
    pub fn symlink_metadata(&self) -> io::Result<fs::Metadata> {
        fs::symlink_metadata(self)
    }

    /// Returns the canonical, absolute form of the path with all intermediate
    /// components normalized and symbolic links resolved.
    #[inline]
    pub fn canonicalize(&self) -> io::Result<PathBuf> {
        fs::canonicalize(self)
    }

    /// Reads a symbolic link, returning the file that the link points to.
    #[inline]
    pub fn read_link(&self) -> io::Result<PathBuf> {
        fs::read_link(self)
    }

    /// Returns an iterator over the entries within a directory.
    #[inline]
    pub fn read_dir(&self) -> io::Result<fs::ReadDir> {
        fs::read_dir(self)
    }

    /// Returns `true` if the path points at an existing entity.
    pub fn exists(&self) -> bool {
        fs::metadata(self).is_ok()
    }

    /// Returns `Ok(true)` if the path points at an existing entity.
    pub fn try_exists(&self) -> io::Result<bool> {
        fs::exists(self)
    }

    /// Returns `true` if the path exists on disk and is pointing at a regular file.
    pub fn is_file(&self) -> bool {
        match fs::metadata(self) {
            Ok(m) => m.is_file(),
            Err(_) => false,
        }
    }

    /// Returns `true` if the path exists on disk and is pointing at a directory.
    pub fn is_dir(&self) -> bool {
        match fs::metadata(self) {
            Ok(m) => m.is_dir(),
            Err(_) => false,
        }
    }

    /// Returns `true` if the path exists on disk and is pointing at a symbolic link.
    pub fn is_symlink(&self) -> bool {
        match fs::symlink_metadata(self) {
            Ok(m) => m.is_symlink(),
            Err(_) => false,
        }
    }

    /// Converts a `Box<Path>` into a `PathBuf` without copying or allocating.
    pub fn into_path_buf(self: Box<Path>) -> PathBuf {
        self.to_path_buf()
    }
}

impl AsRef<OsStr> for Path {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl fmt::Debug for Path {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_os_str(), formatter)
    }
}

/// Helper struct for safely printing paths with `format!` and `{}`.
pub struct Display<'a> {
    path: &'a Path,
}

impl<'a> fmt::Debug for Display<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.path, f)
    }
}

impl<'a> fmt::Display for Display<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crate::ffi::os_str::display_os_bytes(&self.path.inner, f)
    }
}

impl PartialEq for Path {
    #[inline]
    fn eq(&self, other: &Path) -> bool {
        self.components() == other.components()
    }
}

impl Eq for Path {}

impl Hash for Path {
    fn hash<H: Hasher>(&self, h: &mut H) {
        let bytes = &self.inner;
        let mut component_start = 0;
        // like std: extra state so that ["foo", "bar"] and ["foobar"] differ
        let mut chunk_bits: usize = 0;
        let mut i = 0;
        while i < bytes.len() {
            if is_sep_byte(bytes[i]) {
                if i > component_start {
                    let to_hash = &bytes[component_start..i];
                    chunk_bits = chunk_bits.wrapping_add(to_hash.len());
                    chunk_bits = chunk_bits.rotate_right(2);
                    h.write(to_hash);
                }
                // skip over separator and optionally a following CurDir item
                // since components() would normalize these away.
                component_start = i + 1;
                let tail = &bytes[component_start..];
                if tail.len() == 1 && tail[0] == b'.' {
                    component_start += 1;
                } else if tail.len() >= 2 && tail[0] == b'.' && is_sep_byte(tail[1]) {
                    component_start += 1;
                }
            }
            i += 1;
        }
        if component_start < bytes.len() {
            let to_hash = &bytes[component_start..];
            chunk_bits = chunk_bits.wrapping_add(to_hash.len());
            chunk_bits = chunk_bits.rotate_right(2);
            h.write(to_hash);
        }
        h.write_usize(chunk_bits);
    }
}

impl PartialOrd for Path {
    #[inline]
    fn partial_cmp(&self, other: &Path) -> Option<Ordering> {
        Some(compare_components(self.components(), other.components()))
    }
}

impl Ord for Path {
    #[inline]
    fn cmp(&self, other: &Path) -> Ordering {
        compare_components(self.components(), other.components())
    }
}

impl AsRef<Path> for Path {
    #[inline]
    fn as_ref(&self) -> &Path {
        self
    }
}

impl AsRef<Path> for OsStr {
    #[inline]
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

impl<'a> AsRef<Path> for Cow<'a, OsStr> {
    #[inline]
    fn as_ref(&self) -> &Path {
        Path::new(&**self)
    }
}

impl AsRef<Path> for OsString {
    #[inline]
    fn as_ref(&self) -> &Path {
        Path::new(self.as_os_str())
    }
}

impl AsRef<Path> for str {
    #[inline]
    fn as_ref(&self) -> &Path {
        Path::from_bytes(self.as_bytes())
    }
}

impl AsRef<Path> for String {
    #[inline]
    fn as_ref(&self) -> &Path {
        Path::from_bytes(self.as_bytes())
    }
}

impl AsRef<Path> for PathBuf {
    #[inline]
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl<'a> IntoIterator for &'a PathBuf {
    type Item = &'a OsStr;
    type IntoIter = Iter<'a>;
    #[inline]
    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a Path {
    type Item = &'a OsStr;
    type IntoIter = Iter<'a>;
    #[inline]
    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}

// Cross-type comparisons.
macro_rules! impl_cmp {
    ($lhs:ty, $rhs:ty) => {
        impl<'a, 'b> PartialEq<$rhs> for $lhs {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                <Path as PartialEq>::eq(self.as_ref(), other.as_ref())
            }
        }

        impl<'a, 'b> PartialEq<$lhs> for $rhs {
            #[inline]
            fn eq(&self, other: &$lhs) -> bool {
                <Path as PartialEq>::eq(self.as_ref(), other.as_ref())
            }
        }

        impl<'a, 'b> PartialOrd<$rhs> for $lhs {
            #[inline]
            fn partial_cmp(&self, other: &$rhs) -> Option<Ordering> {
                <Path as PartialOrd>::partial_cmp(self.as_ref(), other.as_ref())
            }
        }

        impl<'a, 'b> PartialOrd<$lhs> for $rhs {
            #[inline]
            fn partial_cmp(&self, other: &$lhs) -> Option<Ordering> {
                <Path as PartialOrd>::partial_cmp(self.as_ref(), other.as_ref())
            }
        }
    };
}

impl_cmp!(PathBuf, Path);
impl_cmp!(PathBuf, &'a Path);
impl_cmp!(Cow<'a, Path>, Path);
impl_cmp!(Cow<'a, Path>, &'b Path);
impl_cmp!(Cow<'a, Path>, PathBuf);

macro_rules! impl_cmp_os_str {
    ($lhs:ty, $rhs:ty) => {
        impl<'a, 'b> PartialEq<$rhs> for $lhs {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                <Path as PartialEq>::eq(self.as_ref(), Path::new(other))
            }
        }

        impl<'a, 'b> PartialEq<$lhs> for $rhs {
            #[inline]
            fn eq(&self, other: &$lhs) -> bool {
                <Path as PartialEq>::eq(Path::new(self), other.as_ref())
            }
        }

        impl<'a, 'b> PartialOrd<$rhs> for $lhs {
            #[inline]
            fn partial_cmp(&self, other: &$rhs) -> Option<Ordering> {
                <Path as PartialOrd>::partial_cmp(self.as_ref(), Path::new(other))
            }
        }

        impl<'a, 'b> PartialOrd<$lhs> for $rhs {
            #[inline]
            fn partial_cmp(&self, other: &$lhs) -> Option<Ordering> {
                <Path as PartialOrd>::partial_cmp(Path::new(self), other.as_ref())
            }
        }
    };
}

impl_cmp_os_str!(PathBuf, OsStr);
impl_cmp_os_str!(PathBuf, &'a OsStr);
impl_cmp_os_str!(PathBuf, OsString);
impl_cmp_os_str!(Path, OsStr);
impl_cmp_os_str!(Path, &'a OsStr);
impl_cmp_os_str!(Path, OsString);
impl_cmp_os_str!(&'a Path, OsStr);
impl_cmp_os_str!(&'a Path, OsString);

impl fmt::Display for StripPrefixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("prefix not found")
    }
}

impl Error for StripPrefixError {
    #[allow(deprecated)]
    fn description(&self) -> &str {
        "prefix not found"
    }
}

/// Makes the path absolute without accessing the filesystem.
pub fn absolute<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    let path = path.as_ref();
    if path.as_os_str().is_empty() {
        return Err(io::const_io_error(io::ErrorKind::InvalidInput, "cannot make an empty path absolute"));
    }
    let mut components = path.strip_prefix(".").unwrap_or(path).components();
    let path_os = path.as_os_str().as_encoded_bytes();

    let mut normalized = if path.is_absolute() {
        // "If a pathname begins with two successive <slash> characters, the
        // first component following the leading <slash> characters may be
        // interpreted in an implementation-defined manner, although more than
        // two leading <slash> characters shall be treated as a single
        // <slash> character."
        if path_os.starts_with(b"//") && !path_os.starts_with(b"///") {
            components.next();
            PathBuf::from("//")
        } else {
            PathBuf::new()
        }
    } else {
        crate::env::current_dir()?
    };
    normalized.extend(components);

    // "Interfaces using pathname resolution may specify additional constraints
    // when a pathname that does not name an existing directory contains at
    // least one non- <slash> character and contains one or more trailing
    // <slash> characters".
    // A trailing <slash> is also meaningful if "a symbolic link is
    // encountered during pathname resolution".
    if path_os.ends_with(b"/") {
        normalized.push("");
    }

    Ok(normalized)
}
