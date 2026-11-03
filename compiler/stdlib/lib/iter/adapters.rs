//! Iterator adapters.

use super::traits::{DoubleEndedIterator, ExactSizeIterator, FusedIterator, IntoIterator, Iterator};
use crate::fmt;

// ---------------------------------------------------------------------------
// Map

/// An iterator that maps the values of `iter` with `f`.
#[derive(Clone)]
#[must_use]
pub struct Map<I, F> {
    pub(crate) iter: I,
    f: F,
}

impl<I, F> Map<I, F> {
    pub(crate) fn new(iter: I, f: F) -> Map<I, F> {
        Map { iter, f }
    }
}

impl<I: fmt::Debug, F> fmt::Debug for Map<I, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Map").field("iter", &self.iter).finish()
    }
}

impl<B, I: Iterator, F> Iterator for Map<I, F>
where
    F: FnMut(I::Item) -> B,
{
    type Item = B;
    #[inline]
    fn next(&mut self) -> Option<B> {
        match self.iter.next() {
            Some(x) => Some((self.f)(x)),
            None => None,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<B, I: DoubleEndedIterator, F> DoubleEndedIterator for Map<I, F>
where
    F: FnMut(I::Item) -> B,
{
    #[inline]
    fn next_back(&mut self) -> Option<B> {
        match self.iter.next_back() {
            Some(x) => Some((self.f)(x)),
            None => None,
        }
    }
}

impl<B, I: ExactSizeIterator, F> ExactSizeIterator for Map<I, F>
where
    F: FnMut(I::Item) -> B,
{
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<B, I: FusedIterator, F> FusedIterator for Map<I, F> where F: FnMut(I::Item) -> B {}

// ---------------------------------------------------------------------------
// Filter

/// An iterator that filters the elements of `iter` with `predicate`.
#[derive(Clone)]
#[must_use]
pub struct Filter<I, P> {
    iter: I,
    predicate: P,
}

impl<I, P> Filter<I, P> {
    pub(crate) fn new(iter: I, predicate: P) -> Filter<I, P> {
        Filter { iter, predicate }
    }
}

impl<I: fmt::Debug, P> fmt::Debug for Filter<I, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Filter").field("iter", &self.iter).finish()
    }
}

impl<I: Iterator, P> Iterator for Filter<I, P>
where
    P: FnMut(&I::Item) -> bool,
{
    type Item = I::Item;
    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        while let Some(x) = self.iter.next() {
            if (self.predicate)(&x) {
                return Some(x);
            }
        }
        None
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper)
    }
}

impl<I: DoubleEndedIterator, P> DoubleEndedIterator for Filter<I, P>
where
    P: FnMut(&I::Item) -> bool,
{
    #[inline]
    fn next_back(&mut self) -> Option<I::Item> {
        while let Some(x) = self.iter.next_back() {
            if (self.predicate)(&x) {
                return Some(x);
            }
        }
        None
    }
}

impl<I: FusedIterator, P> FusedIterator for Filter<I, P> where P: FnMut(&I::Item) -> bool {}

// ---------------------------------------------------------------------------
// FilterMap

/// An iterator that uses `f` to both filter and map elements from `iter`.
#[derive(Clone)]
#[must_use]
pub struct FilterMap<I, F> {
    iter: I,
    f: F,
}

impl<I, F> FilterMap<I, F> {
    pub(crate) fn new(iter: I, f: F) -> FilterMap<I, F> {
        FilterMap { iter, f }
    }
}

impl<I: fmt::Debug, F> fmt::Debug for FilterMap<I, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilterMap").field("iter", &self.iter).finish()
    }
}

impl<B, I: Iterator, F> Iterator for FilterMap<I, F>
where
    F: FnMut(I::Item) -> Option<B>,
{
    type Item = B;
    #[inline]
    fn next(&mut self) -> Option<B> {
        while let Some(x) = self.iter.next() {
            if let Some(y) = (self.f)(x) {
                return Some(y);
            }
        }
        None
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper)
    }
}

impl<B, I: DoubleEndedIterator, F> DoubleEndedIterator for FilterMap<I, F>
where
    F: FnMut(I::Item) -> Option<B>,
{
    #[inline]
    fn next_back(&mut self) -> Option<B> {
        while let Some(x) = self.iter.next_back() {
            if let Some(y) = (self.f)(x) {
                return Some(y);
            }
        }
        None
    }
}

impl<B, I: FusedIterator, F> FusedIterator for FilterMap<I, F> where F: FnMut(I::Item) -> Option<B> {}

// ---------------------------------------------------------------------------
// Enumerate

/// An iterator that yields the current count and the element during iteration.
#[derive(Clone, Debug)]
#[must_use]
pub struct Enumerate<I> {
    iter: I,
    count: usize,
}

impl<I> Enumerate<I> {
    pub(crate) fn new(iter: I) -> Enumerate<I> {
        Enumerate { iter, count: 0 }
    }
}

impl<I: Iterator> Iterator for Enumerate<I> {
    type Item = (usize, I::Item);
    #[inline]
    fn next(&mut self) -> Option<(usize, I::Item)> {
        match self.iter.next() {
            Some(a) => {
                let i = self.count;
                self.count += 1;
                Some((i, a))
            }
            None => None,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
    #[inline]
    fn nth(&mut self, n: usize) -> Option<(usize, I::Item)> {
        match self.iter.nth(n) {
            Some(a) => {
                let i = self.count + n;
                self.count = i + 1;
                Some((i, a))
            }
            None => None,
        }
    }
    #[inline]
    fn count(self) -> usize {
        self.iter.count()
    }
}

impl<I: ExactSizeIterator + DoubleEndedIterator> DoubleEndedIterator for Enumerate<I> {
    #[inline]
    fn next_back(&mut self) -> Option<(usize, I::Item)> {
        match self.iter.next_back() {
            Some(a) => {
                let len = self.iter.len();
                Some((self.count + len, a))
            }
            None => None,
        }
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for Enumerate<I> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<I: FusedIterator> FusedIterator for Enumerate<I> {}

// ---------------------------------------------------------------------------
// Peekable

/// An iterator with a `peek()` that returns an optional reference to the next element.
#[must_use]
pub struct Peekable<I: Iterator> {
    iter: I,
    /// Remember a peeked value, even if it was None.
    peeked: Option<Option<I::Item>>,
}

impl<I: Iterator> Peekable<I> {
    pub(crate) fn new(iter: I) -> Peekable<I> {
        Peekable { iter, peeked: None }
    }

    /// Returns a reference to the next() value without advancing the iterator.
    #[inline]
    pub fn peek(&mut self) -> Option<&I::Item> {
        if self.peeked.is_none() {
            self.peeked = Some(self.iter.next());
        }
        match &self.peeked {
            Some(Some(v)) => Some(v),
            _ => None,
        }
    }

    /// Returns a mutable reference to the next() value without advancing the iterator.
    #[inline]
    pub fn peek_mut(&mut self) -> Option<&mut I::Item> {
        if self.peeked.is_none() {
            self.peeked = Some(self.iter.next());
        }
        match &mut self.peeked {
            Some(Some(v)) => Some(v),
            _ => None,
        }
    }

    /// Consume and return the next value of this iterator if a condition is true.
    pub fn next_if<F>(&mut self, func: F) -> Option<I::Item>
    where
        F: FnOnce(&I::Item) -> bool,
    {
        let next = match self.peeked.take() {
            Some(v) => v,
            None => self.iter.next(),
        };
        match next {
            Some(matched) => {
                if func(&matched) {
                    Some(matched)
                } else {
                    self.peeked = Some(Some(matched));
                    None
                }
            }
            None => {
                self.peeked = Some(None);
                None
            }
        }
    }

    /// Consume and return the next item if it is equal to `expected`.
    pub fn next_if_eq<T>(&mut self, expected: &T) -> Option<I::Item>
    where
        T: ?Sized,
        I::Item: PartialEq<T>,
    {
        self.next_if(|next| next == expected)
    }
}

impl<I: Iterator + Clone> Clone for Peekable<I>
where
    I::Item: Clone,
{
    fn clone(&self) -> Self {
        Peekable { iter: self.iter.clone(), peeked: self.peeked.clone() }
    }
}

impl<I: Iterator + fmt::Debug> fmt::Debug for Peekable<I>
where
    I::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Peekable").field("iter", &self.iter).field("peeked", &self.peeked).finish()
    }
}

impl<I: Iterator> Iterator for Peekable<I> {
    type Item = I::Item;
    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        match self.peeked.take() {
            Some(v) => v,
            None => self.iter.next(),
        }
    }
    #[inline]
    fn count(self) -> usize {
        let mut s = self;
        match s.peeked.take() {
            Some(None) => 0,
            Some(Some(_)) => 1 + s.iter.count(),
            None => s.iter.count(),
        }
    }
    #[inline]
    fn nth(&mut self, n: usize) -> Option<I::Item> {
        match self.peeked.take() {
            Some(None) => None,
            Some(v) => {
                if n == 0 {
                    v
                } else {
                    self.iter.nth(n - 1)
                }
            }
            None => self.iter.nth(n),
        }
    }
    #[inline]
    fn last(self) -> Option<I::Item> {
        let mut s = self;
        let peek_opt = match s.peeked.take() {
            Some(None) => return None,
            Some(v) => v,
            None => None,
        };
        match s.iter.last() {
            Some(x) => Some(x),
            None => peek_opt,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let peek_len = match self.peeked {
            Some(None) => return (0, Some(0)),
            Some(Some(_)) => 1,
            None => 0,
        };
        let (lo, hi) = self.iter.size_hint();
        let lo = lo.saturating_add(peek_len);
        let hi = match hi {
            Some(x) => x.checked_add(peek_len),
            None => None,
        };
        (lo, hi)
    }
}

impl<I: DoubleEndedIterator> DoubleEndedIterator for Peekable<I> {
    #[inline]
    fn next_back(&mut self) -> Option<I::Item> {
        match self.peeked.as_mut() {
            Some(None) => None,
            Some(Some(_)) => match self.iter.next_back() {
                Some(x) => Some(x),
                None => match self.peeked.take() {
                    Some(v) => v,
                    None => None,
                },
            },
            None => self.iter.next_back(),
        }
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for Peekable<I> {}
impl<I: FusedIterator> FusedIterator for Peekable<I> {}

// ---------------------------------------------------------------------------
// SkipWhile / TakeWhile / MapWhile

/// An iterator that rejects elements while `predicate` returns `true`.
#[derive(Clone)]
#[must_use]
pub struct SkipWhile<I, P> {
    iter: I,
    flag: bool,
    predicate: P,
}

impl<I, P> SkipWhile<I, P> {
    pub(crate) fn new(iter: I, predicate: P) -> SkipWhile<I, P> {
        SkipWhile { iter, flag: false, predicate }
    }
}

impl<I: Iterator, P> Iterator for SkipWhile<I, P>
where
    P: FnMut(&I::Item) -> bool,
{
    type Item = I::Item;
    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        loop {
            let x = match self.iter.next() {
                Some(x) => x,
                None => return None,
            };
            if self.flag || !(self.predicate)(&x) {
                self.flag = true;
                return Some(x);
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper)
    }
}

impl<I: FusedIterator, P> FusedIterator for SkipWhile<I, P> where P: FnMut(&I::Item) -> bool {}

/// An iterator that only accepts elements while `predicate` returns `true`.
#[derive(Clone)]
#[must_use]
pub struct TakeWhile<I, P> {
    iter: I,
    flag: bool,
    predicate: P,
}

impl<I, P> TakeWhile<I, P> {
    pub(crate) fn new(iter: I, predicate: P) -> TakeWhile<I, P> {
        TakeWhile { iter, flag: false, predicate }
    }
}

impl<I: Iterator, P> Iterator for TakeWhile<I, P>
where
    P: FnMut(&I::Item) -> bool,
{
    type Item = I::Item;
    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        if self.flag {
            None
        } else {
            let x = match self.iter.next() {
                Some(x) => x,
                None => return None,
            };
            if (self.predicate)(&x) {
                Some(x)
            } else {
                self.flag = true;
                None
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.flag {
            (0, Some(0))
        } else {
            let (_, upper) = self.iter.size_hint();
            (0, upper)
        }
    }
}

impl<I: FusedIterator, P> FusedIterator for TakeWhile<I, P> where P: FnMut(&I::Item) -> bool {}

/// An iterator that only accepts elements while `predicate` returns `Some(_)`.
#[derive(Clone)]
#[must_use]
pub struct MapWhile<I, P> {
    iter: I,
    predicate: P,
}

impl<I, P> MapWhile<I, P> {
    pub(crate) fn new(iter: I, predicate: P) -> MapWhile<I, P> {
        MapWhile { iter, predicate }
    }
}

impl<B, I: Iterator, P> Iterator for MapWhile<I, P>
where
    P: FnMut(I::Item) -> Option<B>,
{
    type Item = B;
    #[inline]
    fn next(&mut self) -> Option<B> {
        let x = match self.iter.next() {
            Some(x) => x,
            None => return None,
        };
        (self.predicate)(x)
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper)
    }
}

// ---------------------------------------------------------------------------
// Skip / Take

/// An iterator that skips over `n` elements of `iter`.
#[derive(Clone, Debug)]
#[must_use]
pub struct Skip<I> {
    iter: I,
    n: usize,
}

impl<I> Skip<I> {
    pub(crate) fn new(iter: I, n: usize) -> Skip<I> {
        Skip { iter, n }
    }
}

impl<I: Iterator> Iterator for Skip<I> {
    type Item = I::Item;
    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        if self.n > 0 {
            let n = self.n;
            self.n = 0;
            self.iter.nth(n)
        } else {
            self.iter.next()
        }
    }
    #[inline]
    fn nth(&mut self, n: usize) -> Option<I::Item> {
        if self.n > 0 {
            let skip = self.n;
            self.n = 0;
            let n = match skip.checked_add(n) {
                Some(nth) => nth,
                None => {
                    self.iter.nth(skip - 1);
                    n
                }
            };
            self.iter.nth(n)
        } else {
            self.iter.nth(n)
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.iter.size_hint();
        let lower = lower.saturating_sub(self.n);
        let upper = match upper {
            Some(x) => Some(x.saturating_sub(self.n)),
            None => None,
        };
        (lower, upper)
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for Skip<I> {}

impl<I> DoubleEndedIterator for Skip<I>
where
    I: DoubleEndedIterator + ExactSizeIterator,
{
    fn next_back(&mut self) -> Option<I::Item> {
        if self.len() > 0 {
            self.iter.next_back()
        } else {
            None
        }
    }
}

impl<I: FusedIterator> FusedIterator for Skip<I> {}

/// An iterator that only iterates over the first `n` iterations of `iter`.
#[derive(Clone, Debug)]
#[must_use]
pub struct Take<I> {
    iter: I,
    n: usize,
}

impl<I> Take<I> {
    pub(crate) fn new(iter: I, n: usize) -> Take<I> {
        Take { iter, n }
    }
}

impl<I: Iterator> Iterator for Take<I> {
    type Item = I::Item;
    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        if self.n != 0 {
            self.n -= 1;
            self.iter.next()
        } else {
            None
        }
    }
    #[inline]
    fn nth(&mut self, n: usize) -> Option<I::Item> {
        if self.n > n {
            self.n -= n + 1;
            self.iter.nth(n)
        } else {
            if self.n > 0 {
                self.iter.nth(self.n - 1);
                self.n = 0;
            }
            None
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.n == 0 {
            return (0, Some(0));
        }
        let (lower, upper) = self.iter.size_hint();
        let lower = if lower < self.n { lower } else { self.n };
        let upper = match upper {
            Some(x) if x < self.n => Some(x),
            _ => Some(self.n),
        };
        (lower, upper)
    }
}

impl<I> DoubleEndedIterator for Take<I>
where
    I: DoubleEndedIterator + ExactSizeIterator,
{
    #[inline]
    fn next_back(&mut self) -> Option<I::Item> {
        if self.n == 0 {
            None
        } else {
            let n = self.n;
            self.n -= 1;
            self.iter.nth_back(self.iter.len().saturating_sub(n))
        }
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for Take<I> {}

impl<A: Clone> DoubleEndedIterator for Take<crate::iter::Repeat<A>> {
    #[inline]
    fn next_back(&mut self) -> Option<A> {
        if self.n == 0 {
            None
        } else {
            self.n -= 1;
            self.iter.next()
        }
    }
}

impl<A: Clone> ExactSizeIterator for Take<crate::iter::Repeat<A>> {}
impl<I: FusedIterator> FusedIterator for Take<I> {}

// ---------------------------------------------------------------------------
// Scan

/// An iterator to maintain state while iterating another iterator.
#[derive(Clone)]
#[must_use]
pub struct Scan<I, St, F> {
    iter: I,
    f: F,
    state: St,
}

impl<I, St, F> Scan<I, St, F> {
    pub(crate) fn new(iter: I, state: St, f: F) -> Scan<I, St, F> {
        Scan { iter, f, state }
    }
}

impl<B, I, St, F> Iterator for Scan<I, St, F>
where
    I: Iterator,
    F: FnMut(&mut St, I::Item) -> Option<B>,
{
    type Item = B;
    #[inline]
    fn next(&mut self) -> Option<B> {
        let a = match self.iter.next() {
            Some(a) => a,
            None => return None,
        };
        (self.f)(&mut self.state, a)
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper)
    }
}

// ---------------------------------------------------------------------------
// FlatMap / Flatten

/// An iterator that maps each element to an iterator, and yields the elements of the produced iterators.
#[must_use]
pub struct FlatMap<I, U: IntoIterator, F> {
    iter: I,
    f: F,
    frontiter: Option<U::IntoIter>,
    backiter: Option<U::IntoIter>,
}

impl<I, U: IntoIterator, F> FlatMap<I, U, F> {
    pub(crate) fn new(iter: I, f: F) -> FlatMap<I, U, F> {
        FlatMap { iter, f, frontiter: None, backiter: None }
    }
}

impl<I: Clone, U, F: Clone> Clone for FlatMap<I, U, F>
where
    U: IntoIterator,
    U::IntoIter: Clone,
{
    fn clone(&self) -> Self {
        FlatMap {
            iter: self.iter.clone(),
            f: self.f.clone(),
            frontiter: self.frontiter.clone(),
            backiter: self.backiter.clone(),
        }
    }
}

impl<I: Iterator, U: IntoIterator, F> Iterator for FlatMap<I, U, F>
where
    F: FnMut(I::Item) -> U,
{
    type Item = U::Item;
    #[inline]
    fn next(&mut self) -> Option<U::Item> {
        loop {
            if let Some(inner) = &mut self.frontiter {
                match inner.next() {
                    None => self.frontiter = None,
                    elt => return elt,
                }
            }
            match self.iter.next() {
                None => {
                    return match &mut self.backiter {
                        Some(inner) => {
                            let r = inner.next();
                            if r.is_none() {
                                self.backiter = None;
                            }
                            r
                        }
                        None => None,
                    };
                }
                Some(x) => self.frontiter = Some((self.f)(x).into_iter()),
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let front = match &self.frontiter {
            Some(it) => it.size_hint(),
            None => (0, Some(0)),
        };
        let back = match &self.backiter {
            Some(it) => it.size_hint(),
            None => (0, Some(0)),
        };
        flatten_size_hint(front, back, self.iter.size_hint(), U::__flatten_bounds())
    }
}

/// `Flatten`/`FlatMap::size_hint`: the partially consumed front and back
/// inner iterators plus, per remaining outer item, the bounds its type
/// guarantees (exact for arrays, at most one for `Option`/`Result`).
fn flatten_size_hint(
    front: (usize, Option<usize>),
    back: (usize, Option<usize>),
    outer: (usize, Option<usize>),
    per_item: (usize, Option<usize>),
) -> (usize, Option<usize>) {
    let lo = front.0.saturating_add(back.0).saturating_add(outer.0.saturating_mul(per_item.0));
    let rest = match outer.1 {
        Some(0) => Some(0),
        Some(n) => match per_item.1 {
            Some(m) => n.checked_mul(m),
            None => None,
        },
        None => None,
    };
    let hi = match (front.1, back.1, rest) {
        (Some(a), Some(b), Some(c)) => match a.checked_add(b) {
            Some(ab) => ab.checked_add(c),
            None => None,
        },
        _ => None,
    };
    (lo, hi)
}

impl<I: DoubleEndedIterator, U, F> DoubleEndedIterator for FlatMap<I, U, F>
where
    F: FnMut(I::Item) -> U,
    U: IntoIterator,
    U::IntoIter: DoubleEndedIterator,
{
    #[inline]
    fn next_back(&mut self) -> Option<U::Item> {
        loop {
            if let Some(inner) = &mut self.backiter {
                match inner.next_back() {
                    None => self.backiter = None,
                    elt => return elt,
                }
            }
            match self.iter.next_back() {
                None => {
                    return match &mut self.frontiter {
                        Some(inner) => {
                            let r = inner.next_back();
                            if r.is_none() {
                                self.frontiter = None;
                            }
                            r
                        }
                        None => None,
                    };
                }
                Some(x) => self.backiter = Some((self.f)(x).into_iter()),
            }
        }
    }
}

impl<I: FusedIterator, U: IntoIterator, F> FusedIterator for FlatMap<I, U, F> where F: FnMut(I::Item) -> U {}

/// An iterator that flattens one level of nesting in an iterator of things that can be turned into iterators.
#[must_use]
pub struct Flatten<I>
where
    I: Iterator,
    I::Item: IntoIterator,
{
    iter: I,
    frontiter: Option<<I::Item as IntoIterator>::IntoIter>,
    backiter: Option<<I::Item as IntoIterator>::IntoIter>,
}

impl<I> Flatten<I>
where
    I: Iterator,
    I::Item: IntoIterator,
{
    pub(crate) fn new(iter: I) -> Flatten<I> {
        Flatten { iter, frontiter: None, backiter: None }
    }
}

impl<I> Clone for Flatten<I>
where
    I: Iterator + Clone,
    I::Item: IntoIterator,
    <I::Item as IntoIterator>::IntoIter: Clone,
{
    fn clone(&self) -> Self {
        Flatten { iter: self.iter.clone(), frontiter: self.frontiter.clone(), backiter: self.backiter.clone() }
    }
}

impl<I> Iterator for Flatten<I>
where
    I: Iterator,
    I::Item: IntoIterator,
{
    type Item = <I::Item as IntoIterator>::Item;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(inner) = &mut self.frontiter {
                match inner.next() {
                    None => self.frontiter = None,
                    elt => return elt,
                }
            }
            match self.iter.next() {
                None => {
                    return match &mut self.backiter {
                        Some(inner) => {
                            let r = inner.next();
                            if r.is_none() {
                                self.backiter = None;
                            }
                            r
                        }
                        None => None,
                    };
                }
                Some(x) => self.frontiter = Some(x.into_iter()),
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let front = match &self.frontiter {
            Some(it) => it.size_hint(),
            None => (0, Some(0)),
        };
        let back = match &self.backiter {
            Some(it) => it.size_hint(),
            None => (0, Some(0)),
        };
        flatten_size_hint(front, back, self.iter.size_hint(), <I::Item as IntoIterator>::__flatten_bounds())
    }
}

impl<I> DoubleEndedIterator for Flatten<I>
where
    I: DoubleEndedIterator,
    I::Item: IntoIterator,
    <I::Item as IntoIterator>::IntoIter: DoubleEndedIterator,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(inner) = &mut self.backiter {
                match inner.next_back() {
                    None => self.backiter = None,
                    elt => return elt,
                }
            }
            match self.iter.next_back() {
                None => {
                    return match &mut self.frontiter {
                        Some(inner) => {
                            let r = inner.next_back();
                            if r.is_none() {
                                self.frontiter = None;
                            }
                            r
                        }
                        None => None,
                    };
                }
                Some(x) => self.backiter = Some(x.into_iter()),
            }
        }
    }
}

impl<I> FusedIterator for Flatten<I>
where
    I: FusedIterator,
    I::Item: IntoIterator,
{
}

// ---------------------------------------------------------------------------
// Fuse

/// An iterator that yields `None` forever after the underlying iterator yields `None` once.
#[derive(Clone, Debug)]
#[must_use]
pub struct Fuse<I> {
    iter: Option<I>,
}

impl<I> Fuse<I> {
    pub(crate) fn new(iter: I) -> Fuse<I> {
        Fuse { iter: Some(iter) }
    }
}

impl<I: Iterator> Iterator for Fuse<I> {
    type Item = I::Item;
    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        let r = match &mut self.iter {
            Some(it) => it.next(),
            None => return None,
        };
        if r.is_none() {
            self.iter = None;
        }
        r
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.iter {
            Some(it) => it.size_hint(),
            None => (0, Some(0)),
        }
    }
}

impl<I: DoubleEndedIterator> DoubleEndedIterator for Fuse<I> {
    #[inline]
    fn next_back(&mut self) -> Option<I::Item> {
        let r = match &mut self.iter {
            Some(it) => it.next_back(),
            None => return None,
        };
        if r.is_none() {
            self.iter = None;
        }
        r
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for Fuse<I> {}
impl<I: Iterator> FusedIterator for Fuse<I> {}

// ---------------------------------------------------------------------------
// Inspect

/// An iterator that calls a function with a reference to each element before yielding it.
#[derive(Clone)]
#[must_use]
pub struct Inspect<I, F> {
    iter: I,
    f: F,
}

impl<I, F> Inspect<I, F> {
    pub(crate) fn new(iter: I, f: F) -> Inspect<I, F> {
        Inspect { iter, f }
    }
}

impl<I: Iterator, F> Iterator for Inspect<I, F>
where
    F: FnMut(&I::Item),
{
    type Item = I::Item;
    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        match self.iter.next() {
            Some(a) => {
                (self.f)(&a);
                Some(a)
            }
            None => None,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<I: DoubleEndedIterator, F> DoubleEndedIterator for Inspect<I, F>
where
    F: FnMut(&I::Item),
{
    #[inline]
    fn next_back(&mut self) -> Option<I::Item> {
        match self.iter.next_back() {
            Some(a) => {
                (self.f)(&a);
                Some(a)
            }
            None => None,
        }
    }
}

impl<I: ExactSizeIterator, F> ExactSizeIterator for Inspect<I, F> where F: FnMut(&I::Item) {}

// ---------------------------------------------------------------------------
// Chain

/// An iterator that links two iterators together, in a chain.
#[derive(Clone, Debug)]
#[must_use]
pub struct Chain<A, B> {
    a: Option<A>,
    b: Option<B>,
}

impl<A, B> Chain<A, B> {
    pub(crate) fn new(a: A, b: B) -> Chain<A, B> {
        Chain { a: Some(a), b: Some(b) }
    }
}

impl<A, B> Iterator for Chain<A, B>
where
    A: Iterator,
    B: Iterator<Item = A::Item>,
{
    type Item = A::Item;
    #[inline]
    fn next(&mut self) -> Option<A::Item> {
        if let Some(a) = &mut self.a {
            match a.next() {
                None => self.a = None,
                item => return item,
            }
        }
        match &mut self.b {
            Some(b) => b.next(),
            None => None,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match (&self.a, &self.b) {
            (Some(a), Some(b)) => {
                let (a_lower, a_upper) = a.size_hint();
                let (b_lower, b_upper) = b.size_hint();
                let lower = a_lower.saturating_add(b_lower);
                let upper = match (a_upper, b_upper) {
                    (Some(x), Some(y)) => x.checked_add(y),
                    _ => None,
                };
                (lower, upper)
            }
            (Some(a), None) => a.size_hint(),
            (None, Some(b)) => b.size_hint(),
            (None, None) => (0, Some(0)),
        }
    }
    #[inline]
    fn count(self) -> usize {
        let a_count = match self.a {
            Some(a) => a.count(),
            None => 0,
        };
        let b_count = match self.b {
            Some(b) => b.count(),
            None => 0,
        };
        a_count + b_count
    }
}

impl<A, B> DoubleEndedIterator for Chain<A, B>
where
    A: DoubleEndedIterator,
    B: DoubleEndedIterator<Item = A::Item>,
{
    #[inline]
    fn next_back(&mut self) -> Option<A::Item> {
        if let Some(b) = &mut self.b {
            match b.next_back() {
                None => self.b = None,
                item => return item,
            }
        }
        match &mut self.a {
            Some(a) => a.next_back(),
            None => None,
        }
    }
}

impl<A, B> ExactSizeIterator for Chain<A, B>
where
    A: ExactSizeIterator,
    B: ExactSizeIterator<Item = A::Item>,
{
}

impl<A, B> FusedIterator for Chain<A, B>
where
    A: FusedIterator,
    B: FusedIterator<Item = A::Item>,
{
}

// ---------------------------------------------------------------------------
// Zip

/// An iterator that iterates two other iterators simultaneously.
#[derive(Clone, Debug)]
#[must_use]
pub struct Zip<A, B> {
    a: A,
    b: B,
}

impl<A, B> Zip<A, B> {
    pub(crate) fn new(a: A, b: B) -> Zip<A, B> {
        Zip { a, b }
    }
}

impl<A, B> Iterator for Zip<A, B>
where
    A: Iterator,
    B: Iterator,
{
    type Item = (A::Item, B::Item);
    #[inline]
    fn next(&mut self) -> Option<(A::Item, B::Item)> {
        let x = match self.a.next() {
            Some(x) => x,
            None => return None,
        };
        let y = match self.b.next() {
            Some(y) => y,
            None => return None,
        };
        Some((x, y))
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (a_lower, a_upper) = self.a.size_hint();
        let (b_lower, b_upper) = self.b.size_hint();
        let lower = if a_lower < b_lower { a_lower } else { b_lower };
        let upper = match (a_upper, b_upper) {
            (Some(x), Some(y)) => Some(if x < y { x } else { y }),
            (Some(x), None) => Some(x),
            (None, Some(y)) => Some(y),
            (None, None) => None,
        };
        (lower, upper)
    }
}

impl<A, B> DoubleEndedIterator for Zip<A, B>
where
    A: DoubleEndedIterator + ExactSizeIterator,
    B: DoubleEndedIterator + ExactSizeIterator,
{
    #[inline]
    fn next_back(&mut self) -> Option<(A::Item, B::Item)> {
        let a_sz = self.a.len();
        let b_sz = self.b.len();
        if a_sz != b_sz {
            if a_sz > b_sz {
                let mut i = 0;
                while i < a_sz - b_sz {
                    self.a.next_back();
                    i += 1;
                }
            } else {
                let mut i = 0;
                while i < b_sz - a_sz {
                    self.b.next_back();
                    i += 1;
                }
            }
        }
        match (self.a.next_back(), self.b.next_back()) {
            (Some(x), Some(y)) => Some((x, y)),
            (None, None) => None,
            _ => None,
        }
    }
}

impl<A, B> ExactSizeIterator for Zip<A, B>
where
    A: ExactSizeIterator,
    B: ExactSizeIterator,
{
}

impl<A, B> FusedIterator for Zip<A, B>
where
    A: FusedIterator,
    B: FusedIterator,
{
}

// ---------------------------------------------------------------------------
// Rev

/// A double-ended iterator with the direction inverted.
#[derive(Clone, Debug)]
#[must_use]
pub struct Rev<T> {
    iter: T,
}

impl<T> Rev<T> {
    pub(crate) fn new(iter: T) -> Rev<T> {
        Rev { iter }
    }
}

impl<I: DoubleEndedIterator> Iterator for Rev<I> {
    type Item = I::Item;
    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        self.iter.next_back()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
    #[inline]
    fn nth(&mut self, n: usize) -> Option<I::Item> {
        self.iter.nth_back(n)
    }
    #[inline]
    fn find<P>(&mut self, predicate: P) -> Option<Self::Item>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        self.iter.rfind(predicate)
    }
}

impl<I: DoubleEndedIterator> DoubleEndedIterator for Rev<I> {
    #[inline]
    fn next_back(&mut self) -> Option<I::Item> {
        self.iter.next()
    }
    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<I::Item> {
        self.iter.nth(n)
    }
    #[inline]
    fn rfind<P>(&mut self, predicate: P) -> Option<Self::Item>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        self.iter.find(predicate)
    }
}

impl<I: DoubleEndedIterator + ExactSizeIterator> ExactSizeIterator for Rev<I> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<I: DoubleEndedIterator + FusedIterator> FusedIterator for Rev<I> {}

// ---------------------------------------------------------------------------
// Copied / Cloned

/// An iterator that copies the elements of an underlying iterator.
#[derive(Clone, Debug)]
#[must_use]
pub struct Copied<I> {
    it: I,
}

impl<I> Copied<I> {
    pub(crate) fn new(it: I) -> Copied<I> {
        Copied { it }
    }
}

impl<'a, I, T: 'a> Iterator for Copied<I>
where
    I: Iterator<Item = &'a T>,
    T: Copy,
{
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        match self.it.next() {
            Some(x) => Some(*x),
            None => None,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.it.size_hint()
    }
    #[inline]
    fn nth(&mut self, n: usize) -> Option<T> {
        match self.it.nth(n) {
            Some(x) => Some(*x),
            None => None,
        }
    }
}

impl<'a, I, T: 'a> DoubleEndedIterator for Copied<I>
where
    I: DoubleEndedIterator<Item = &'a T>,
    T: Copy,
{
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        match self.it.next_back() {
            Some(x) => Some(*x),
            None => None,
        }
    }
}

impl<'a, I, T: 'a> ExactSizeIterator for Copied<I>
where
    I: ExactSizeIterator<Item = &'a T>,
    T: Copy,
{
    fn len(&self) -> usize {
        self.it.len()
    }
}

impl<'a, I, T: 'a> FusedIterator for Copied<I>
where
    I: FusedIterator<Item = &'a T>,
    T: Copy,
{
}

/// An iterator that clones the elements of an underlying iterator.
#[derive(Clone, Debug)]
#[must_use]
pub struct Cloned<I> {
    it: I,
}

impl<I> Cloned<I> {
    pub(crate) fn new(it: I) -> Cloned<I> {
        Cloned { it }
    }
}

impl<'a, I, T: 'a> Iterator for Cloned<I>
where
    I: Iterator<Item = &'a T>,
    T: Clone,
{
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        match self.it.next() {
            Some(x) => Some(x.clone()),
            None => None,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.it.size_hint()
    }
    #[inline]
    fn nth(&mut self, n: usize) -> Option<T> {
        match self.it.nth(n) {
            Some(x) => Some(x.clone()),
            None => None,
        }
    }
}

impl<'a, I, T: 'a> DoubleEndedIterator for Cloned<I>
where
    I: DoubleEndedIterator<Item = &'a T>,
    T: Clone,
{
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        match self.it.next_back() {
            Some(x) => Some(x.clone()),
            None => None,
        }
    }
}

impl<'a, I, T: 'a> ExactSizeIterator for Cloned<I>
where
    I: ExactSizeIterator<Item = &'a T>,
    T: Clone,
{
    fn len(&self) -> usize {
        self.it.len()
    }
}

impl<'a, I, T: 'a> FusedIterator for Cloned<I>
where
    I: FusedIterator<Item = &'a T>,
    T: Clone,
{
}

// ---------------------------------------------------------------------------
// Cycle

/// An iterator that repeats endlessly.
#[derive(Clone, Debug)]
#[must_use]
pub struct Cycle<I> {
    orig: I,
    iter: I,
}

impl<I: Clone> Cycle<I> {
    pub(crate) fn new(iter: I) -> Cycle<I> {
        Cycle { orig: iter.clone(), iter }
    }
}

impl<I> Iterator for Cycle<I>
where
    I: Clone + Iterator,
{
    type Item = I::Item;
    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        match self.iter.next() {
            None => {
                self.iter = self.orig.clone();
                self.iter.next()
            }
            y => y,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.orig.size_hint() {
            (0, Some(0)) => (0, Some(0)),
            (0, _) => (0, None),
            _ => (usize::MAX, None),
        }
    }
}

// ---------------------------------------------------------------------------
// StepBy

/// An iterator for stepping iterators by a custom amount.
#[derive(Clone, Debug)]
#[must_use]
pub struct StepBy<I> {
    iter: I,
    step_minus_one: usize,
    first_take: bool,
}

impl<I> StepBy<I> {
    pub(crate) fn new(iter: I, step: usize) -> StepBy<I> {
        if step == 0 {
            crate::rt::panic_str("assertion failed: step != 0", file!(), line!(), column!());
        }
        StepBy { iter, step_minus_one: step - 1, first_take: true }
    }
}

impl<I: Iterator> Iterator for StepBy<I> {
    type Item = I::Item;
    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        let step_size = if self.first_take { 0 } else { self.step_minus_one };
        self.first_take = false;
        self.iter.nth(step_size)
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let step = self.step_minus_one + 1;
        let (low, high) = self.iter.size_hint();
        if self.first_take {
            let f = |n: usize| if n == 0 { 0 } else { 1 + (n - 1) / step };
            let h = match high {
                Some(h) => Some(f(h)),
                None => None,
            };
            (f(low), h)
        } else {
            let h = match high {
                Some(h) => Some(h / step),
                None => None,
            };
            (low / step, h)
        }
    }
}

impl<I: DoubleEndedIterator + ExactSizeIterator> StepBy<I> {
    fn next_back_index(&self) -> usize {
        let rem = self.iter.len() % (self.step_minus_one + 1);
        if self.first_take {
            if rem == 0 {
                self.step_minus_one
            } else {
                rem - 1
            }
        } else {
            rem
        }
    }
}

impl<I: DoubleEndedIterator + ExactSizeIterator> DoubleEndedIterator for StepBy<I> {
    #[inline]
    fn next_back(&mut self) -> Option<I::Item> {
        let n = self.next_back_index();
        self.iter.nth_back(n)
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for StepBy<I> {}
