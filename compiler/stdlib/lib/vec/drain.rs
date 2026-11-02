//! The draining iterator of `Vec<T>`.

use crate::fmt;
use crate::intrinsics;
use crate::iter::{DoubleEndedIterator, ExactSizeIterator, FusedIterator, Iterator};
use crate::marker::PhantomData;
use crate::slice;

/// A draining iterator for `Vec<T>`.
pub struct Drain<'a, T: 'a> {
    vec: *mut Vec<T>,
    /// Next index to yield from the front.
    idx: usize,
    /// One past the last index to yield from the back.
    end: usize,
    /// Start of the tail to preserve.
    tail_start: usize,
    /// Length of the tail to preserve.
    tail_len: usize,
    _marker: PhantomData<&'a mut Vec<T>>,
}

impl<'a, T> Drain<'a, T> {
    pub(crate) fn new(vec: &'a mut Vec<T>, start: usize, end: usize) -> Drain<'a, T> {
        let len = vec.len();
        unsafe {
            vec.set_len(start);
        }
        Drain { vec: vec as *mut Vec<T>, idx: start, end, tail_start: end, tail_len: len - end, _marker: PhantomData }
    }

    #[inline]
    fn base(&self) -> *mut T {
        unsafe { (*self.vec).as_mut_ptr() }
    }

    /// Returns the remaining items of this iterator as a slice.
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(intrinsics::ptr_add(self.base() as *const T, self.idx), self.end - self.idx) }
    }

    /// Keep unyielded elements in the source `Vec`.
    pub fn keep_rest(self) {
        let mut this = crate::mem::ManuallyDrop::new(self);
        unsafe {
            let vec = &mut *this.vec;
            let start = vec.len();
            let base = vec.as_mut_ptr();
            let unyielded = this.end - this.idx;
            if this.idx != start {
                intrinsics::copy(intrinsics::ptr_add(base as *const T, this.idx), intrinsics::ptr_add_mut(base, start), unyielded);
            }
            let new_tail = start + unyielded;
            if this.tail_start != new_tail {
                intrinsics::copy(
                    intrinsics::ptr_add(base as *const T, this.tail_start),
                    intrinsics::ptr_add_mut(base, new_tail),
                    this.tail_len,
                );
            }
            vec.set_len(new_tail + this.tail_len);
            this.idx = this.end;
        }
    }
}

impl<'a, T> Iterator for Drain<'a, T> {
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        if self.idx < self.end {
            let i = self.idx;
            self.idx = i + 1;
            unsafe { Some(intrinsics::read(intrinsics::ptr_add(self.base() as *const T, i))) }
        } else {
            None
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.end - self.idx;
        (n, Some(n))
    }
}

impl<'a, T> DoubleEndedIterator for Drain<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        if self.idx < self.end {
            self.end -= 1;
            unsafe { Some(intrinsics::read(intrinsics::ptr_add(self.base() as *const T, self.end))) }
        } else {
            None
        }
    }
}

impl<'a, T> ExactSizeIterator for Drain<'a, T> {}
impl<'a, T> FusedIterator for Drain<'a, T> {}

impl<'a, T> Drop for Drain<'a, T> {
    fn drop(&mut self) {
        unsafe {
            let base = self.base();
            // Drop the elements that were not yielded.
            if self.idx < self.end {
                let rest = slice::from_raw_parts_mut(intrinsics::ptr_add_mut(base, self.idx), self.end - self.idx);
                intrinsics::drop_in_place(rest as *mut [T]);
            }
            // Move the tail back.
            let vec = &mut *self.vec;
            let start = vec.len();
            if self.tail_len > 0 && self.tail_start != start {
                intrinsics::copy(
                    intrinsics::ptr_add(base as *const T, self.tail_start),
                    intrinsics::ptr_add_mut(base, start),
                    self.tail_len,
                );
            }
            vec.set_len(start + self.tail_len);
        }
    }
}

impl<'a, T: fmt::Debug> fmt::Debug for Drain<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain").field(&self.as_slice()).finish()
    }
}
