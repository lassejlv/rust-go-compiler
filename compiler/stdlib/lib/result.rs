//! Error handling with the `Result` type.

use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::iter::{DoubleEndedIterator, ExactSizeIterator, FromIterator, FusedIterator, IntoIterator, Iterator};
use crate::ops::{Deref, DerefMut};

/// `Result` is a type that represents either success (`Ok`) or failure (`Err`).
#[lang = "Result"]
#[must_use]
pub enum Result<T, E> {
    /// Contains the success value.
    #[lang = "Ok"]
    Ok(T),
    /// Contains the error value.
    #[lang = "Err"]
    Err(E),
}

pub use self::Result::{Err, Ok};

#[track_caller]
#[inline(never)]
fn unwrap_failed<E: fmt::Debug>(msg: &str, error: &E) -> ! {
    crate::rt::panic_fmt(format_args!("{msg}: {error:?}"), file!(), line!(), column!())
}

impl<T, E> Result<T, E> {
    /// Returns `true` if the result is `Ok`.
    #[inline]
    pub const fn is_ok(&self) -> bool {
        match *self {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Returns `true` if the result is `Ok` and the value inside of it matches a predicate.
    #[inline]
    pub fn is_ok_and<F: FnOnce(T) -> bool>(self, f: F) -> bool {
        match self {
            Err(_) => false,
            Ok(x) => f(x),
        }
    }

    /// Returns `true` if the result is `Err`.
    #[inline]
    pub const fn is_err(&self) -> bool {
        !self.is_ok()
    }

    /// Returns `true` if the result is `Err` and the value inside of it matches a predicate.
    #[inline]
    pub fn is_err_and<F: FnOnce(E) -> bool>(self, f: F) -> bool {
        match self {
            Ok(_) => false,
            Err(e) => f(e),
        }
    }

    /// Converts from `Result<T, E>` to `Option<T>`.
    #[inline]
    pub fn ok(self) -> Option<T> {
        match self {
            Ok(x) => Some(x),
            Err(_) => None,
        }
    }

    /// Converts from `Result<T, E>` to `Option<E>`.
    #[inline]
    pub fn err(self) -> Option<E> {
        match self {
            Ok(_) => None,
            Err(x) => Some(x),
        }
    }

    /// Converts from `&Result<T, E>` to `Result<&T, &E>`.
    #[inline]
    pub const fn as_ref(&self) -> Result<&T, &E> {
        match *self {
            Ok(ref x) => Ok(x),
            Err(ref x) => Err(x),
        }
    }

    /// Converts from `&mut Result<T, E>` to `Result<&mut T, &mut E>`.
    #[inline]
    pub fn as_mut(&mut self) -> Result<&mut T, &mut E> {
        match *self {
            Ok(ref mut x) => Ok(x),
            Err(ref mut x) => Err(x),
        }
    }

    /// Maps a `Result<T, E>` to `Result<U, E>` by applying a function to a contained `Ok` value.
    #[inline]
    pub fn map<U, F: FnOnce(T) -> U>(self, op: F) -> Result<U, E> {
        match self {
            Ok(t) => Ok(op(t)),
            Err(e) => Err(e),
        }
    }

    /// Maps the `Ok` value with `f`, or returns the default value of `U` if `Err`.
    #[inline]
    pub fn map_or_default<U: Default, F: FnOnce(T) -> U>(self, f: F) -> U {
        match self {
            Ok(t) => f(t),
            Err(_) => U::default(),
        }
    }

    /// Returns the provided default (if `Err`), or applies a function to the contained value (if `Ok`).
    #[inline]
    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, f: F) -> U {
        match self {
            Ok(t) => f(t),
            Err(_) => default,
        }
    }

    /// Maps a `Result<T, E>` to `U` by applying fallback function `default` to a contained `Err` value, or function `f` to a contained `Ok` value.
    #[inline]
    pub fn map_or_else<U, D: FnOnce(E) -> U, F: FnOnce(T) -> U>(self, default: D, f: F) -> U {
        match self {
            Ok(t) => f(t),
            Err(e) => default(e),
        }
    }

    /// Maps a `Result<T, E>` to `Result<T, F>` by applying a function to a contained `Err` value.
    #[inline]
    pub fn map_err<F, O: FnOnce(E) -> F>(self, op: O) -> Result<T, F> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(op(e)),
        }
    }

    /// Calls a function with a reference to the contained value if `Ok`.
    #[inline]
    pub fn inspect<F: FnOnce(&T)>(self, f: F) -> Self {
        if let Ok(ref t) = self {
            f(t);
        }
        self
    }

    /// Calls a function with a reference to the contained value if `Err`.
    #[inline]
    pub fn inspect_err<F: FnOnce(&E)>(self, f: F) -> Self {
        if let Err(ref e) = self {
            f(e);
        }
        self
    }

    /// Converts from `Result<T, E>` (or `&Result<T, E>`) to `Result<&<T as Deref>::Target, &E>`.
    #[inline]
    pub fn as_deref(&self) -> Result<&T::Target, &E>
    where
        T: Deref,
    {
        match self.as_ref() {
            Ok(t) => Ok(t.deref()),
            Err(e) => Err(e),
        }
    }

    /// Converts from `Result<T, E>` (or `&mut Result<T, E>`) to `Result<&mut <T as DerefMut>::Target, &mut E>`.
    #[inline]
    pub fn as_deref_mut(&mut self) -> Result<&mut T::Target, &mut E>
    where
        T: DerefMut,
    {
        match self.as_mut() {
            Ok(t) => Ok(t.deref_mut()),
            Err(e) => Err(e),
        }
    }

    /// Returns an iterator over the possibly contained value.
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter { inner: self.as_ref().ok() }
    }

    /// Returns a mutable iterator over the possibly contained value.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut { inner: self.as_mut().ok() }
    }

    /// Returns the contained `Ok` value, panicking with `msg` if the value is an `Err`.
    #[inline]
    #[track_caller]
    pub fn expect(self, msg: &str) -> T
    where
        E: fmt::Debug,
    {
        match self {
            Ok(t) => t,
            Err(e) => unwrap_failed(msg, &e),
        }
    }

    /// Returns the contained `Ok` value, panicking if the value is an `Err`.
    #[inline]
    #[track_caller]
    pub fn unwrap(self) -> T
    where
        E: fmt::Debug,
    {
        match self {
            Ok(t) => t,
            Err(e) => unwrap_failed("called `Result::unwrap()` on an `Err` value", &e),
        }
    }

    /// Returns the contained `Ok` value or a default.
    #[inline]
    pub fn unwrap_or_default(self) -> T
    where
        T: Default,
    {
        match self {
            Ok(x) => x,
            Err(_) => Default::default(),
        }
    }

    /// Returns the contained `Err` value, panicking with `msg` if the value is an `Ok`.
    #[inline]
    #[track_caller]
    pub fn expect_err(self, msg: &str) -> E
    where
        T: fmt::Debug,
    {
        match self {
            Ok(t) => unwrap_failed(msg, &t),
            Err(e) => e,
        }
    }

    /// Returns the contained `Err` value, panicking if the value is an `Ok`.
    #[inline]
    #[track_caller]
    pub fn unwrap_err(self) -> E
    where
        T: fmt::Debug,
    {
        match self {
            Ok(t) => unwrap_failed("called `Result::unwrap_err()` on an `Ok` value", &t),
            Err(e) => e,
        }
    }

    /// Returns `res` if the result is `Ok`, otherwise returns the `Err` value of `self`.
    #[inline]
    pub fn and<U>(self, res: Result<U, E>) -> Result<U, E> {
        match self {
            Ok(_) => res,
            Err(e) => Err(e),
        }
    }

    /// Calls `op` if the result is `Ok`, otherwise returns the `Err` value of `self`.
    #[inline]
    pub fn and_then<U, F: FnOnce(T) -> Result<U, E>>(self, op: F) -> Result<U, E> {
        match self {
            Ok(t) => op(t),
            Err(e) => Err(e),
        }
    }

    /// Returns `res` if the result is `Err`, otherwise returns the `Ok` value of `self`.
    #[inline]
    pub fn or<F>(self, res: Result<T, F>) -> Result<T, F> {
        match self {
            Ok(v) => Ok(v),
            Err(_) => res,
        }
    }

    /// Calls `op` if the result is `Err`, otherwise returns the `Ok` value of `self`.
    #[inline]
    pub fn or_else<F, O: FnOnce(E) -> Result<T, F>>(self, op: O) -> Result<T, F> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => op(e),
        }
    }

    /// Returns the contained `Ok` value or a provided default.
    #[inline]
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Ok(t) => t,
            Err(_) => default,
        }
    }

    /// Returns the contained `Ok` value or computes it from a closure.
    #[inline]
    pub fn unwrap_or_else<F: FnOnce(E) -> T>(self, op: F) -> T {
        match self {
            Ok(t) => t,
            Err(e) => op(e),
        }
    }

    /// Returns the contained `Ok` value without checking.
    #[inline]
    #[track_caller]
    pub unsafe fn unwrap_unchecked(self) -> T {
        match self {
            Ok(t) => t,
            Err(_) => crate::rt::panic_str("called `Result::unwrap_unchecked()` on an `Err` value", file!(), line!(), column!()),
        }
    }
}

impl<T, E> Result<&T, E> {
    /// Maps a `Result<&T, E>` to a `Result<T, E>` by copying the contents of the `Ok` part.
    #[inline]
    pub fn copied(self) -> Result<T, E>
    where
        T: Copy,
    {
        match self {
            Ok(&v) => Ok(v),
            Err(e) => Err(e),
        }
    }

    /// Maps a `Result<&T, E>` to a `Result<T, E>` by cloning the contents of the `Ok` part.
    #[inline]
    pub fn cloned(self) -> Result<T, E>
    where
        T: Clone,
    {
        match self {
            Ok(t) => Ok(t.clone()),
            Err(e) => Err(e),
        }
    }
}

impl<T, E> Result<Option<T>, E> {
    /// Transposes a `Result` of an `Option` into an `Option` of a `Result`.
    #[inline]
    pub fn transpose(self) -> Option<Result<T, E>> {
        match self {
            Ok(Some(x)) => Some(Ok(x)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

impl<T, E> Result<Result<T, E>, E> {
    /// Converts from `Result<Result<T, E>, E>` to `Result<T, E>`.
    #[inline]
    pub fn flatten(self) -> Result<T, E> {
        match self {
            Ok(inner) => inner,
            Err(e) => Err(e),
        }
    }
}

impl<T: Clone, E: Clone> Clone for Result<T, E> {
    #[inline]
    fn clone(&self) -> Self {
        match self {
            Ok(x) => Ok(x.clone()),
            Err(x) => Err(x.clone()),
        }
    }
}

impl<T: Copy, E: Copy> Copy for Result<T, E> {}

impl<T: PartialEq, E: PartialEq> PartialEq for Result<T, E> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Ok(a), Ok(b)) => *a == *b,
            (Err(a), Err(b)) => *a == *b,
            _ => false,
        }
    }
}

impl<T: Eq, E: Eq> Eq for Result<T, E> {}

impl<T: PartialOrd, E: PartialOrd> PartialOrd for Result<T, E> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Ok(a), Ok(b)) => a.partial_cmp(b),
            (Err(a), Err(b)) => a.partial_cmp(b),
            (Ok(_), Err(_)) => Some(Ordering::Less),
            (Err(_), Ok(_)) => Some(Ordering::Greater),
        }
    }
}

impl<T: Ord, E: Ord> Ord for Result<T, E> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Ok(a), Ok(b)) => a.cmp(b),
            (Err(a), Err(b)) => a.cmp(b),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
        }
    }
}

impl<T: Hash, E: Hash> Hash for Result<T, E> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Ok(x) => {
                state.write_isize(0);
                x.hash(state);
            }
            Err(e) => {
                state.write_isize(1);
                e.hash(state);
            }
        }
    }
}

impl<T: fmt::Debug, E: fmt::Debug> fmt::Debug for Result<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ok(x) => f.debug_tuple("Ok").field(x).finish(),
            Err(e) => f.debug_tuple("Err").field(e).finish(),
        }
    }
}

// ---------------------------------------------------------------------------
// Iterators

/// An iterator over a reference to the `Ok` variant of a `Result`.
pub struct Iter<'a, T: 'a> {
    inner: Option<&'a T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    #[inline]
    fn next(&mut self) -> Option<&'a T> {
        self.inner.take()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = if self.inner.is_some() { 1 } else { 0 };
        (n, Some(n))
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a T> {
        self.inner.take()
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {}
impl<'a, T> FusedIterator for Iter<'a, T> {}

/// An iterator over a mutable reference to the `Ok` variant of a `Result`.
pub struct IterMut<'a, T: 'a> {
    inner: Option<&'a mut T>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    #[inline]
    fn next(&mut self) -> Option<&'a mut T> {
        self.inner.take()
    }
}

impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a mut T> {
        self.inner.take()
    }
}

/// An iterator over the value in an `Ok` variant of a `Result`.
pub struct IntoIter<T> {
    inner: Option<T>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        self.inner.take()
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        self.inner.take()
    }
}

impl<T, E> IntoIterator for Result<T, E> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    #[inline]
    fn into_iter(self) -> IntoIter<T> {
        IntoIter { inner: self.ok() }
    }
}

impl<'a, T, E> IntoIterator for &'a Result<T, E> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T, E> IntoIterator for &'a mut Result<T, E> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;
    fn into_iter(self) -> IterMut<'a, T> {
        self.iter_mut()
    }
}

impl<A, E, V: FromIterator<A>> FromIterator<Result<A, E>> for Result<V, E> {
    /// Takes each element in the iterator: if it is an `Err`, no further
    /// elements are taken, and the `Err` is returned.
    fn from_iter<I: IntoIterator<Item = Result<A, E>>>(iter: I) -> Result<V, E> {
        let mut error: Option<E> = None;
        let v: V = {
            let shunt = ResultShunt { iter: iter.into_iter(), error: &mut error };
            FromIterator::from_iter(shunt)
        };
        match error {
            Some(e) => Err(e),
            None => Ok(v),
        }
    }
}

struct ResultShunt<'a, I, E> {
    iter: I,
    error: &'a mut Option<E>,
}

impl<'a, A, E, I: Iterator<Item = Result<A, E>>> Iterator for ResultShunt<'a, I, E> {
    type Item = A;
    fn next(&mut self) -> Option<A> {
        if self.error.is_some() {
            return None;
        }
        match self.iter.next() {
            Some(Ok(x)) => Some(x),
            Some(Err(e)) => {
                *self.error = Some(e);
                None
            }
            None => None,
        }
    }
}
