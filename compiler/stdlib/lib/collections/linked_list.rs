//! A doubly-linked list (implemented on top of `VecDeque`; the API and
//! observable behavior match the real `LinkedList`, except cursors).

use super::vec_deque::{self, VecDeque};
use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::iter::{DoubleEndedIterator, ExactSizeIterator, Extend, FromIterator, FusedIterator, IntoIterator, Iterator};

/// A doubly-linked list.
pub struct LinkedList<T> {
    inner: VecDeque<T>,
}

impl<T> LinkedList<T> {
    /// Creates an empty `LinkedList`.
    #[inline]
    pub const fn new() -> Self {
        LinkedList { inner: VecDeque::new() }
    }

    /// Moves all elements from `other` to the end of the list.
    pub fn append(&mut self, other: &mut Self) {
        self.inner.append(&mut other.inner);
    }

    /// Provides a forward iterator.
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter { inner: self.inner.iter() }
    }

    /// Provides a forward iterator with mutable references.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut { inner: self.inner.iter_mut() }
    }

    /// Returns `true` if the `LinkedList` is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the length of the `LinkedList`.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Removes all elements from the `LinkedList`.
    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear()
    }

    /// Returns `true` if the `LinkedList` contains an element equal to the given value.
    pub fn contains(&self, x: &T) -> bool
    where
        T: PartialEq<T>,
    {
        self.inner.contains(x)
    }

    /// Provides a reference to the front element, or `None` if the list is empty.
    #[inline]
    pub fn front(&self) -> Option<&T> {
        self.inner.front()
    }

    /// Provides a mutable reference to the front element, or `None` if the list is empty.
    #[inline]
    pub fn front_mut(&mut self) -> Option<&mut T> {
        self.inner.front_mut()
    }

    /// Provides a reference to the back element, or `None` if the list is empty.
    #[inline]
    pub fn back(&self) -> Option<&T> {
        self.inner.back()
    }

    /// Provides a mutable reference to the back element, or `None` if the list is empty.
    #[inline]
    pub fn back_mut(&mut self) -> Option<&mut T> {
        self.inner.back_mut()
    }

    /// Adds an element first in the list.
    #[inline]
    pub fn push_front(&mut self, elt: T) {
        self.inner.push_front(elt)
    }

    /// Removes the first element and returns it, or `None` if the list is empty.
    #[inline]
    pub fn pop_front(&mut self) -> Option<T> {
        self.inner.pop_front()
    }

    /// Appends an element to the back of a list.
    #[inline]
    pub fn push_back(&mut self, elt: T) {
        self.inner.push_back(elt)
    }

    /// Appends an element to the back of the list, returning a reference to it.
    pub fn push_back_mut(&mut self, elt: T) -> &mut T {
        self.inner.push_back_mut(elt)
    }

    /// Adds an element first in the list, returning a reference to it.
    pub fn push_front_mut(&mut self, elt: T) -> &mut T {
        self.inner.push_front_mut(elt)
    }

    /// Removes the last element from a list and returns it, or `None` if it is empty.
    #[inline]
    pub fn pop_back(&mut self) -> Option<T> {
        self.inner.pop_back()
    }

    /// Splits the list into two at the given index.
    pub fn split_off(&mut self, at: usize) -> LinkedList<T> {
        LinkedList { inner: self.inner.split_off(at) }
    }

    /// Removes the element at the given index and returns it.
    pub fn remove(&mut self, at: usize) -> T {
        match self.inner.remove(at) {
            Some(v) => v,
            None => crate::rt::panic_str("Cannot remove at an index outside of the list bounds", file!(), line!(), column!()),
        }
    }

    /// Retains only the elements specified by the predicate.
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.inner.retain(f)
    }

    /// Creates an iterator which uses a closure to determine if an element should be removed.
    /// Elements not visited (if the iterator is dropped early) stay in the list.
    pub fn extract_if<F>(&mut self, filter: F) -> ExtractIf<'_, T, F>
    where
        F: FnMut(&mut T) -> bool,
    {
        ExtractIf { list: self, idx: 0, pred: filter }
    }
}

/// An iterator produced by calling `extract_if` on `LinkedList`.
pub struct ExtractIf<'a, T: 'a, F: 'a> {
    list: &'a mut LinkedList<T>,
    idx: usize,
    pred: F,
}

impl<'a, T, F> Iterator for ExtractIf<'a, T, F>
where
    F: FnMut(&mut T) -> bool,
{
    type Item = T;
    fn next(&mut self) -> Option<T> {
        while self.idx < self.list.inner.len() {
            let hit = match self.list.inner.get_mut(self.idx) {
                Some(x) => (self.pred)(x),
                None => false,
            };
            if hit {
                return self.list.inner.remove(self.idx);
            }
            self.idx += 1;
        }
        None
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.list.inner.len() - self.idx))
    }
}

impl<'a, T: fmt::Debug, F> fmt::Debug for ExtractIf<'a, T, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ExtractIf").field(&*self.list).finish()
    }
}

impl<T> Default for LinkedList<T> {
    fn default() -> Self {
        LinkedList::new()
    }
}

impl<T: Clone> Clone for LinkedList<T> {
    fn clone(&self) -> Self {
        LinkedList { inner: self.inner.clone() }
    }
}

impl<T: fmt::Debug> fmt::Debug for LinkedList<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T: PartialEq> PartialEq for LinkedList<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T: Eq> Eq for LinkedList<T> {}

impl<T: PartialOrd> PartialOrd for LinkedList<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<T: Ord> Ord for LinkedList<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl<T: Hash> Hash for LinkedList<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state)
    }
}

impl<T> FromIterator<T> for LinkedList<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        LinkedList { inner: VecDeque::from_iter(iter) }
    }
}

impl<T> Extend<T> for LinkedList<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.inner.extend(iter)
    }
}

impl<'a, T: 'a + Copy> Extend<&'a T> for LinkedList<T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.inner.extend(iter)
    }
}

impl<T, const N: usize> From<[T; N]> for LinkedList<T> {
    fn from(arr: [T; N]) -> Self {
        LinkedList { inner: VecDeque::from(arr) }
    }
}

impl<T> IntoIterator for LinkedList<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> {
        IntoIter { inner: self.inner.into_iter() }
    }
}

impl<'a, T> IntoIterator for &'a LinkedList<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut LinkedList<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;
    fn into_iter(self) -> IterMut<'a, T> {
        self.iter_mut()
    }
}

/// An iterator over the elements of a `LinkedList`.
pub struct Iter<'a, T: 'a> {
    inner: vec_deque::Iter<'a, T>,
}

impl<'a, T> Clone for Iter<'a, T> {
    fn clone(&self) -> Self {
        Iter { inner: self.inner.clone() }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<&'a T> {
        self.inner.next_back()
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {}
impl<'a, T> FusedIterator for Iter<'a, T> {}

/// Debug-prints the remaining elements of a list iterator as a list.
struct RemainingList<'a, T>(&'a [T], &'a [T]);

impl<'a, T: fmt::Debug> fmt::Debug for RemainingList<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.0.iter()).entries(self.1.iter()).finish()
    }
}

impl<'a, T: fmt::Debug> fmt::Debug for Iter<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Rest<'a, T>(vec_deque::Iter<'a, T>);
        impl<'a, T: fmt::Debug> fmt::Debug for Rest<'a, T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.0.clone()).finish()
            }
        }
        f.debug_tuple("Iter").field(&Rest(self.inner.clone())).field(&self.inner.len()).finish()
    }
}

impl<'a, T: fmt::Debug> fmt::Debug for IterMut<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (a, b) = self.inner.remaining();
        f.debug_tuple("IterMut").field(&RemainingList(a, b)).field(&(a.len() + b.len())).finish()
    }
}

impl<T: Clone> Clone for IntoIter<T> {
    fn clone(&self) -> Self {
        IntoIter { inner: self.inner.clone() }
    }
}

impl<T: fmt::Debug> fmt::Debug for IntoIter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoIter").field(self.inner.as_deque()).finish()
    }
}

/// A mutable iterator over the elements of a `LinkedList`.
pub struct IterMut<'a, T: 'a> {
    inner: vec_deque::IterMut<'a, T>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<&'a mut T> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    fn next_back(&mut self) -> Option<&'a mut T> {
        self.inner.next_back()
    }
}

impl<'a, T> ExactSizeIterator for IterMut<'a, T> {}

/// An owning iterator over the elements of a `LinkedList`.
pub struct IntoIter<T> {
    inner: vec_deque::IntoIter<T>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        self.inner.next_back()
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {}

impl<'a, T> Default for Iter<'a, T> {
    fn default() -> Self {
        Iter { inner: Default::default() }
    }
}

impl<'a, T> Default for IterMut<'a, T> {
    fn default() -> Self {
        IterMut { inner: Default::default() }
    }
}

impl<T> Default for IntoIter<T> {
    fn default() -> Self {
        IntoIter { inner: VecDeque::new().into_iter() }
    }
}
