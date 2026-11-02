//! Optional values.

use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::iter::{DoubleEndedIterator, ExactSizeIterator, FromIterator, FusedIterator, IntoIterator, Iterator};
use crate::ops::{Deref, DerefMut};

/// The `Option` type.
#[lang = "Option"]
pub enum Option<T> {
    /// No value.
    #[lang = "None"]
    None,
    /// Some value of type `T`.
    #[lang = "Some"]
    Some(T),
}

pub use self::Option::{None, Some};

#[track_caller]
#[inline(never)]
fn expect_failed(msg: &str) -> ! {
    crate::rt::panic_str(msg, file!(), line!(), column!())
}

impl<T> Option<T> {
    /// Returns `true` if the option is a `Some` value.
    #[inline]
    pub const fn is_some(&self) -> bool {
        match *self {
            Some(_) => true,
            None => false,
        }
    }

    /// Returns `true` if the option is a `Some` and the value inside of it matches a predicate.
    #[inline]
    pub fn is_some_and<F: FnOnce(T) -> bool>(self, f: F) -> bool {
        match self {
            None => false,
            Some(x) => f(x),
        }
    }

    /// Returns `true` if the option is a `None` value.
    #[inline]
    pub const fn is_none(&self) -> bool {
        !self.is_some()
    }

    /// Returns `true` if the option is a `None` or the value inside of it matches a predicate.
    #[inline]
    pub fn is_none_or<F: FnOnce(T) -> bool>(self, f: F) -> bool {
        match self {
            None => true,
            Some(x) => f(x),
        }
    }

    /// Converts from `&Option<T>` to `Option<&T>`.
    #[inline]
    pub const fn as_ref(&self) -> Option<&T> {
        match *self {
            Some(ref x) => Some(x),
            None => None,
        }
    }

    /// Converts from `&mut Option<T>` to `Option<&mut T>`.
    #[inline]
    pub fn as_mut(&mut self) -> Option<&mut T> {
        match *self {
            Some(ref mut x) => Some(x),
            None => None,
        }
    }

    /// Converts from `Pin<&Option<T>>` to `Option<Pin<&T>>`.
    #[inline]
    pub fn as_pin_ref(self: crate::pin::Pin<&Self>) -> Option<crate::pin::Pin<&T>> {
        match crate::pin::Pin::get_ref(self) {
            Some(x) => unsafe { Some(crate::pin::Pin::new_unchecked(x)) },
            None => None,
        }
    }

    /// Converts from `Pin<&mut Option<T>>` to `Option<Pin<&mut T>>`.
    #[inline]
    pub fn as_pin_mut(self: crate::pin::Pin<&mut Self>) -> Option<crate::pin::Pin<&mut T>> {
        unsafe {
            match crate::pin::Pin::get_unchecked_mut(self).as_mut() {
                Some(x) => Some(crate::pin::Pin::new_unchecked(x)),
                None => None,
            }
        }
    }

    /// Returns a mutable slice of the contained value, if any.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        match *self {
            Some(ref mut x) => crate::slice::from_mut(x),
            None => &mut [],
        }
    }

    /// Maps the contained value with `f`, or returns the default value of `U` if `None`.
    #[inline]
    pub fn map_or_default<U: Default, F: FnOnce(T) -> U>(self, f: F) -> U {
        match self {
            Some(t) => f(t),
            None => U::default(),
        }
    }

    /// Returns a slice of the contained value, if any.
    pub fn as_slice(&self) -> &[T] {
        match *self {
            Some(ref x) => crate::slice::from_ref(x),
            None => &[],
        }
    }

    /// Returns the contained `Some` value, panicking with `msg` if `None`.
    #[inline]
    #[track_caller]
    pub fn expect(self, msg: &str) -> T {
        match self {
            Some(val) => val,
            None => expect_failed(msg),
        }
    }

    /// Returns the contained `Some` value, panicking if `None`.
    #[inline]
    #[track_caller]
    pub fn unwrap(self) -> T {
        match self {
            Some(val) => val,
            None => crate::rt::panic_str("called `Option::unwrap()` on a `None` value", file!(), line!(), column!()),
        }
    }

    /// Returns the contained `Some` value or a provided default.
    #[inline]
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Some(x) => x,
            None => default,
        }
    }

    /// Returns the contained `Some` value or computes it from a closure.
    #[inline]
    pub fn unwrap_or_else<F: FnOnce() -> T>(self, f: F) -> T {
        match self {
            Some(x) => x,
            None => f(),
        }
    }

    /// Returns the contained `Some` value or a default.
    #[inline]
    pub fn unwrap_or_default(self) -> T
    where
        T: Default,
    {
        match self {
            Some(x) => x,
            None => T::default(),
        }
    }

    /// Returns the contained `Some` value without checking.
    #[inline]
    #[track_caller]
    pub unsafe fn unwrap_unchecked(self) -> T {
        match self {
            Some(val) => val,
            None => crate::rt::panic_str("called `Option::unwrap_unchecked()` on a `None` value", file!(), line!(), column!()),
        }
    }

    /// Maps an `Option<T>` to `Option<U>` by applying a function to a contained value.
    #[inline]
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Option<U> {
        match self {
            Some(x) => Some(f(x)),
            None => None,
        }
    }

    /// Calls a function with a reference to the contained value if `Some`.
    #[inline]
    pub fn inspect<F: FnOnce(&T)>(self, f: F) -> Self {
        if let Some(ref x) = self {
            f(x);
        }
        self
    }

    /// Returns the provided default result (if none), or applies a function to the contained value.
    #[inline]
    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, f: F) -> U {
        match self {
            Some(t) => f(t),
            None => default,
        }
    }

    /// Computes a default function result (if none), or applies a different function to the contained value.
    #[inline]
    pub fn map_or_else<U, D: FnOnce() -> U, F: FnOnce(T) -> U>(self, default: D, f: F) -> U {
        match self {
            Some(t) => f(t),
            None => default(),
        }
    }

    /// Transforms the `Option<T>` into a `Result<T, E>`.
    #[inline]
    pub fn ok_or<E>(self, err: E) -> Result<T, E> {
        match self {
            Some(v) => Ok(v),
            None => Err(err),
        }
    }

    /// Transforms the `Option<T>` into a `Result<T, E>`, mapping `None` to `Err(err())`.
    #[inline]
    pub fn ok_or_else<E, F: FnOnce() -> E>(self, err: F) -> Result<T, E> {
        match self {
            Some(v) => Ok(v),
            None => Err(err()),
        }
    }

    /// Converts from `Option<T>` (or `&Option<T>`) to `Option<&T::Target>`.
    #[inline]
    pub fn as_deref(&self) -> Option<&T::Target>
    where
        T: Deref,
    {
        match self.as_ref() {
            Some(t) => Some(t.deref()),
            None => None,
        }
    }

    /// Converts from `Option<T>` (or `&mut Option<T>`) to `Option<&mut T::Target>`.
    #[inline]
    pub fn as_deref_mut(&mut self) -> Option<&mut T::Target>
    where
        T: DerefMut,
    {
        match self.as_mut() {
            Some(t) => Some(t.deref_mut()),
            None => None,
        }
    }

    /// Returns an iterator over the possibly contained value.
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter { inner: self.as_ref() }
    }

    /// Returns a mutable iterator over the possibly contained value.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut { inner: self.as_mut() }
    }

    /// Returns `None` if the option is `None`, otherwise returns `optb`.
    #[inline]
    pub fn and<U>(self, optb: Option<U>) -> Option<U> {
        match self {
            Some(_) => optb,
            None => None,
        }
    }

    /// Returns `None` if the option is `None`, otherwise calls `f` with the wrapped value and returns the result.
    #[inline]
    pub fn and_then<U, F: FnOnce(T) -> Option<U>>(self, f: F) -> Option<U> {
        match self {
            Some(x) => f(x),
            None => None,
        }
    }

    /// Returns `None` if the option is `None`, otherwise calls `predicate` with the wrapped value.
    #[inline]
    pub fn filter<P: FnOnce(&T) -> bool>(self, predicate: P) -> Self {
        if let Some(x) = self {
            if predicate(&x) {
                return Some(x);
            }
        }
        None
    }

    /// Returns the option if it contains a value, otherwise returns `optb`.
    #[inline]
    pub fn or(self, optb: Option<T>) -> Option<T> {
        match self {
            x @ Some(_) => x,
            None => optb,
        }
    }

    /// Returns the option if it contains a value, otherwise calls `f` and returns the result.
    #[inline]
    pub fn or_else<F: FnOnce() -> Option<T>>(self, f: F) -> Option<T> {
        match self {
            x @ Some(_) => x,
            None => f(),
        }
    }

    /// Returns `Some` if exactly one of `self`, `optb` is `Some`, otherwise returns `None`.
    #[inline]
    pub fn xor(self, optb: Option<T>) -> Option<T> {
        match (self, optb) {
            (a @ Some(_), None) => a,
            (None, b @ Some(_)) => b,
            _ => None,
        }
    }

    /// Inserts `value` into the option, then returns a mutable reference to it.
    #[inline]
    pub fn insert(&mut self, value: T) -> &mut T {
        *self = Some(value);
        match self {
            Some(v) => v,
            None => unreachable!(),
        }
    }

    /// Inserts `value` into the option if it is `None`, then returns a mutable reference to the contained value.
    #[inline]
    pub fn get_or_insert(&mut self, value: T) -> &mut T {
        if let None = *self {
            *self = Some(value);
        }
        match self {
            Some(v) => v,
            None => unreachable!(),
        }
    }

    /// Inserts the default value into the option if it is `None`, then returns a mutable reference to the contained value.
    #[inline]
    pub fn get_or_insert_default(&mut self) -> &mut T
    where
        T: Default,
    {
        self.get_or_insert_with(T::default)
    }

    /// Inserts a value computed from `f` into the option if it is `None`, then returns a mutable reference to the contained value.
    #[inline]
    pub fn get_or_insert_with<F: FnOnce() -> T>(&mut self, f: F) -> &mut T {
        if let None = *self {
            *self = Some(f());
        }
        match self {
            Some(v) => v,
            None => unreachable!(),
        }
    }

    /// Takes the value out of the option, leaving a `None` in its place.
    #[inline]
    pub fn take(&mut self) -> Option<T> {
        crate::mem::replace(self, None)
    }

    /// Takes the value out of the option, but only if the predicate evaluates to `true` on a mutable reference to the value.
    #[inline]
    pub fn take_if<P: FnOnce(&mut T) -> bool>(&mut self, predicate: P) -> Option<T> {
        let take = match self.as_mut() {
            Some(v) => predicate(v),
            None => false,
        };
        if take {
            self.take()
        } else {
            None
        }
    }

    /// Replaces the actual value in the option by the value given in parameter, returning the old value if present.
    #[inline]
    pub fn replace(&mut self, value: T) -> Option<T> {
        crate::mem::replace(self, Some(value))
    }

    /// Zips `self` with another `Option`.
    #[inline]
    pub fn zip<U>(self, other: Option<U>) -> Option<(T, U)> {
        match (self, other) {
            (Some(a), Some(b)) => Some((a, b)),
            _ => None,
        }
    }

    /// Zips `self` and another `Option` with function `f`.
    #[inline]
    pub fn zip_with<U, F: FnOnce(T, U) -> R, R>(self, other: Option<U>, f: F) -> Option<R> {
        match (self, other) {
            (Some(a), Some(b)) => Some(f(a, b)),
            _ => None,
        }
    }
}

impl<T, U> Option<(T, U)> {
    /// Unzips an option containing a tuple of two options.
    #[inline]
    pub fn unzip(self) -> (Option<T>, Option<U>) {
        match self {
            Some((a, b)) => (Some(a), Some(b)),
            None => (None, None),
        }
    }
}

impl<T> Option<&T> {
    /// Maps an `Option<&T>` to an `Option<T>` by copying the contents of the option.
    #[inline]
    pub fn copied(self) -> Option<T>
    where
        T: Copy,
    {
        match self {
            Some(&v) => Some(v),
            None => None,
        }
    }

    /// Maps an `Option<&T>` to an `Option<T>` by cloning the contents of the option.
    #[inline]
    pub fn cloned(self) -> Option<T>
    where
        T: Clone,
    {
        match self {
            Some(t) => Some(t.clone()),
            None => None,
        }
    }
}

impl<T> Option<&mut T> {
    /// Maps an `Option<&mut T>` to an `Option<T>` by copying the contents of the option.
    #[inline]
    pub fn copied(self) -> Option<T>
    where
        T: Copy,
    {
        match self {
            Some(&mut t) => Some(t),
            None => None,
        }
    }

    /// Maps an `Option<&mut T>` to an `Option<T>` by cloning the contents of the option.
    #[inline]
    pub fn cloned(self) -> Option<T>
    where
        T: Clone,
    {
        match self {
            Some(t) => Some(t.clone()),
            None => None,
        }
    }
}

impl<T, E> Option<Result<T, E>> {
    /// Transposes an `Option` of a `Result` into a `Result` of an `Option`.
    #[inline]
    pub fn transpose(self) -> Result<Option<T>, E> {
        match self {
            Some(Ok(x)) => Ok(Some(x)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }
}

impl<T> Option<Option<T>> {
    /// Converts from `Option<Option<T>>` to `Option<T>`.
    #[inline]
    pub fn flatten(self) -> Option<T> {
        match self {
            Some(inner) => inner,
            None => None,
        }
    }
}

impl<T: Clone> Clone for Option<T> {
    #[inline]
    fn clone(&self) -> Self {
        match self {
            Some(x) => Some(x.clone()),
            None => None,
        }
    }
}

impl<T: Copy> Copy for Option<T> {}

impl<T> Default for Option<T> {
    #[inline]
    fn default() -> Option<T> {
        None
    }
}

impl<T: PartialEq> PartialEq for Option<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Some(a), Some(b)) => *a == *b,
            (None, None) => true,
            _ => false,
        }
    }
}

impl<T: Eq> Eq for Option<T> {}

impl<T: PartialOrd> PartialOrd for Option<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Some(a), Some(b)) => a.partial_cmp(b),
            (None, None) => Some(Ordering::Equal),
            (None, Some(_)) => Some(Ordering::Less),
            (Some(_), None) => Some(Ordering::Greater),
        }
    }
}

impl<T: Ord> Ord for Option<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Some(a), Some(b)) => a.cmp(b),
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
        }
    }
}

impl<T: Hash> Hash for Option<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            None => state.write_isize(0),
            Some(x) => {
                state.write_isize(1);
                x.hash(state);
            }
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Option<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Some(x) => f.debug_tuple("Some").field(x).finish(),
            None => f.write_str("None"),
        }
    }
}

impl<T> From<T> for Option<T> {
    #[inline]
    fn from(val: T) -> Option<T> {
        Some(val)
    }
}

impl<'a, T> From<&'a Option<T>> for Option<&'a T> {
    #[inline]
    fn from(o: &'a Option<T>) -> Option<&'a T> {
        o.as_ref()
    }
}

impl<'a, T> From<&'a mut Option<T>> for Option<&'a mut T> {
    #[inline]
    fn from(o: &'a mut Option<T>) -> Option<&'a mut T> {
        o.as_mut()
    }
}

// ---------------------------------------------------------------------------
// Iterators

/// An iterator over a reference to the `Some` variant of an `Option`.
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

impl<'a, T> Clone for Iter<'a, T> {
    fn clone(&self) -> Self {
        Iter { inner: self.inner }
    }
}

/// An iterator over a mutable reference to the `Some` variant of an `Option`.
pub struct IterMut<'a, T: 'a> {
    inner: Option<&'a mut T>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    #[inline]
    fn next(&mut self) -> Option<&'a mut T> {
        self.inner.take()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = if self.inner.is_some() { 1 } else { 0 };
        (n, Some(n))
    }
}

impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a mut T> {
        self.inner.take()
    }
}

impl<'a, T> ExactSizeIterator for IterMut<'a, T> {}
impl<'a, T> FusedIterator for IterMut<'a, T> {}

/// An iterator over the value in `Some` variant of an `Option`.
pub struct IntoIter<T> {
    inner: Option<T>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        self.inner.take()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = if self.inner.is_some() { 1 } else { 0 };
        (n, Some(n))
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        self.inner.take()
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {}
impl<T> FusedIterator for IntoIter<T> {}

impl<T: Clone> Clone for IntoIter<T> {
    fn clone(&self) -> Self {
        IntoIter { inner: self.inner.clone() }
    }
}

impl<T> IntoIterator for Option<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    #[inline]
    fn into_iter(self) -> IntoIter<T> {
        IntoIter { inner: self }
    }
}

impl<'a, T> IntoIterator for &'a Option<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Option<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;
    fn into_iter(self) -> IterMut<'a, T> {
        self.iter_mut()
    }
}

impl<A, V: FromIterator<A>> FromIterator<Option<A>> for Option<V> {
    /// Takes each element in the iterator: if it is `None`, no further
    /// elements are taken, and `None` is returned.
    fn from_iter<I: IntoIterator<Item = Option<A>>>(iter: I) -> Option<V> {
        let mut failed = false;
        let v: V = {
            let shunt = OptionShunt { iter: iter.into_iter(), failed: &mut failed };
            FromIterator::from_iter(shunt)
        };
        if failed {
            None
        } else {
            Some(v)
        }
    }
}

struct OptionShunt<'a, I> {
    iter: I,
    failed: &'a mut bool,
}

impl<'a, A, I: Iterator<Item = Option<A>>> Iterator for OptionShunt<'a, I> {
    type Item = A;
    fn next(&mut self) -> Option<A> {
        if *self.failed {
            return None;
        }
        match self.iter.next() {
            Some(Some(x)) => Some(x),
            Some(None) => {
                *self.failed = true;
                None
            }
            None => None,
        }
    }
}
