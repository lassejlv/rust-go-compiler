//! The iterator traits.

use super::adapters::{
    Chain, Cloned, Copied, Cycle, Enumerate, Filter, FilterMap, FlatMap, Flatten, Fuse, Inspect, Map, MapWhile, Peekable,
    Rev, Scan, Skip, SkipWhile, StepBy, Take, TakeWhile, Zip,
};
use crate::cmp::Ordering;
use crate::ops::{ControlFlow, Try};

/// An interface for dealing with iterators.
#[lang = "iterator"]
#[must_use]
pub trait Iterator {
    /// The type of the elements being iterated over.
    type Item;

    /// Advances the iterator and returns the next value.
    fn next(&mut self) -> Option<Self::Item>;

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }

    /// Consumes the iterator, counting the number of iterations and returning it.
    #[inline]
    fn count(self) -> usize
    where
        Self: Sized,
    {
        let mut it = self;
        let mut n: usize = 0;
        while let Some(_) = it.next() {
            n += 1;
        }
        n
    }

    /// Consumes the iterator, returning the last element.
    #[inline]
    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        let mut it = self;
        let mut last = None;
        while let Some(x) = it.next() {
            last = Some(x);
        }
        last
    }

    /// Advances the iterator by `n` elements. Returns the number of steps that could not be taken.
    #[inline]
    fn advance_by(&mut self, n: usize) -> Result<(), usize> {
        let mut i = 0;
        while i < n {
            if self.next().is_none() {
                return Err(n - i);
            }
            i += 1;
        }
        Ok(())
    }

    /// Returns the `n`th element of the iterator.
    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let mut i = 0;
        while i < n {
            if self.next().is_none() {
                return None;
            }
            i += 1;
        }
        self.next()
    }

    /// Creates an iterator starting at the same point, but stepping by the given amount at each iteration.
    #[inline]
    fn step_by(self, step: usize) -> StepBy<Self>
    where
        Self: Sized,
    {
        StepBy::new(self, step)
    }

    /// Takes two iterators and creates a new iterator over both in sequence.
    #[inline]
    fn chain<U>(self, other: U) -> Chain<Self, U::IntoIter>
    where
        Self: Sized,
        U: IntoIterator<Item = Self::Item>,
    {
        Chain::new(self, other.into_iter())
    }

    /// 'Zips up' two iterators into a single iterator of pairs.
    #[inline]
    fn zip<U>(self, other: U) -> Zip<Self, U::IntoIter>
    where
        Self: Sized,
        U: IntoIterator,
    {
        Zip::new(self, other.into_iter())
    }

    /// Takes a closure and creates an iterator which calls that closure on each element.
    #[inline]
    fn map<B, F>(self, f: F) -> Map<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> B,
    {
        Map::new(self, f)
    }

    /// Calls a closure on each element of an iterator.
    #[inline]
    fn for_each<F>(self, f: F)
    where
        Self: Sized,
        F: FnMut(Self::Item),
    {
        let mut it = self;
        let mut f = f;
        while let Some(x) = it.next() {
            f(x);
        }
    }

    /// Creates an iterator which uses a closure to determine if an element should be yielded.
    #[inline]
    fn filter<P>(self, predicate: P) -> Filter<Self, P>
    where
        Self: Sized,
        P: FnMut(&Self::Item) -> bool,
    {
        Filter::new(self, predicate)
    }

    /// Creates an iterator that both filters and maps.
    #[inline]
    fn filter_map<B, F>(self, f: F) -> FilterMap<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> Option<B>,
    {
        FilterMap::new(self, f)
    }

    /// Creates an iterator which gives the current iteration count as well as the next value.
    #[inline]
    fn enumerate(self) -> Enumerate<Self>
    where
        Self: Sized,
    {
        Enumerate::new(self)
    }

    /// Creates an iterator which can use the `peek` and `peek_mut` methods.
    #[inline]
    fn peekable(self) -> Peekable<Self>
    where
        Self: Sized,
    {
        Peekable::new(self)
    }

    /// Creates an iterator that skips elements based on a predicate.
    #[inline]
    fn skip_while<P>(self, predicate: P) -> SkipWhile<Self, P>
    where
        Self: Sized,
        P: FnMut(&Self::Item) -> bool,
    {
        SkipWhile::new(self, predicate)
    }

    /// Creates an iterator that yields elements based on a predicate.
    #[inline]
    fn take_while<P>(self, predicate: P) -> TakeWhile<Self, P>
    where
        Self: Sized,
        P: FnMut(&Self::Item) -> bool,
    {
        TakeWhile::new(self, predicate)
    }

    /// Creates an iterator that both yields elements based on a predicate and maps.
    #[inline]
    fn map_while<B, P>(self, predicate: P) -> MapWhile<Self, P>
    where
        Self: Sized,
        P: FnMut(Self::Item) -> Option<B>,
    {
        MapWhile::new(self, predicate)
    }

    /// Creates an iterator that skips the first `n` elements.
    #[inline]
    fn skip(self, n: usize) -> Skip<Self>
    where
        Self: Sized,
    {
        Skip::new(self, n)
    }

    /// Creates an iterator that yields the first `n` elements, or fewer if the underlying iterator ends sooner.
    #[inline]
    fn take(self, n: usize) -> Take<Self>
    where
        Self: Sized,
    {
        Take::new(self, n)
    }

    /// An iterator adapter which, like `fold`, holds internal state, but unlike `fold`, produces a new iterator.
    #[inline]
    fn scan<St, B, F>(self, initial_state: St, f: F) -> Scan<Self, St, F>
    where
        Self: Sized,
        F: FnMut(&mut St, Self::Item) -> Option<B>,
    {
        Scan::new(self, initial_state, f)
    }

    /// Creates an iterator that works like map, but flattens nested structure.
    #[inline]
    fn flat_map<U, F>(self, f: F) -> FlatMap<Self, U, F>
    where
        Self: Sized,
        U: IntoIterator,
        F: FnMut(Self::Item) -> U,
    {
        FlatMap::new(self, f)
    }

    /// Creates an iterator that flattens nested structure.
    #[inline]
    fn flatten(self) -> Flatten<Self>
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        Flatten::new(self)
    }

    /// Creates an iterator which ends after the first `None`.
    #[inline]
    fn fuse(self) -> Fuse<Self>
    where
        Self: Sized,
    {
        Fuse::new(self)
    }

    /// Does something with each element of an iterator, passing the value on.
    #[inline]
    fn inspect<F>(self, f: F) -> Inspect<Self, F>
    where
        Self: Sized,
        F: FnMut(&Self::Item),
    {
        Inspect::new(self, f)
    }

    /// Borrows an iterator, rather than consuming it.
    #[inline]
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }

    /// Transforms an iterator into a collection.
    #[inline]
    fn collect<B: FromIterator<Self::Item>>(self) -> B
    where
        Self: Sized,
    {
        FromIterator::from_iter(self)
    }

    /// Consumes an iterator, creating two collections from it.
    fn partition<B, F>(self, f: F) -> (B, B)
    where
        Self: Sized,
        B: Default + Extend<Self::Item>,
        F: FnMut(&Self::Item) -> bool,
    {
        let mut left: B = Default::default();
        let mut right: B = Default::default();
        let mut it = self;
        let mut f = f;
        while let Some(x) = it.next() {
            if f(&x) {
                left.extend_one(x);
            } else {
                right.extend_one(x);
            }
        }
        (left, right)
    }

    /// Applies a function as long as it returns successfully, producing a single, final value.
    #[inline]
    fn try_fold<B, F, R>(&mut self, init: B, f: F) -> R
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> R,
        R: Try<Output = B>,
    {
        let mut f = f;
        let mut accum = init;
        while let Some(x) = self.next() {
            match f(accum, x).branch() {
                ControlFlow::Continue(c) => accum = c,
                ControlFlow::Break(r) => return R::from_residual(r),
            }
        }
        R::from_output(accum)
    }

    /// Applies a fallible function to each item in the iterator, stopping at the first error.
    #[inline]
    fn try_for_each<F, R>(&mut self, f: F) -> R
    where
        Self: Sized,
        F: FnMut(Self::Item) -> R,
        R: Try<Output = ()>,
    {
        let mut f = f;
        while let Some(x) = self.next() {
            match f(x).branch() {
                ControlFlow::Continue(_) => {}
                ControlFlow::Break(r) => return R::from_residual(r),
            }
        }
        R::from_output(())
    }

    /// Folds every element into an accumulator by applying an operation, returning the final result.
    #[inline]
    fn fold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        let mut it = self;
        let mut f = f;
        let mut accum = init;
        while let Some(x) = it.next() {
            accum = f(accum, x);
        }
        accum
    }

    /// Reduces the elements to a single one, by repeatedly applying a reducing operation.
    #[inline]
    fn reduce<F>(self, f: F) -> Option<Self::Item>
    where
        Self: Sized,
        F: FnMut(Self::Item, Self::Item) -> Self::Item,
    {
        let mut it = self;
        let first = match it.next() {
            Some(x) => x,
            None => return None,
        };
        Some(it.fold(first, f))
    }

    /// Reduces the elements to a single one by repeatedly applying a reducing operation that may fail.
    #[inline]
    fn try_reduce<F, R>(&mut self, f: F) -> Option<R::Output>
    where
        Self: Sized,
        F: FnMut(Self::Item, Self::Item) -> R,
        R: Try<Output = Self::Item>,
    {
        let mut f = f;
        let mut accum = match self.next() {
            Some(x) => x,
            None => return None,
        };
        while let Some(x) = self.next() {
            match f(accum, x).branch() {
                ControlFlow::Continue(c) => accum = c,
                ControlFlow::Break(_) => return None,
            }
        }
        Some(accum)
    }

    /// Tests if every element of the iterator matches a predicate.
    #[inline]
    fn all<F>(&mut self, f: F) -> bool
    where
        Self: Sized,
        F: FnMut(Self::Item) -> bool,
    {
        let mut f = f;
        while let Some(x) = self.next() {
            if !f(x) {
                return false;
            }
        }
        true
    }

    /// Tests if any element of the iterator matches a predicate.
    #[inline]
    fn any<F>(&mut self, f: F) -> bool
    where
        Self: Sized,
        F: FnMut(Self::Item) -> bool,
    {
        let mut f = f;
        while let Some(x) = self.next() {
            if f(x) {
                return true;
            }
        }
        false
    }

    /// Searches for an element of an iterator that satisfies a predicate.
    #[inline]
    fn find<P>(&mut self, predicate: P) -> Option<Self::Item>
    where
        Self: Sized,
        P: FnMut(&Self::Item) -> bool,
    {
        let mut predicate = predicate;
        while let Some(x) = self.next() {
            if predicate(&x) {
                return Some(x);
            }
        }
        None
    }

    /// Applies function to the elements of iterator and returns the first non-none result.
    #[inline]
    fn find_map<B, F>(&mut self, f: F) -> Option<B>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> Option<B>,
    {
        let mut f = f;
        while let Some(x) = self.next() {
            if let Some(b) = f(x) {
                return Some(b);
            }
        }
        None
    }

    /// Searches for an element in an iterator, returning its index.
    #[inline]
    fn position<P>(&mut self, predicate: P) -> Option<usize>
    where
        Self: Sized,
        P: FnMut(Self::Item) -> bool,
    {
        let mut predicate = predicate;
        let mut i: usize = 0;
        while let Some(x) = self.next() {
            if predicate(x) {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    /// Searches for an element in an iterator from the right, returning its index.
    #[inline]
    fn rposition<P>(&mut self, predicate: P) -> Option<usize>
    where
        P: FnMut(Self::Item) -> bool,
        Self: Sized + ExactSizeIterator + DoubleEndedIterator,
    {
        let mut predicate = predicate;
        let mut i = self.len();
        while let Some(x) = self.next_back() {
            i -= 1;
            if predicate(x) {
                return Some(i);
            }
        }
        None
    }

    /// Returns the maximum element of an iterator (the last one if several are equally maximum).
    #[inline]
    fn max(self) -> Option<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        self.max_by(|a, b| a.cmp(b))
    }

    /// Returns the minimum element of an iterator (the first one if several are equally minimum).
    #[inline]
    fn min(self) -> Option<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        self.min_by(|a, b| a.cmp(b))
    }

    /// Returns the element that gives the maximum value from the specified function.
    #[inline]
    fn max_by_key<B: Ord, F>(self, f: F) -> Option<Self::Item>
    where
        Self: Sized,
        F: FnMut(&Self::Item) -> B,
    {
        let mut it = self;
        let mut f = f;
        let mut best = match it.next() {
            Some(x) => x,
            None => return None,
        };
        let mut best_key = f(&best);
        while let Some(x) = it.next() {
            let k = f(&x);
            if k >= best_key {
                best = x;
                best_key = k;
            }
        }
        Some(best)
    }

    /// Returns the element that gives the maximum value with respect to the specified comparison function.
    #[inline]
    fn max_by<F>(self, compare: F) -> Option<Self::Item>
    where
        Self: Sized,
        F: FnMut(&Self::Item, &Self::Item) -> Ordering,
    {
        let mut it = self;
        let mut compare = compare;
        let mut best = match it.next() {
            Some(x) => x,
            None => return None,
        };
        while let Some(x) = it.next() {
            match compare(&best, &x) {
                Ordering::Greater => {}
                _ => best = x,
            }
        }
        Some(best)
    }

    /// Returns the element that gives the minimum value from the specified function.
    #[inline]
    fn min_by_key<B: Ord, F>(self, f: F) -> Option<Self::Item>
    where
        Self: Sized,
        F: FnMut(&Self::Item) -> B,
    {
        let mut it = self;
        let mut f = f;
        let mut best = match it.next() {
            Some(x) => x,
            None => return None,
        };
        let mut best_key = f(&best);
        while let Some(x) = it.next() {
            let k = f(&x);
            if k < best_key {
                best = x;
                best_key = k;
            }
        }
        Some(best)
    }

    /// Returns the element that gives the minimum value with respect to the specified comparison function.
    #[inline]
    fn min_by<F>(self, compare: F) -> Option<Self::Item>
    where
        Self: Sized,
        F: FnMut(&Self::Item, &Self::Item) -> Ordering,
    {
        let mut it = self;
        let mut compare = compare;
        let mut best = match it.next() {
            Some(x) => x,
            None => return None,
        };
        while let Some(x) = it.next() {
            match compare(&best, &x) {
                Ordering::Greater => best = x,
                _ => {}
            }
        }
        Some(best)
    }

    /// Reverses an iterator's direction.
    #[inline]
    fn rev(self) -> Rev<Self>
    where
        Self: Sized + DoubleEndedIterator,
    {
        Rev::new(self)
    }

    /// Converts an iterator of pairs into a pair of containers.
    fn unzip<A, B, FromA, FromB>(self) -> (FromA, FromB)
    where
        FromA: Default + Extend<A>,
        FromB: Default + Extend<B>,
        Self: Sized + Iterator<Item = (A, B)>,
    {
        let mut a: FromA = Default::default();
        let mut b: FromB = Default::default();
        let mut it = self;
        while let Some((x, y)) = it.next() {
            a.extend_one(x);
            b.extend_one(y);
        }
        (a, b)
    }

    /// Creates an iterator which copies all of its elements.
    #[inline]
    fn copied<'a, T>(self) -> Copied<Self>
    where
        T: 'a + Copy,
        Self: Sized + Iterator<Item = &'a T>,
    {
        Copied::new(self)
    }

    /// Creates an iterator which clones all of its elements.
    #[inline]
    fn cloned<'a, T>(self) -> Cloned<Self>
    where
        T: 'a + Clone,
        Self: Sized + Iterator<Item = &'a T>,
    {
        Cloned::new(self)
    }

    /// Repeats an iterator endlessly.
    #[inline]
    fn cycle(self) -> Cycle<Self>
    where
        Self: Sized + Clone,
    {
        Cycle::new(self)
    }

    /// Sums the elements of an iterator.
    #[inline]
    fn sum<S>(self) -> S
    where
        Self: Sized,
        S: Sum<Self::Item>,
    {
        Sum::sum(self)
    }

    /// Iterates over the entire iterator, multiplying all the elements.
    #[inline]
    fn product<P>(self) -> P
    where
        Self: Sized,
        P: Product<Self::Item>,
    {
        Product::product(self)
    }

    /// Lexicographically compares the elements of this iterator with those of another.
    fn cmp<I>(self, other: I) -> Ordering
    where
        I: IntoIterator<Item = Self::Item>,
        Self::Item: Ord,
        Self: Sized,
    {
        let mut a = self;
        let mut b = other.into_iter();
        loop {
            match (a.next(), b.next()) {
                (None, None) => return Ordering::Equal,
                (None, Some(_)) => return Ordering::Less,
                (Some(_), None) => return Ordering::Greater,
                (Some(x), Some(y)) => match x.cmp(&y) {
                    Ordering::Equal => {}
                    non_eq => return non_eq,
                },
            }
        }
    }

    /// Lexicographically compares the elements of this iterator with those of another with respect to the specified comparison function.
    fn cmp_by<I, F>(self, other: I, cmp: F) -> Ordering
    where
        Self: Sized,
        I: IntoIterator,
        F: FnMut(Self::Item, I::Item) -> Ordering,
    {
        let mut a = self;
        let mut b = other.into_iter();
        let mut cmp = cmp;
        loop {
            match (a.next(), b.next()) {
                (None, None) => return Ordering::Equal,
                (None, Some(_)) => return Ordering::Less,
                (Some(_), None) => return Ordering::Greater,
                (Some(x), Some(y)) => match cmp(x, y) {
                    Ordering::Equal => {}
                    non_eq => return non_eq,
                },
            }
        }
    }

    /// Lexicographically compares the `PartialOrd` elements of this iterator with those of another.
    fn partial_cmp<I>(self, other: I) -> Option<Ordering>
    where
        I: IntoIterator,
        Self::Item: PartialOrd<I::Item>,
        Self: Sized,
    {
        let mut a = self;
        let mut b = other.into_iter();
        loop {
            match (a.next(), b.next()) {
                (None, None) => return Some(Ordering::Equal),
                (None, Some(_)) => return Some(Ordering::Less),
                (Some(_), None) => return Some(Ordering::Greater),
                (Some(x), Some(y)) => match x.partial_cmp(&y) {
                    Some(Ordering::Equal) => {}
                    non_eq => return non_eq,
                },
            }
        }
    }

    /// Determines if the elements of this iterator are equal to those of another.
    fn eq<I>(self, other: I) -> bool
    where
        I: IntoIterator,
        Self::Item: PartialEq<I::Item>,
        Self: Sized,
    {
        let mut a = self;
        let mut b = other.into_iter();
        loop {
            match (a.next(), b.next()) {
                (None, None) => return true,
                (Some(x), Some(y)) => {
                    if !(x == y) {
                        return false;
                    }
                }
                _ => return false,
            }
        }
    }

    /// Determines if the elements of this iterator are equal to those of another with respect to the specified equality function.
    fn eq_by<I, F>(self, other: I, eq: F) -> bool
    where
        Self: Sized,
        I: IntoIterator,
        F: FnMut(Self::Item, I::Item) -> bool,
    {
        let mut a = self;
        let mut b = other.into_iter();
        let mut eq = eq;
        loop {
            match (a.next(), b.next()) {
                (None, None) => return true,
                (Some(x), Some(y)) => {
                    if !eq(x, y) {
                        return false;
                    }
                }
                _ => return false,
            }
        }
    }

    /// Determines if the elements of this iterator are not equal to those of another.
    fn ne<I>(self, other: I) -> bool
    where
        I: IntoIterator,
        Self::Item: PartialEq<I::Item>,
        Self: Sized,
    {
        !self.eq(other)
    }

    /// Determines if the elements of this iterator are lexicographically less than those of another.
    fn lt<I>(self, other: I) -> bool
    where
        I: IntoIterator,
        Self::Item: PartialOrd<I::Item>,
        Self: Sized,
    {
        self.partial_cmp(other) == Some(Ordering::Less)
    }

    /// Determines if the elements of this iterator are lexicographically less or equal to those of another.
    fn le<I>(self, other: I) -> bool
    where
        I: IntoIterator,
        Self::Item: PartialOrd<I::Item>,
        Self: Sized,
    {
        match self.partial_cmp(other) {
            Some(Ordering::Less) | Some(Ordering::Equal) => true,
            _ => false,
        }
    }

    /// Determines if the elements of this iterator are lexicographically greater than those of another.
    fn gt<I>(self, other: I) -> bool
    where
        I: IntoIterator,
        Self::Item: PartialOrd<I::Item>,
        Self: Sized,
    {
        self.partial_cmp(other) == Some(Ordering::Greater)
    }

    /// Determines if the elements of this iterator are lexicographically greater than or equal to those of another.
    fn ge<I>(self, other: I) -> bool
    where
        I: IntoIterator,
        Self::Item: PartialOrd<I::Item>,
        Self: Sized,
    {
        match self.partial_cmp(other) {
            Some(Ordering::Greater) | Some(Ordering::Equal) => true,
            _ => false,
        }
    }

    /// Checks if the elements of this iterator are sorted.
    fn is_sorted(self) -> bool
    where
        Self: Sized,
        Self::Item: PartialOrd,
    {
        self.is_sorted_by(|a, b| a <= b)
    }

    /// Checks if the elements of this iterator are sorted using the given comparator function.
    fn is_sorted_by<F>(self, compare: F) -> bool
    where
        Self: Sized,
        F: FnMut(&Self::Item, &Self::Item) -> bool,
    {
        let mut it = self;
        let mut compare = compare;
        let mut last = match it.next() {
            Some(e) => e,
            None => return true,
        };
        while let Some(curr) = it.next() {
            if !compare(&last, &curr) {
                return false;
            }
            last = curr;
        }
        true
    }

    /// Checks if the elements of this iterator are sorted using the given key extraction function.
    fn is_sorted_by_key<F, K>(self, f: F) -> bool
    where
        Self: Sized,
        F: FnMut(Self::Item) -> K,
        K: PartialOrd,
    {
        self.map(f).is_sorted()
    }
}

/// An iterator able to yield elements from both ends.
pub trait DoubleEndedIterator: Iterator {
    /// Removes and returns an element from the end of the iterator.
    fn next_back(&mut self) -> Option<Self::Item>;

    /// Advances the iterator from the back by `n` elements.
    #[inline]
    fn advance_back_by(&mut self, n: usize) -> Result<(), usize> {
        let mut i = 0;
        while i < n {
            if self.next_back().is_none() {
                return Err(n - i);
            }
            i += 1;
        }
        Ok(())
    }

    /// Returns the `n`th element from the end of the iterator.
    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let mut i = 0;
        while i < n {
            if self.next_back().is_none() {
                return None;
            }
            i += 1;
        }
        self.next_back()
    }

    /// The reverse version of `try_fold`.
    #[inline]
    fn try_rfold<B, F, R>(&mut self, init: B, f: F) -> R
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> R,
        R: Try<Output = B>,
    {
        let mut f = f;
        let mut accum = init;
        while let Some(x) = self.next_back() {
            match f(accum, x).branch() {
                ControlFlow::Continue(c) => accum = c,
                ControlFlow::Break(r) => return R::from_residual(r),
            }
        }
        R::from_output(accum)
    }

    /// An iterator method that reduces the iterator's elements to a single, final value, starting from the back.
    #[inline]
    fn rfold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        let mut it = self;
        let mut f = f;
        let mut accum = init;
        while let Some(x) = it.next_back() {
            accum = f(accum, x);
        }
        accum
    }

    /// Searches for an element of an iterator from the back that satisfies a predicate.
    #[inline]
    fn rfind<P>(&mut self, predicate: P) -> Option<Self::Item>
    where
        Self: Sized,
        P: FnMut(&Self::Item) -> bool,
    {
        let mut predicate = predicate;
        while let Some(x) = self.next_back() {
            if predicate(&x) {
                return Some(x);
            }
        }
        None
    }
}

/// An iterator that knows its exact length.
pub trait ExactSizeIterator: Iterator {
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        let (lower, _) = self.size_hint();
        lower
    }

    /// Returns `true` if the iterator is empty.
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// An iterator that always continues to yield `None` when exhausted.
pub trait FusedIterator: Iterator {}

/// An iterator that reports an accurate length using size_hint.
pub unsafe trait TrustedLen: Iterator {}

/// Conversion from an `Iterator`.
pub trait FromIterator<A>: Sized {
    /// Creates a value from an iterator.
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self;
}

/// Conversion into an `Iterator`.
#[lang = "into_iterator"]
pub trait IntoIterator {
    /// The type of the elements being iterated over.
    type Item;
    /// Which kind of iterator are we turning this into?
    type IntoIter: Iterator<Item = Self::Item>;
    /// Creates an iterator from a value.
    fn into_iter(self) -> Self::IntoIter;

    /// Bounds on the number of items of every value of this type (core's
    /// `ConstSizeIntoIterator`/`BoundedSize` specializations, used by
    /// `Flatten`/`FlatMap::size_hint`).
    #[doc(hidden)]
    fn __flatten_bounds() -> (usize, Option<usize>)
    where
        Self: Sized,
    {
        (0, None)
    }
}

impl<I: Iterator> IntoIterator for I {
    type Item = I::Item;
    type IntoIter = I;
    #[inline]
    fn into_iter(self) -> I {
        self
    }
}

/// Extend a collection with the contents of an iterator.
pub trait Extend<A> {
    /// Extends a collection with the contents of an iterator.
    fn extend<T: IntoIterator<Item = A>>(&mut self, iter: T);

    /// Extends a collection with exactly one element.
    fn extend_one(&mut self, item: A) {
        self.extend(Some(item));
    }

    /// Reserves capacity in a collection for the given number of additional elements.
    fn extend_reserve(&mut self, additional: usize) {
        let _ = additional;
    }
}

impl Extend<()> for () {
    fn extend<T: IntoIterator<Item = ()>>(&mut self, iter: T) {
        let mut it = iter.into_iter();
        while let Some(_) = it.next() {}
    }
    fn extend_one(&mut self, _item: ()) {}
}

impl FromIterator<()> for () {
    fn from_iter<I: IntoIterator<Item = ()>>(iter: I) -> () {
        let mut it = iter.into_iter();
        while let Some(_) = it.next() {}
    }
}

impl<A, B, ExtendA, ExtendB> Extend<(A, B)> for (ExtendA, ExtendB)
where
    ExtendA: Extend<A>,
    ExtendB: Extend<B>,
{
    fn extend<T: IntoIterator<Item = (A, B)>>(&mut self, into_iter: T) {
        let mut it = into_iter.into_iter();
        while let Some((a, b)) = it.next() {
            self.0.extend_one(a);
            self.1.extend_one(b);
        }
    }
    fn extend_one(&mut self, item: (A, B)) {
        self.0.extend_one(item.0);
        self.1.extend_one(item.1);
    }
}

impl<A, B, AE, BE> FromIterator<(AE, BE)> for (A, B)
where
    A: Default + Extend<AE>,
    B: Default + Extend<BE>,
{
    fn from_iter<I: IntoIterator<Item = (AE, BE)>>(iter: I) -> Self {
        let mut res: (A, B) = (Default::default(), Default::default());
        res.extend(iter);
        res
    }
}

// `&mut I` is an iterator too.
impl<I: Iterator + ?Sized> Iterator for &mut I {
    type Item = I::Item;
    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        (**self).next()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (**self).size_hint()
    }
    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        (**self).nth(n)
    }
}

impl<I: DoubleEndedIterator + ?Sized> DoubleEndedIterator for &mut I {
    #[inline]
    fn next_back(&mut self) -> Option<I::Item> {
        (**self).next_back()
    }
    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<I::Item> {
        (**self).nth_back(n)
    }
}

impl<I: ExactSizeIterator + ?Sized> ExactSizeIterator for &mut I {
    #[inline]
    fn len(&self) -> usize {
        (**self).len()
    }
}

impl<I: FusedIterator + ?Sized> FusedIterator for &mut I {}

/// Trait to represent types that can be created by summing up an iterator.
pub trait Sum<A = Self>: Sized {
    /// Takes an iterator and generates `Self` from the elements by "summing up" the items.
    fn sum<I: Iterator<Item = A>>(iter: I) -> Self;
}

/// Trait to represent types that can be created by multiplying elements of an iterator.
pub trait Product<A = Self>: Sized {
    /// Takes an iterator and generates `Self` from the elements by multiplying the items.
    fn product<I: Iterator<Item = A>>(iter: I) -> Self;
}

macro_rules! __std_iter_sum_product {
    ($zero:expr, $one:expr; $($t:ty)*) => {
        $(
            impl Sum for $t {
                #[inline]
                fn sum<I: Iterator<Item = $t>>(iter: I) -> $t {
                    let mut it = iter;
                    let mut acc: $t = $zero;
                    while let Some(x) = it.next() {
                        acc = acc + x;
                    }
                    acc
                }
            }
            impl<'a> Sum<&'a $t> for $t {
                #[inline]
                fn sum<I: Iterator<Item = &'a $t>>(iter: I) -> $t {
                    let mut it = iter;
                    let mut acc: $t = $zero;
                    while let Some(x) = it.next() {
                        acc = acc + *x;
                    }
                    acc
                }
            }
            impl Product for $t {
                #[inline]
                fn product<I: Iterator<Item = $t>>(iter: I) -> $t {
                    let mut it = iter;
                    let mut acc: $t = $one;
                    while let Some(x) = it.next() {
                        acc = acc * x;
                    }
                    acc
                }
            }
            impl<'a> Product<&'a $t> for $t {
                #[inline]
                fn product<I: Iterator<Item = &'a $t>>(iter: I) -> $t {
                    let mut it = iter;
                    let mut acc: $t = $one;
                    while let Some(x) = it.next() {
                        acc = acc * *x;
                    }
                    acc
                }
            }
        )*
    };
}

__std_iter_sum_product! { 0, 1; i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }
__std_iter_sum_product! { -0.0, 1.0; f32 f64 }

impl<T, U> Sum<Option<U>> for Option<T>
where
    T: Sum<U>,
{
    fn sum<I: Iterator<Item = Option<U>>>(iter: I) -> Option<T> {
        let mut failed = false;
        let v: T = {
            let shunt = SumShuntOption { iter, failed: &mut failed };
            Sum::sum(shunt)
        };
        if failed {
            None
        } else {
            Some(v)
        }
    }
}

impl<T, U> Product<Option<U>> for Option<T>
where
    T: Product<U>,
{
    fn product<I: Iterator<Item = Option<U>>>(iter: I) -> Option<T> {
        let mut failed = false;
        let v: T = {
            let shunt = SumShuntOption { iter, failed: &mut failed };
            Product::product(shunt)
        };
        if failed {
            None
        } else {
            Some(v)
        }
    }
}

impl<T, U, E> Sum<Result<U, E>> for Result<T, E>
where
    T: Sum<U>,
{
    fn sum<I: Iterator<Item = Result<U, E>>>(iter: I) -> Result<T, E> {
        let mut error: Option<E> = None;
        let v: T = {
            let shunt = SumShuntResult { iter, error: &mut error };
            Sum::sum(shunt)
        };
        match error {
            Some(e) => Err(e),
            None => Ok(v),
        }
    }
}

impl<T, U, E> Product<Result<U, E>> for Result<T, E>
where
    T: Product<U>,
{
    fn product<I: Iterator<Item = Result<U, E>>>(iter: I) -> Result<T, E> {
        let mut error: Option<E> = None;
        let v: T = {
            let shunt = SumShuntResult { iter, error: &mut error };
            Product::product(shunt)
        };
        match error {
            Some(e) => Err(e),
            None => Ok(v),
        }
    }
}

struct SumShuntOption<'a, I> {
    iter: I,
    failed: &'a mut bool,
}

impl<'a, A, I: Iterator<Item = Option<A>>> Iterator for SumShuntOption<'a, I> {
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

struct SumShuntResult<'a, I, E> {
    iter: I,
    error: &'a mut Option<E>,
}

impl<'a, A, E, I: Iterator<Item = Result<A, E>>> Iterator for SumShuntResult<'a, I, E> {
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
