//! A priority queue implemented with a binary heap (same algorithms as the
//! real standard library, so element order matches).

use crate::fmt;
use crate::intrinsics;
use crate::iter::{Extend, FromIterator, IntoIterator, Iterator};
use crate::mem;
use crate::ops::{Deref, DerefMut};
use crate::slice;
use crate::vec;

/// An iterator over the elements of a `BinaryHeap` (in internal order).
pub type Iter<'a, T> = slice::Iter<'a, T>;
/// An owning iterator over the elements of a `BinaryHeap` (in internal order).
pub type IntoIter<T> = vec::IntoIter<T>;
/// A draining iterator over the elements of a `BinaryHeap`.
pub type Drain<'a, T> = vec::Drain<'a, T>;

#[inline(always)]
fn log2_fast(x: usize) -> usize {
    (usize::BITS - x.leading_zeros() - 1) as usize
}

/// A priority queue implemented with a binary heap (a max-heap).
pub struct BinaryHeap<T> {
    data: Vec<T>,
}

impl<T: Ord> BinaryHeap<T> {
    /// Returns a mutable reference to the greatest item in the binary heap, or `None` if it is empty.
    pub fn peek_mut(&mut self) -> Option<PeekMut<'_, T>> {
        if self.is_empty() {
            None
        } else {
            Some(PeekMut { heap: self })
        }
    }

    /// Removes the greatest item from the binary heap and returns it, or `None` if it is empty.
    pub fn pop(&mut self) -> Option<T> {
        match self.data.pop() {
            None => None,
            Some(mut item) => {
                if !self.is_empty() {
                    mem::swap(&mut item, &mut self.data[0]);
                    self.sift_down_to_bottom(0);
                }
                Some(item)
            }
        }
    }

    /// Pushes an item onto the binary heap.
    pub fn push(&mut self, item: T) {
        let old_len = self.len();
        self.data.push(item);
        self.sift_up(0, old_len);
    }

    /// Consumes the `BinaryHeap` and returns a vector in sorted (ascending) order.
    pub fn into_sorted_vec(self) -> Vec<T> {
        let mut me = self;
        let mut end = me.len();
        while end > 1 {
            end -= 1;
            me.data.swap(0, end);
            me.sift_down_range(0, end);
        }
        me.into_vec()
    }

    // Take out the element at `pos` and move it up the heap while it is
    // greater than its parent.
    fn sift_up(&mut self, start: usize, pos: usize) -> usize {
        unsafe {
            let p = self.data.as_mut_ptr();
            let elem = intrinsics::read(p.add(pos) as *const T);
            let mut hole = pos;
            while hole > start {
                let parent = (hole - 1) / 2;
                if elem <= *p.add(parent) {
                    break;
                }
                intrinsics::copy_nonoverlapping(p.add(parent) as *const T, p.add(hole), 1);
                hole = parent;
            }
            intrinsics::write(p.add(hole), elem);
            hole
        }
    }

    // Take an element at `pos` and move it down the heap, while its children
    // are larger. Returns whether the element moved.
    fn sift_down_range(&mut self, pos: usize, end: usize) -> bool {
        unsafe {
            let p = self.data.as_mut_ptr();
            let elem = intrinsics::read(p.add(pos) as *const T);
            let mut hole = pos;
            let mut child = 2 * hole + 1;
            while child <= end.saturating_sub(2) {
                if *p.add(child) <= *p.add(child + 1) {
                    child += 1;
                }
                if elem >= *p.add(child) {
                    intrinsics::write(p.add(hole), elem);
                    return hole != pos;
                }
                intrinsics::copy_nonoverlapping(p.add(child) as *const T, p.add(hole), 1);
                hole = child;
                child = 2 * hole + 1;
            }
            if child == end - 1 && elem < *p.add(child) {
                intrinsics::copy_nonoverlapping(p.add(child) as *const T, p.add(hole), 1);
                hole = child;
            }
            intrinsics::write(p.add(hole), elem);
            hole != pos
        }
    }

    fn sift_down(&mut self, pos: usize) -> bool {
        let len = self.len();
        self.sift_down_range(pos, len)
    }

    // Take an element at `pos` and move it all the way down the heap, then
    // sift it up to its position.
    fn sift_down_to_bottom(&mut self, pos: usize) {
        let end = self.len();
        let start = pos;
        let hole_end;
        unsafe {
            let p = self.data.as_mut_ptr();
            let elem = intrinsics::read(p.add(pos) as *const T);
            let mut hole = pos;
            let mut child = 2 * hole + 1;
            while child <= end.saturating_sub(2) {
                if *p.add(child) <= *p.add(child + 1) {
                    child += 1;
                }
                intrinsics::copy_nonoverlapping(p.add(child) as *const T, p.add(hole), 1);
                hole = child;
                child = 2 * hole + 1;
            }
            if child == end - 1 {
                intrinsics::copy_nonoverlapping(p.add(child) as *const T, p.add(hole), 1);
                hole = child;
            }
            intrinsics::write(p.add(hole), elem);
            hole_end = hole;
        }
        self.sift_up(start, hole_end);
    }

    fn rebuild_tail(&mut self, start: usize) {
        if start == self.len() {
            return;
        }
        let tail_len = self.len() - start;
        let better_to_rebuild = if start < tail_len {
            true
        } else if self.len() <= 2048 {
            2 * self.len() < tail_len * log2_fast(start)
        } else {
            2 * self.len() < tail_len * 11
        };
        if better_to_rebuild {
            self.rebuild();
        } else {
            let mut i = start;
            while i < self.len() {
                self.sift_up(0, i);
                i += 1;
            }
        }
    }

    fn rebuild(&mut self) {
        let mut n = self.len() / 2;
        while n > 0 {
            n -= 1;
            self.sift_down(n);
        }
    }

    /// Moves all the elements of `other` into `self`, leaving `other` empty.
    pub fn append(&mut self, other: &mut Self) {
        if self.len() < other.len() {
            mem::swap(self, other);
        }
        let start = self.data.len();
        self.data.append(&mut other.data);
        self.rebuild_tail(start);
    }

    /// Retains only the elements specified by the predicate.
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        let mut f = f;
        let mut first_removed = self.len();
        let mut i = 0;
        self.data.retain(|e| {
            let keep = f(e);
            if !keep && i < first_removed {
                first_removed = i;
            }
            i += 1;
            keep
        });
        if first_removed < self.data.len() {
            self.rebuild_tail(first_removed);
        }
    }
}

impl<T> BinaryHeap<T> {
    /// Creates an empty `BinaryHeap` as a max-heap.
    #[inline]
    pub const fn new() -> BinaryHeap<T> {
        BinaryHeap { data: Vec::new() }
    }

    /// Creates an empty `BinaryHeap` with at least the specified capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> BinaryHeap<T> {
        BinaryHeap { data: Vec::with_capacity(capacity) }
    }

    /// Clears the binary heap, returning an iterator over the removed elements in heap order.
    pub fn drain(&mut self) -> vec::Drain<'_, T> {
        self.data.drain(..)
    }

    /// Returns an iterator visiting all values in the underlying vector, in arbitrary order.
    pub fn iter(&self) -> slice::Iter<'_, T> {
        self.data.iter()
    }

    /// Returns the greatest item in the binary heap, or `None` if it is empty.
    #[inline]
    pub fn peek(&self) -> Option<&T> {
        self.data.first()
    }

    /// Returns the number of elements the binary heap can hold without reallocating.
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Reserves capacity for at least `additional` elements more than the current length.
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }

    /// Shrinks the capacity with a lower bound.
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.data.shrink_to(min_capacity)
    }

    /// Tries to reserve capacity for at least `additional` more elements.
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), crate::collections::TryReserveError> {
        self.data.try_reserve(additional)
    }

    /// Tries to reserve the minimum capacity for at least `additional` more elements.
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), crate::collections::TryReserveError> {
        self.data.try_reserve_exact(additional)
    }

    /// Reserves the minimum capacity for at least `additional` elements more than the current length.
    pub fn reserve_exact(&mut self, additional: usize) {
        self.data.reserve_exact(additional);
    }

    /// Discards as much additional capacity as possible.
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }

    /// Returns a slice of all values in the underlying vector, in arbitrary order.
    pub fn as_slice(&self) -> &[T] {
        self.data.as_slice()
    }

    /// Consumes the `BinaryHeap` and returns the underlying vector in arbitrary order.
    pub fn into_vec(self) -> Vec<T> {
        self.data
    }

    /// Returns the length of the binary heap.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Checks if the binary heap is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Drops all items from the binary heap.
    pub fn clear(&mut self) {
        self.data.clear();
    }
}

/// Structure wrapping a mutable reference to the greatest item on a `BinaryHeap`.
pub struct PeekMut<'a, T: 'a + Ord> {
    heap: &'a mut BinaryHeap<T>,
}

impl<'a, T: Ord> PeekMut<'a, T> {
    /// Removes the peeked value from the heap and returns it.
    pub fn pop(this: PeekMut<'a, T>) -> T {
        let heap = this.heap as *mut BinaryHeap<T>;
        mem::forget(this);
        unsafe { (*heap).pop().unwrap() }
    }
}

impl<'a, T: Ord> Drop for PeekMut<'a, T> {
    fn drop(&mut self) {
        if !self.heap.is_empty() {
            self.heap.sift_down(0);
        }
    }
}

impl<'a, T: Ord> Deref for PeekMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.heap.data[0]
    }
}

impl<'a, T: Ord> DerefMut for PeekMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.heap.data[0]
    }
}

impl<'a, T: Ord + fmt::Debug> fmt::Debug for PeekMut<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PeekMut").field(&self.heap.data[0]).finish()
    }
}

impl<T: Clone> Clone for BinaryHeap<T> {
    fn clone(&self) -> Self {
        BinaryHeap { data: self.data.clone() }
    }
}

impl<T: Ord> Default for BinaryHeap<T> {
    #[inline]
    fn default() -> BinaryHeap<T> {
        BinaryHeap::new()
    }
}

impl<T: fmt::Debug> fmt::Debug for BinaryHeap<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.data.iter()).finish()
    }
}

impl<T: Ord> From<Vec<T>> for BinaryHeap<T> {
    fn from(vec: Vec<T>) -> BinaryHeap<T> {
        let mut heap = BinaryHeap { data: vec };
        heap.rebuild();
        heap
    }
}

impl<T: Ord, const N: usize> From<[T; N]> for BinaryHeap<T> {
    fn from(arr: [T; N]) -> Self {
        BinaryHeap::from(Vec::from(arr))
    }
}

impl<T> From<BinaryHeap<T>> for Vec<T> {
    fn from(heap: BinaryHeap<T>) -> Vec<T> {
        heap.data
    }
}

impl<T: Ord> FromIterator<T> for BinaryHeap<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> BinaryHeap<T> {
        let v: Vec<T> = FromIterator::from_iter(iter);
        BinaryHeap::from(v)
    }
}

impl<T> IntoIterator for BinaryHeap<T> {
    type Item = T;
    type IntoIter = vec::IntoIter<T>;
    fn into_iter(self) -> vec::IntoIter<T> {
        self.data.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a BinaryHeap<T> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;
    fn into_iter(self) -> slice::Iter<'a, T> {
        self.iter()
    }
}

impl<T: Ord> Extend<T> for BinaryHeap<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let start = self.data.len();
        self.data.extend(iter);
        self.rebuild_tail(start);
    }
    #[inline]
    fn extend_one(&mut self, item: T) {
        self.push(item);
    }
}

impl<'a, T: 'a + Ord + Copy> Extend<&'a T> for BinaryHeap<T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        let start = self.data.len();
        self.data.extend(iter);
        self.rebuild_tail(start);
    }
}
