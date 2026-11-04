//! Functions creating iterators.

use super::adapters::{Chain, Zip};
use super::traits::{DoubleEndedIterator, ExactSizeIterator, FusedIterator, IntoIterator, Iterator};
use crate::fmt;
use crate::marker::PhantomData;

/// An iterator that yields nothing.
#[must_use]
pub struct Empty<T>(PhantomData<T>);

/// Creates an iterator that yields nothing.
pub const fn empty<T>() -> Empty<T> {
    Empty(PhantomData)
}

impl<T> Iterator for Empty<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        None
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(0))
    }
}

impl<T> DoubleEndedIterator for Empty<T> {
    fn next_back(&mut self) -> Option<T> {
        None
    }
}

impl<T> ExactSizeIterator for Empty<T> {
    fn len(&self) -> usize {
        0
    }
}

impl<T> FusedIterator for Empty<T> {}

impl<T> Clone for Empty<T> {
    fn clone(&self) -> Empty<T> {
        Empty(PhantomData)
    }
}

impl<T> Default for Empty<T> {
    fn default() -> Empty<T> {
        Empty(PhantomData)
    }
}

impl<T> fmt::Debug for Empty<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Empty")
    }
}

/// An iterator that yields an element exactly once.
#[derive(Clone)]
#[must_use]
pub struct Once<T> {
    inner: Option<T>,
}

impl<T: fmt::Debug> fmt::Debug for Once<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // std: Once { inner: option::IntoIter { inner: Item { opt } } }
        struct IntoIterDebug<'a, T>(&'a Option<T>);
        impl<'a, T: fmt::Debug> fmt::Debug for IntoIterDebug<'a, T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("IntoIter").field("inner", &crate::option::ItemDebug(self.0)).finish()
            }
        }
        f.debug_struct("Once").field("inner", &IntoIterDebug(&self.inner)).finish()
    }
}

/// Creates an iterator that yields an element exactly once.
pub fn once<T>(value: T) -> Once<T> {
    Once { inner: Some(value) }
}

impl<T> Iterator for Once<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.inner.take()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = if self.inner.is_some() { 1 } else { 0 };
        (n, Some(n))
    }
}

impl<T> DoubleEndedIterator for Once<T> {
    fn next_back(&mut self) -> Option<T> {
        self.inner.take()
    }
}

impl<T> ExactSizeIterator for Once<T> {}
impl<T> FusedIterator for Once<T> {}

/// An iterator that yields a single element of type `A` by applying the provided closure `F: FnOnce() -> A`.
#[derive(Clone)]
#[must_use]
pub struct OnceWith<F> {
    make: Option<F>,
}

impl<F> fmt::Debug for OnceWith<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.make.is_some() { f.write_str("OnceWith(Some(_))") } else { f.write_str("OnceWith(None)") }
    }
}

/// Creates an iterator that lazily generates a value exactly once by invoking the provided closure.
pub fn once_with<A, F: FnOnce() -> A>(make: F) -> OnceWith<F> {
    OnceWith { make: Some(make) }
}

impl<A, F: FnOnce() -> A> Iterator for OnceWith<F> {
    type Item = A;
    fn next(&mut self) -> Option<A> {
        match self.make.take() {
            Some(f) => Some(f()),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = if self.make.is_some() { 1 } else { 0 };
        (n, Some(n))
    }
}

/// An iterator that repeats an element endlessly.
#[derive(Clone, Debug)]
#[must_use]
pub struct Repeat<A> {
    element: A,
}

/// Creates a new iterator that endlessly repeats a single element.
pub fn repeat<T: Clone>(elt: T) -> Repeat<T> {
    Repeat { element: elt }
}

impl<A: Clone> Iterator for Repeat<A> {
    type Item = A;
    #[inline]
    fn next(&mut self) -> Option<A> {
        Some(self.element.clone())
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (usize::MAX, None)
    }
    #[inline]
    fn nth(&mut self, _n: usize) -> Option<A> {
        Some(self.element.clone())
    }
}

impl<A: Clone> DoubleEndedIterator for Repeat<A> {
    #[inline]
    fn next_back(&mut self) -> Option<A> {
        Some(self.element.clone())
    }
}

impl<A: Clone> FusedIterator for Repeat<A> {}

/// An iterator that repeats an element an exact number of times.
#[derive(Clone, Debug)]
#[must_use]
pub struct RepeatN<A> {
    element: Option<A>,
    count: usize,
}

/// Creates a new iterator that repeats a single element a given number of times.
pub fn repeat_n<T: Clone>(element: T, count: usize) -> RepeatN<T> {
    if count == 0 {
        RepeatN { element: None, count: 0 }
    } else {
        RepeatN { element: Some(element), count }
    }
}

impl<A: Clone> Iterator for RepeatN<A> {
    type Item = A;
    #[inline]
    fn next(&mut self) -> Option<A> {
        if self.count == 0 {
            return None;
        }
        self.count -= 1;
        if self.count == 0 {
            self.element.take()
        } else {
            match &self.element {
                Some(e) => Some(e.clone()),
                None => None,
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count, Some(self.count))
    }
}

impl<A: Clone> DoubleEndedIterator for RepeatN<A> {
    #[inline]
    fn next_back(&mut self) -> Option<A> {
        self.next()
    }
}

impl<A: Clone> ExactSizeIterator for RepeatN<A> {
    fn len(&self) -> usize {
        self.count
    }
}

impl<A: Clone> FusedIterator for RepeatN<A> {}

/// An iterator that repeats elements of type `A` endlessly by applying the provided closure.
#[derive(Clone)]
#[must_use]
pub struct RepeatWith<F> {
    repeater: F,
}

impl<F> fmt::Debug for RepeatWith<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RepeatWith").finish_non_exhaustive()
    }
}

/// Creates a new iterator that repeats elements endlessly by applying the provided closure.
pub fn repeat_with<A, F: FnMut() -> A>(repeater: F) -> RepeatWith<F> {
    RepeatWith { repeater }
}

impl<A, F: FnMut() -> A> Iterator for RepeatWith<F> {
    type Item = A;
    #[inline]
    fn next(&mut self) -> Option<A> {
        Some((self.repeater)())
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (usize::MAX, None)
    }
}

/// An iterator where each iteration calls the provided closure `F: FnMut() -> Option<T>`.
#[derive(Clone)]
#[must_use]
pub struct FromFn<F>(F);

/// Creates an iterator where each iteration calls the provided closure.
#[inline]
pub fn from_fn<T, F>(f: F) -> FromFn<F>
where
    F: FnMut() -> Option<T>,
{
    FromFn(f)
}

impl<T, F> Iterator for FromFn<F>
where
    F: FnMut() -> Option<T>,
{
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        (self.0)()
    }
}

impl<F> fmt::Debug for FromFn<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FromFn")
    }
}

/// An iterator where each successive item is computed based on the preceding one.
#[derive(Clone)]
#[must_use]
pub struct Successors<T, F> {
    next: Option<T>,
    succ: F,
}

/// Creates a new iterator where each successive item is computed based on the preceding one.
pub fn successors<T, F>(first: Option<T>, succ: F) -> Successors<T, F>
where
    F: FnMut(&T) -> Option<T>,
{
    Successors { next: first, succ }
}

impl<T, F> Iterator for Successors<T, F>
where
    F: FnMut(&T) -> Option<T>,
{
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        let item = match self.next.take() {
            Some(item) => item,
            None => return None,
        };
        self.next = (self.succ)(&item);
        Some(item)
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.next.is_some() {
            (1, None)
        } else {
            (0, Some(0))
        }
    }
}

impl<T, F> FusedIterator for Successors<T, F> where F: FnMut(&T) -> Option<T> {}

impl<T: fmt::Debug, F> fmt::Debug for Successors<T, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Successors").field("next", &self.next).finish()
    }
}

/// Converts the arguments to iterators and zips them.
pub fn zip<A, B>(a: A, b: B) -> Zip<A::IntoIter, B::IntoIter>
where
    A: IntoIterator,
    B: IntoIterator,
{
    a.into_iter().zip(b)
}

/// Converts the arguments to iterators and links them together, in a chain.
pub fn chain<A, B>(a: A, b: B) -> Chain<A::IntoIter, B::IntoIter>
where
    A: IntoIterator,
    B: IntoIterator<Item = A::Item>,
{
    a.into_iter().chain(b)
}
