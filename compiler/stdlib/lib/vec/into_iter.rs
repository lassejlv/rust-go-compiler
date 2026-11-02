//! The by-value iterator of `Vec<T>`.

use crate::fmt;
use crate::intrinsics;
use crate::iter::{DoubleEndedIterator, ExactSizeIterator, FusedIterator, Iterator, TrustedLen};
use crate::marker::PhantomData;
use crate::slice;

/// An iterator that moves out of a vector.
pub struct IntoIter<T> {
    buf: *mut T,
    cap: usize,
    start: usize,
    end: usize,
    _marker: PhantomData<T>,
}

impl<T> IntoIter<T> {
    #[inline]
    pub(crate) fn new(buf: *mut T, cap: usize, len: usize) -> IntoIter<T> {
        IntoIter { buf, cap, start: 0, end: len, _marker: PhantomData }
    }

    /// Returns the remaining items of this iterator as a slice.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(intrinsics::ptr_add(self.buf as *const T, self.start), self.end - self.start) }
    }

    /// Returns the remaining items of this iterator as a mutable slice.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(intrinsics::ptr_add_mut(self.buf, self.start), self.end - self.start) }
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        if self.start < self.end {
            let i = self.start;
            self.start = i + 1;
            unsafe { Some(intrinsics::read(intrinsics::ptr_add(self.buf as *const T, i))) }
        } else {
            None
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.end - self.start;
        (n, Some(n))
    }
    #[inline]
    fn count(self) -> usize {
        self.end - self.start
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        if self.start < self.end {
            self.end -= 1;
            unsafe { Some(intrinsics::read(intrinsics::ptr_add(self.buf as *const T, self.end))) }
        } else {
            None
        }
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {
    #[inline]
    fn len(&self) -> usize {
        self.end - self.start
    }
}

impl<T> FusedIterator for IntoIter<T> {}
unsafe impl<T> TrustedLen for IntoIter<T> {}

impl<T> Drop for IntoIter<T> {
    fn drop(&mut self) {
        unsafe {
            intrinsics::drop_in_place(self.as_mut_slice() as *mut [T]);
            if crate::mem::size_of::<T>() != 0 {
                crate::alloc::dealloc_array(self.buf, self.cap);
            }
        }
    }
}

impl<T: Clone> Clone for IntoIter<T> {
    fn clone(&self) -> IntoIter<T> {
        self.as_slice().to_vec().into_iter()
    }
}

impl<T> Default for IntoIter<T> {
    fn default() -> Self {
        Vec::new().into_iter()
    }
}

impl<T: fmt::Debug> fmt::Debug for IntoIter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoIter").field(&self.as_slice()).finish()
    }
}
