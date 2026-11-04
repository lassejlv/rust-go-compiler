//! Iterators over slices.

use crate::fmt;
use crate::intrinsics;
use crate::iter::{DoubleEndedIterator, ExactSizeIterator, FusedIterator, IntoIterator, Iterator, TrustedLen};
use crate::marker::PhantomData;

use super::{from_raw_parts, from_raw_parts_mut};

/// Immutable slice iterator.
#[must_use]
pub struct Iter<'a, T: 'a> {
    ptr: *const T,
    start: usize,
    end: usize,
    _marker: PhantomData<&'a T>,
}

impl<'a, T> Iter<'a, T> {
    #[inline]
    pub(crate) fn new(slice: &'a [T]) -> Iter<'a, T> {
        Iter { ptr: slice.as_ptr(), start: 0, end: slice.len(), _marker: PhantomData }
    }

    /// Views the underlying data as a subslice of the original data.
    #[inline]
    pub fn as_slice(&self) -> &'a [T] {
        unsafe { from_raw_parts(intrinsics::ptr_add(self.ptr, self.start), self.end - self.start) }
    }
}

impl<'a, T> Clone for Iter<'a, T> {
    #[inline]
    fn clone(&self) -> Self {
        Iter { ptr: self.ptr, start: self.start, end: self.end, _marker: PhantomData }
    }
}

impl<'a, T: fmt::Debug> fmt::Debug for Iter<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Iter").field(&self.as_slice()).finish()
    }
}

impl<'a, T> Default for Iter<'a, T> {
    fn default() -> Self {
        Iter { ptr: crate::ptr::dangling::<T>(), start: 0, end: 0, _marker: PhantomData }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    #[inline]
    fn next(&mut self) -> Option<&'a T> {
        if self.start < self.end {
            let i = self.start;
            self.start = i + 1;
            unsafe { Some(&*intrinsics::ptr_add(self.ptr, i)) }
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
    #[inline]
    fn nth(&mut self, n: usize) -> Option<&'a T> {
        if n < self.end - self.start {
            let i = self.start + n;
            self.start = i + 1;
            unsafe { Some(&*intrinsics::ptr_add(self.ptr, i)) }
        } else {
            self.start = self.end;
            None
        }
    }
    #[inline]
    fn last(self) -> Option<&'a T> {
        let mut s = self;
        s.next_back()
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a T> {
        if self.start < self.end {
            self.end -= 1;
            unsafe { Some(&*intrinsics::ptr_add(self.ptr, self.end)) }
        } else {
            None
        }
    }
    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<&'a T> {
        if n < self.end - self.start {
            self.end -= n + 1;
            unsafe { Some(&*intrinsics::ptr_add(self.ptr, self.end)) }
        } else {
            self.end = self.start;
            None
        }
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    #[inline]
    fn len(&self) -> usize {
        self.end - self.start
    }
}

impl<'a, T> FusedIterator for Iter<'a, T> {}
unsafe impl<'a, T> TrustedLen for Iter<'a, T> {}

impl<'a, T> AsRef<[T]> for Iter<'a, T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

/// Mutable slice iterator.
#[must_use]
pub struct IterMut<'a, T: 'a> {
    ptr: *mut T,
    start: usize,
    end: usize,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T> IterMut<'a, T> {
    #[inline]
    pub(crate) fn new(slice: &'a mut [T]) -> IterMut<'a, T> {
        let len = slice.len();
        IterMut { ptr: slice.as_mut_ptr(), start: 0, end: len, _marker: PhantomData }
    }

    /// Views the underlying data as a subslice of the original data.
    #[inline]
    pub fn into_slice(self) -> &'a mut [T] {
        unsafe { from_raw_parts_mut(intrinsics::ptr_add_mut(self.ptr, self.start), self.end - self.start) }
    }

    /// Views the underlying data as a subslice of the original data.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        unsafe { from_raw_parts(intrinsics::ptr_add(self.ptr as *const T, self.start), self.end - self.start) }
    }
}

impl<'a, T> Default for IterMut<'a, T> {
    fn default() -> Self {
        IterMut { ptr: crate::ptr::dangling_mut::<T>(), start: 0, end: 0, _marker: PhantomData }
    }
}

impl<'a, T: fmt::Debug> fmt::Debug for IterMut<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IterMut").field(&self.as_slice()).finish()
    }
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    #[inline]
    fn next(&mut self) -> Option<&'a mut T> {
        if self.start < self.end {
            let i = self.start;
            self.start = i + 1;
            unsafe { Some(&mut *intrinsics::ptr_add_mut(self.ptr, i)) }
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
    #[inline]
    fn nth(&mut self, n: usize) -> Option<&'a mut T> {
        if n < self.end - self.start {
            let i = self.start + n;
            self.start = i + 1;
            unsafe { Some(&mut *intrinsics::ptr_add_mut(self.ptr, i)) }
        } else {
            self.start = self.end;
            None
        }
    }
    #[inline]
    fn last(self) -> Option<&'a mut T> {
        let mut s = self;
        s.next_back()
    }
}

impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a mut T> {
        if self.start < self.end {
            self.end -= 1;
            unsafe { Some(&mut *intrinsics::ptr_add_mut(self.ptr, self.end)) }
        } else {
            None
        }
    }
    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<&'a mut T> {
        if n < self.end - self.start {
            self.end -= n + 1;
            unsafe { Some(&mut *intrinsics::ptr_add_mut(self.ptr, self.end)) }
        } else {
            self.end = self.start;
            None
        }
    }
}

impl<'a, T> ExactSizeIterator for IterMut<'a, T> {
    #[inline]
    fn len(&self) -> usize {
        self.end - self.start
    }
}

impl<'a, T> FusedIterator for IterMut<'a, T> {}
unsafe impl<'a, T> TrustedLen for IterMut<'a, T> {}

// ---------------------------------------------------------------------------
// IntoIterator for slices

impl<'a, T> IntoIterator for &'a [T] {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    #[inline]
    fn into_iter(self) -> Iter<'a, T> {
        Iter::new(self)
    }
}

impl<'a, T> IntoIterator for &'a mut [T] {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;
    #[inline]
    fn into_iter(self) -> IterMut<'a, T> {
        IterMut::new(self)
    }
}

// ---------------------------------------------------------------------------
// Windows / Chunks

#[inline]
unsafe fn sub<'a, T>(v: &'a [T], start: usize, end: usize) -> &'a [T] {
    from_raw_parts(intrinsics::ptr_add(v.as_ptr(), start), end - start)
}

/// An iterator over overlapping subslices of length `size`.
#[derive(Debug)]
#[must_use]
pub struct Windows<'a, T: 'a> {
    v: &'a [T],
    size: usize,
}

impl<'a, T> Windows<'a, T> {
    #[inline]
    pub(crate) fn new(v: &'a [T], size: usize) -> Windows<'a, T> {
        Windows { v, size }
    }
}

impl<'a, T> Clone for Windows<'a, T> {
    fn clone(&self) -> Self {
        Windows { v: self.v, size: self.size }
    }
}

impl<'a, T> Iterator for Windows<'a, T> {
    type Item = &'a [T];
    #[inline]
    fn next(&mut self) -> Option<&'a [T]> {
        let len = self.v.len();
        if self.size > len {
            None
        } else {
            unsafe {
                let ret = sub(self.v, 0, self.size);
                self.v = sub(self.v, 1, len);
                Some(ret)
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.v.len();
        if self.size > len {
            (0, Some(0))
        } else {
            let size = len - self.size + 1;
            (size, Some(size))
        }
    }
    #[inline]
    fn count(self) -> usize {
        self.len()
    }
    #[inline]
    fn nth(&mut self, n: usize) -> Option<&'a [T]> {
        let len = self.v.len();
        let end = match self.size.checked_add(n) {
            Some(e) => e,
            None => {
                self.v = unsafe { sub(self.v, len, len) };
                return None;
            }
        };
        if end > len {
            self.v = unsafe { sub(self.v, len, len) };
            None
        } else {
            unsafe {
                let nth = sub(self.v, n, end);
                self.v = sub(self.v, n + 1, len);
                Some(nth)
            }
        }
    }
    #[inline]
    fn last(self) -> Option<&'a [T]> {
        let len = self.v.len();
        if self.size > len {
            None
        } else {
            unsafe { Some(sub(self.v, len - self.size, len)) }
        }
    }
}

impl<'a, T> DoubleEndedIterator for Windows<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a [T]> {
        let len = self.v.len();
        if self.size > len {
            None
        } else {
            unsafe {
                let ret = sub(self.v, len - self.size, len);
                self.v = sub(self.v, 0, len - 1);
                Some(ret)
            }
        }
    }
}

impl<'a, T> ExactSizeIterator for Windows<'a, T> {}
impl<'a, T> FusedIterator for Windows<'a, T> {}

/// An iterator over a slice in (non-overlapping) chunks (`chunk_size` elements at a time), starting at the beginning of the slice.
#[derive(Debug)]
#[must_use]
pub struct Chunks<'a, T: 'a> {
    v: &'a [T],
    chunk_size: usize,
}

impl<'a, T> Chunks<'a, T> {
    #[inline]
    pub(crate) fn new(v: &'a [T], size: usize) -> Chunks<'a, T> {
        Chunks { v, chunk_size: size }
    }
}

impl<'a, T> Clone for Chunks<'a, T> {
    fn clone(&self) -> Self {
        Chunks { v: self.v, chunk_size: self.chunk_size }
    }
}

impl<'a, T> Iterator for Chunks<'a, T> {
    type Item = &'a [T];
    #[inline]
    fn next(&mut self) -> Option<&'a [T]> {
        let len = self.v.len();
        if len == 0 {
            None
        } else {
            let n = if len < self.chunk_size { len } else { self.chunk_size };
            unsafe {
                let fst = sub(self.v, 0, n);
                self.v = sub(self.v, n, len);
                Some(fst)
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.v.len();
        if len == 0 {
            (0, Some(0))
        } else {
            let n = len / self.chunk_size;
            let rem = len % self.chunk_size;
            let n = if rem > 0 { n + 1 } else { n };
            (n, Some(n))
        }
    }
    #[inline]
    fn count(self) -> usize {
        self.len()
    }
    #[inline]
    fn last(self) -> Option<&'a [T]> {
        let mut s = self;
        s.next_back()
    }
}

impl<'a, T> DoubleEndedIterator for Chunks<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a [T]> {
        let len = self.v.len();
        if len == 0 {
            None
        } else {
            let remainder = len % self.chunk_size;
            let chunksz = if remainder != 0 { remainder } else { self.chunk_size };
            unsafe {
                let snd = sub(self.v, len - chunksz, len);
                self.v = sub(self.v, 0, len - chunksz);
                Some(snd)
            }
        }
    }
}

impl<'a, T> ExactSizeIterator for Chunks<'a, T> {}
impl<'a, T> FusedIterator for Chunks<'a, T> {}

/// An iterator over a slice in (non-overlapping) chunks (`chunk_size` elements at a time), starting at the beginning of the slice; the last up to `chunk_size-1` elements are omitted.
#[derive(Debug)]
#[must_use]
pub struct ChunksExact<'a, T: 'a> {
    v: &'a [T],
    rem: &'a [T],
    chunk_size: usize,
}

impl<'a, T> ChunksExact<'a, T> {
    #[inline]
    pub(crate) fn new(slice: &'a [T], chunk_size: usize) -> ChunksExact<'a, T> {
        let len = slice.len();
        let rem_len = len % chunk_size;
        let fst_len = len - rem_len;
        unsafe { ChunksExact { v: sub(slice, 0, fst_len), rem: sub(slice, fst_len, len), chunk_size } }
    }

    /// Returns the remainder of the original slice that is not going to be returned by the iterator.
    #[inline]
    pub fn remainder(&self) -> &'a [T] {
        self.rem
    }
}

impl<'a, T> Clone for ChunksExact<'a, T> {
    fn clone(&self) -> Self {
        ChunksExact { v: self.v, rem: self.rem, chunk_size: self.chunk_size }
    }
}

impl<'a, T> Iterator for ChunksExact<'a, T> {
    type Item = &'a [T];
    #[inline]
    fn next(&mut self) -> Option<&'a [T]> {
        let len = self.v.len();
        if len < self.chunk_size {
            None
        } else {
            unsafe {
                let fst = sub(self.v, 0, self.chunk_size);
                self.v = sub(self.v, self.chunk_size, len);
                Some(fst)
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.v.len() / self.chunk_size;
        (n, Some(n))
    }
    #[inline]
    fn count(self) -> usize {
        self.len()
    }
}

impl<'a, T> DoubleEndedIterator for ChunksExact<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a [T]> {
        let len = self.v.len();
        if len < self.chunk_size {
            None
        } else {
            unsafe {
                let snd = sub(self.v, len - self.chunk_size, len);
                self.v = sub(self.v, 0, len - self.chunk_size);
                Some(snd)
            }
        }
    }
}

impl<'a, T> ExactSizeIterator for ChunksExact<'a, T> {}
impl<'a, T> FusedIterator for ChunksExact<'a, T> {}

/// An iterator over a slice in (non-overlapping) chunks (`chunk_size` elements at a time), starting at the end of the slice.
#[derive(Debug)]
#[must_use]
pub struct RChunks<'a, T: 'a> {
    v: &'a [T],
    chunk_size: usize,
}

impl<'a, T> RChunks<'a, T> {
    #[inline]
    pub(crate) fn new(v: &'a [T], size: usize) -> RChunks<'a, T> {
        RChunks { v, chunk_size: size }
    }
}

impl<'a, T> Clone for RChunks<'a, T> {
    fn clone(&self) -> Self {
        RChunks { v: self.v, chunk_size: self.chunk_size }
    }
}

impl<'a, T> Iterator for RChunks<'a, T> {
    type Item = &'a [T];
    #[inline]
    fn next(&mut self) -> Option<&'a [T]> {
        let len = self.v.len();
        if len == 0 {
            None
        } else {
            let sz = if len < self.chunk_size { len } else { self.chunk_size };
            unsafe {
                let snd = sub(self.v, len - sz, len);
                self.v = sub(self.v, 0, len - sz);
                Some(snd)
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.v.len();
        if len == 0 {
            (0, Some(0))
        } else {
            let n = len / self.chunk_size;
            let rem = len % self.chunk_size;
            let n = if rem > 0 { n + 1 } else { n };
            (n, Some(n))
        }
    }
}

impl<'a, T> DoubleEndedIterator for RChunks<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a [T]> {
        let len = self.v.len();
        if len == 0 {
            None
        } else {
            let remainder = len % self.chunk_size;
            let chunksz = if remainder != 0 { remainder } else { self.chunk_size };
            unsafe {
                let fst = sub(self.v, 0, chunksz);
                self.v = sub(self.v, chunksz, len);
                Some(fst)
            }
        }
    }
}

impl<'a, T> ExactSizeIterator for RChunks<'a, T> {}
impl<'a, T> FusedIterator for RChunks<'a, T> {}

/// An iterator over a slice in (non-overlapping) mutable chunks (`chunk_size` elements at a time).
#[derive(Debug)]
#[must_use]
pub struct ChunksMut<'a, T: 'a> {
    ptr: *mut T,
    len: usize,
    chunk_size: usize,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T> ChunksMut<'a, T> {
    #[inline]
    pub(crate) fn new(slice: &'a mut [T], size: usize) -> ChunksMut<'a, T> {
        let len = slice.len();
        ChunksMut { ptr: slice.as_mut_ptr(), len, chunk_size: size, _marker: PhantomData }
    }
}

impl<'a, T> Iterator for ChunksMut<'a, T> {
    type Item = &'a mut [T];
    #[inline]
    fn next(&mut self) -> Option<&'a mut [T]> {
        if self.len == 0 {
            None
        } else {
            let sz = if self.len < self.chunk_size { self.len } else { self.chunk_size };
            unsafe {
                let head = from_raw_parts_mut(self.ptr, sz);
                self.ptr = intrinsics::ptr_add_mut(self.ptr, sz);
                self.len -= sz;
                Some(head)
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.len == 0 {
            (0, Some(0))
        } else {
            let n = self.len / self.chunk_size;
            let rem = self.len % self.chunk_size;
            let n = if rem > 0 { n + 1 } else { n };
            (n, Some(n))
        }
    }
}

impl<'a, T> DoubleEndedIterator for ChunksMut<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a mut [T]> {
        if self.len == 0 {
            None
        } else {
            let remainder = self.len % self.chunk_size;
            let sz = if remainder != 0 { remainder } else { self.chunk_size };
            unsafe {
                let tail = from_raw_parts_mut(intrinsics::ptr_add_mut(self.ptr, self.len - sz), sz);
                self.len -= sz;
                Some(tail)
            }
        }
    }
}

impl<'a, T> ExactSizeIterator for ChunksMut<'a, T> {}
impl<'a, T> FusedIterator for ChunksMut<'a, T> {}

/// An iterator over a slice in (non-overlapping) mutable chunks of exactly `chunk_size` elements.
#[derive(Debug)]
#[must_use]
pub struct ChunksExactMut<'a, T: 'a> {
    ptr: *mut T,
    len: usize,
    rem_ptr: *mut T,
    rem_len: usize,
    chunk_size: usize,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T> ChunksExactMut<'a, T> {
    #[inline]
    pub(crate) fn new(slice: &'a mut [T], chunk_size: usize) -> ChunksExactMut<'a, T> {
        let len = slice.len();
        let rem_len = len % chunk_size;
        let fst_len = len - rem_len;
        let ptr = slice.as_mut_ptr();
        ChunksExactMut {
            ptr,
            len: fst_len,
            rem_ptr: unsafe { intrinsics::ptr_add_mut(ptr, fst_len) },
            rem_len,
            chunk_size,
            _marker: PhantomData,
        }
    }

    /// Returns the remainder of the original slice that is not going to be returned by the iterator.
    #[inline]
    pub fn into_remainder(self) -> &'a mut [T] {
        unsafe { from_raw_parts_mut(self.rem_ptr, self.rem_len) }
    }
}

impl<'a, T> Iterator for ChunksExactMut<'a, T> {
    type Item = &'a mut [T];
    #[inline]
    fn next(&mut self) -> Option<&'a mut [T]> {
        if self.len < self.chunk_size {
            None
        } else {
            unsafe {
                let head = from_raw_parts_mut(self.ptr, self.chunk_size);
                self.ptr = intrinsics::ptr_add_mut(self.ptr, self.chunk_size);
                self.len -= self.chunk_size;
                Some(head)
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.len / self.chunk_size;
        (n, Some(n))
    }
}

impl<'a, T> DoubleEndedIterator for ChunksExactMut<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a mut [T]> {
        if self.len < self.chunk_size {
            None
        } else {
            unsafe {
                let tail = from_raw_parts_mut(intrinsics::ptr_add_mut(self.ptr, self.len - self.chunk_size), self.chunk_size);
                self.len -= self.chunk_size;
                Some(tail)
            }
        }
    }
}

impl<'a, T> ExactSizeIterator for ChunksExactMut<'a, T> {}
impl<'a, T> FusedIterator for ChunksExactMut<'a, T> {}

// ---------------------------------------------------------------------------
// Split by predicate

/// An iterator over subslices separated by elements that match a predicate function.
#[must_use]
pub struct Split<'a, T: 'a, P>
where
    P: FnMut(&T) -> bool,
{
    v: &'a [T],
    pred: P,
    finished: bool,
}

impl<'a, T: 'a, P: FnMut(&T) -> bool> Split<'a, T, P> {
    #[inline]
    pub(crate) fn new(slice: &'a [T], pred: P) -> Self {
        Split { v: slice, pred, finished: false }
    }

    #[inline]
    fn finish(&mut self) -> Option<&'a [T]> {
        if self.finished {
            None
        } else {
            self.finished = true;
            Some(self.v)
        }
    }
}

impl<'a, T, P: Clone + FnMut(&T) -> bool> Clone for Split<'a, T, P> {
    fn clone(&self) -> Self {
        Split { v: self.v, pred: self.pred.clone(), finished: self.finished }
    }
}

impl<'a, T, P> Iterator for Split<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    type Item = &'a [T];
    #[inline]
    fn next(&mut self) -> Option<&'a [T]> {
        if self.finished {
            return None;
        }
        let len = self.v.len();
        let mut i = 0;
        while i < len {
            if (self.pred)(&self.v[i]) {
                unsafe {
                    let head = sub(self.v, 0, i);
                    self.v = sub(self.v, i + 1, len);
                    return Some(head);
                }
            }
            i += 1;
        }
        self.finish()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.finished {
            (0, Some(0))
        } else {
            (1, Some(self.v.len() + 1))
        }
    }
}

impl<'a, T, P> DoubleEndedIterator for Split<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    #[inline]
    fn next_back(&mut self) -> Option<&'a [T]> {
        if self.finished {
            return None;
        }
        let len = self.v.len();
        let mut i = len;
        while i > 0 {
            i -= 1;
            if (self.pred)(&self.v[i]) {
                unsafe {
                    let tail = sub(self.v, i + 1, len);
                    self.v = sub(self.v, 0, i);
                    return Some(tail);
                }
            }
        }
        self.finish()
    }
}

impl<'a, T, P> FusedIterator for Split<'a, T, P> where P: FnMut(&T) -> bool {}

impl<'a, T: 'a, P: FnMut(&T) -> bool> Split<'a, T, P> {
    /// Returns a slice which contains items not yet handled by split.
    #[inline]
    pub fn as_slice(&self) -> &'a [T] {
        if self.finished { &[] } else { self.v }
    }
}

impl<'a, T: fmt::Debug, P> fmt::Debug for Split<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Split").field("v", &self.v).field("finished", &self.finished).finish()
    }
}

#[inline]
unsafe fn sub_mut<'a, T>(p: *mut T, start: usize, end: usize) -> &'a mut [T] {
    from_raw_parts_mut(intrinsics::ptr_add_mut(p, start), end - start)
}

// ---------------------------------------------------------------------------
// SplitMut

/// An iterator over the mutable subslices of the vector which are separated
/// by elements that match `pred`.
#[must_use]
pub struct SplitMut<'a, T: 'a, P>
where
    P: FnMut(&T) -> bool,
{
    ptr: *mut T,
    len: usize,
    pred: P,
    finished: bool,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T: 'a, P: FnMut(&T) -> bool> SplitMut<'a, T, P> {
    #[inline]
    pub(crate) fn new(slice: &'a mut [T], pred: P) -> Self {
        SplitMut { ptr: slice.as_mut_ptr(), len: slice.len(), pred, finished: false, _marker: PhantomData }
    }

    #[inline]
    fn finish(&mut self) -> Option<&'a mut [T]> {
        if self.finished {
            None
        } else {
            self.finished = true;
            Some(unsafe { sub_mut(self.ptr, 0, self.len) })
        }
    }
}

impl<'a, T: fmt::Debug, P> fmt::Debug for SplitMut<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SplitMut").field("finished", &self.finished).finish()
    }
}

impl<'a, T, P> Iterator for SplitMut<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    type Item = &'a mut [T];
    #[inline]
    fn next(&mut self) -> Option<&'a mut [T]> {
        if self.finished {
            return None;
        }
        let mut i = 0;
        while i < self.len {
            let hit = unsafe { (self.pred)(&*intrinsics::ptr_add_mut(self.ptr, i)) };
            if hit {
                unsafe {
                    let head = sub_mut(self.ptr, 0, i);
                    self.ptr = intrinsics::ptr_add_mut(self.ptr, i + 1);
                    self.len = self.len - i - 1;
                    return Some(head);
                }
            }
            i += 1;
        }
        self.finish()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.finished { (0, Some(0)) } else { (1, Some(self.len + 1)) }
    }
}

impl<'a, T, P> DoubleEndedIterator for SplitMut<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    #[inline]
    fn next_back(&mut self) -> Option<&'a mut [T]> {
        if self.finished {
            return None;
        }
        let mut i = self.len;
        while i > 0 {
            i -= 1;
            let hit = unsafe { (self.pred)(&*intrinsics::ptr_add_mut(self.ptr, i)) };
            if hit {
                unsafe {
                    let tail = sub_mut(self.ptr, i + 1, self.len);
                    self.len = i;
                    return Some(tail);
                }
            }
        }
        self.finish()
    }
}

impl<'a, T, P> FusedIterator for SplitMut<'a, T, P> where P: FnMut(&T) -> bool {}

// ---------------------------------------------------------------------------
// SplitInclusive / SplitInclusiveMut

/// An iterator over subslices separated by elements that match a predicate
/// function. Unlike `Split`, it contains the matched part as a terminator of
/// the subslice.
#[must_use]
pub struct SplitInclusive<'a, T: 'a, P>
where
    P: FnMut(&T) -> bool,
{
    v: &'a [T],
    pred: P,
    finished: bool,
}

impl<'a, T: 'a, P: FnMut(&T) -> bool> SplitInclusive<'a, T, P> {
    #[inline]
    pub(crate) fn new(slice: &'a [T], pred: P) -> Self {
        let finished = slice.is_empty();
        SplitInclusive { v: slice, pred, finished }
    }
}

impl<'a, T: fmt::Debug, P> fmt::Debug for SplitInclusive<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SplitInclusive").field("v", &self.v).field("finished", &self.finished).finish()
    }
}

impl<'a, T, P: Clone + FnMut(&T) -> bool> Clone for SplitInclusive<'a, T, P> {
    fn clone(&self) -> Self {
        SplitInclusive { v: self.v, pred: self.pred.clone(), finished: self.finished }
    }
}

impl<'a, T, P> Iterator for SplitInclusive<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    type Item = &'a [T];
    #[inline]
    fn next(&mut self) -> Option<&'a [T]> {
        if self.finished {
            return None;
        }
        let len = self.v.len();
        let mut idx = len;
        let mut i = 0;
        while i < len {
            if (self.pred)(&self.v[i]) {
                idx = i + 1;
                break;
            }
            i += 1;
        }
        if idx == len {
            self.finished = true;
        }
        unsafe {
            let head = sub(self.v, 0, idx);
            self.v = sub(self.v, idx, len);
            Some(head)
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.finished { (0, Some(0)) } else { (1, Some(if self.v.len() > 1 { self.v.len() } else { 1 })) }
    }
}

impl<'a, T, P> DoubleEndedIterator for SplitInclusive<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    #[inline]
    fn next_back(&mut self) -> Option<&'a [T]> {
        if self.finished {
            return None;
        }
        let len = self.v.len();
        // The last index of self.v is already checked and found to match
        // by the last iteration, so we start searching a new match
        // one index to the left.
        let remainder = if len == 0 { 0 } else { len - 1 };
        let mut idx = 0;
        let mut i = remainder;
        while i > 0 {
            i -= 1;
            if (self.pred)(&self.v[i]) {
                idx = i + 1;
                break;
            }
        }
        if idx == 0 {
            self.finished = true;
        }
        unsafe {
            let tail = sub(self.v, idx, len);
            self.v = sub(self.v, 0, idx);
            Some(tail)
        }
    }
}

impl<'a, T, P> FusedIterator for SplitInclusive<'a, T, P> where P: FnMut(&T) -> bool {}

/// An iterator over the mutable subslices of the vector which are separated
/// by elements that match `pred`, including the terminator.
#[must_use]
pub struct SplitInclusiveMut<'a, T: 'a, P>
where
    P: FnMut(&T) -> bool,
{
    ptr: *mut T,
    len: usize,
    pred: P,
    finished: bool,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T: 'a, P: FnMut(&T) -> bool> SplitInclusiveMut<'a, T, P> {
    #[inline]
    pub(crate) fn new(slice: &'a mut [T], pred: P) -> Self {
        let finished = slice.is_empty();
        SplitInclusiveMut { ptr: slice.as_mut_ptr(), len: slice.len(), pred, finished, _marker: PhantomData }
    }
}

impl<'a, T: fmt::Debug, P> fmt::Debug for SplitInclusiveMut<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SplitInclusiveMut").field("finished", &self.finished).finish()
    }
}

impl<'a, T, P> Iterator for SplitInclusiveMut<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    type Item = &'a mut [T];
    #[inline]
    fn next(&mut self) -> Option<&'a mut [T]> {
        if self.finished {
            return None;
        }
        let len = self.len;
        let mut idx = len;
        let mut i = 0;
        while i < len {
            let hit = unsafe { (self.pred)(&*intrinsics::ptr_add_mut(self.ptr, i)) };
            if hit {
                idx = i + 1;
                break;
            }
            i += 1;
        }
        if idx == len {
            self.finished = true;
        }
        unsafe {
            let head = sub_mut(self.ptr, 0, idx);
            self.ptr = intrinsics::ptr_add_mut(self.ptr, idx);
            self.len = len - idx;
            Some(head)
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.finished { (0, Some(0)) } else { (1, Some(if self.len > 1 { self.len } else { 1 })) }
    }
}

impl<'a, T, P> DoubleEndedIterator for SplitInclusiveMut<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    #[inline]
    fn next_back(&mut self) -> Option<&'a mut [T]> {
        if self.finished {
            return None;
        }
        let len = self.len;
        let remainder = if len == 0 { 0 } else { len - 1 };
        let mut idx = 0;
        let mut i = remainder;
        while i > 0 {
            i -= 1;
            let hit = unsafe { (self.pred)(&*intrinsics::ptr_add_mut(self.ptr, i)) };
            if hit {
                idx = i + 1;
                break;
            }
        }
        if idx == 0 {
            self.finished = true;
        }
        unsafe {
            let tail = sub_mut(self.ptr, idx, len);
            self.len = idx;
            Some(tail)
        }
    }
}

impl<'a, T, P> FusedIterator for SplitInclusiveMut<'a, T, P> where P: FnMut(&T) -> bool {}

// ---------------------------------------------------------------------------
// RSplit / RSplitMut

/// An iterator over subslices separated by elements that match a predicate
/// function, starting from the end of the slice.
#[must_use]
pub struct RSplit<'a, T: 'a, P>
where
    P: FnMut(&T) -> bool,
{
    inner: Split<'a, T, P>,
}

impl<'a, T: 'a, P: FnMut(&T) -> bool> RSplit<'a, T, P> {
    #[inline]
    pub(crate) fn new(slice: &'a [T], pred: P) -> Self {
        RSplit { inner: Split::new(slice, pred) }
    }
}

impl<'a, T: fmt::Debug, P> fmt::Debug for RSplit<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RSplit").field("inner", &self.inner).finish()
    }
}

impl<'a, T, P: Clone + FnMut(&T) -> bool> Clone for RSplit<'a, T, P> {
    fn clone(&self) -> Self {
        RSplit { inner: self.inner.clone() }
    }
}

impl<'a, T, P> Iterator for RSplit<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    type Item = &'a [T];
    #[inline]
    fn next(&mut self) -> Option<&'a [T]> {
        self.inner.next_back()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, T, P> DoubleEndedIterator for RSplit<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    #[inline]
    fn next_back(&mut self) -> Option<&'a [T]> {
        self.inner.next()
    }
}

impl<'a, T, P> FusedIterator for RSplit<'a, T, P> where P: FnMut(&T) -> bool {}

/// An iterator over the subslices of the vector which are separated by
/// elements that match `pred`, starting from the end of the slice.
#[must_use]
pub struct RSplitMut<'a, T: 'a, P>
where
    P: FnMut(&T) -> bool,
{
    inner: SplitMut<'a, T, P>,
}

impl<'a, T: 'a, P: FnMut(&T) -> bool> RSplitMut<'a, T, P> {
    #[inline]
    pub(crate) fn new(slice: &'a mut [T], pred: P) -> Self {
        RSplitMut { inner: SplitMut::new(slice, pred) }
    }
}

impl<'a, T: fmt::Debug, P> fmt::Debug for RSplitMut<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RSplitMut").field("inner", &self.inner).finish()
    }
}

impl<'a, T, P> Iterator for RSplitMut<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    type Item = &'a mut [T];
    #[inline]
    fn next(&mut self) -> Option<&'a mut [T]> {
        self.inner.next_back()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, T, P> DoubleEndedIterator for RSplitMut<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    #[inline]
    fn next_back(&mut self) -> Option<&'a mut [T]> {
        self.inner.next()
    }
}

impl<'a, T, P> FusedIterator for RSplitMut<'a, T, P> where P: FnMut(&T) -> bool {}

// ---------------------------------------------------------------------------
// SplitN / RSplitN / SplitNMut / RSplitNMut

macro_rules! splitn_iter {
    ($(#[$m:meta])* $name:ident, $inner:ident, $item:ty, $fwd:ident) => {
        $(#[$m])*
        #[must_use]
        pub struct $name<'a, T: 'a, P>
        where
            P: FnMut(&T) -> bool,
        {
            inner: $inner<'a, T, P>,
            count: usize,
        }

        impl<'a, T: 'a, P: FnMut(&T) -> bool> $name<'a, T, P> {
            #[inline]
            pub(crate) fn wrap(inner: $inner<'a, T, P>, count: usize) -> Self {
                $name { inner, count }
            }
        }

        impl<'a, T: fmt::Debug, P> fmt::Debug for $name<'a, T, P>
        where
            P: FnMut(&T) -> bool,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(stringify!($name)).field("inner", &self.inner).field("count", &self.count).finish()
            }
        }

        impl<'a, T, P> Iterator for $name<'a, T, P>
        where
            P: FnMut(&T) -> bool,
        {
            type Item = $item;
            #[inline]
            fn next(&mut self) -> Option<$item> {
                match self.count {
                    0 => None,
                    1 => {
                        self.count -= 1;
                        self.inner.$fwd()
                    }
                    _ => {
                        self.count -= 1;
                        self.inner.next()
                    }
                }
            }
            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                let (lower, upper_opt) = self.inner.size_hint();
                let count = self.count;
                let upper = match upper_opt {
                    Some(u) => if u < count { u } else { count },
                    None => count,
                };
                (if lower < count { lower } else { count }, Some(upper))
            }
        }

        impl<'a, T, P> FusedIterator for $name<'a, T, P> where P: FnMut(&T) -> bool {}
    };
}

impl<'a, T: 'a, P: FnMut(&T) -> bool> Split<'a, T, P> {
    #[inline]
    fn finish_all(&mut self) -> Option<&'a [T]> {
        self.finish()
    }
}

impl<'a, T: 'a, P: FnMut(&T) -> bool> RSplit<'a, T, P> {
    #[inline]
    fn finish_all(&mut self) -> Option<&'a [T]> {
        self.inner.finish()
    }
}

impl<'a, T: 'a, P: FnMut(&T) -> bool> SplitMut<'a, T, P> {
    #[inline]
    fn finish_all(&mut self) -> Option<&'a mut [T]> {
        self.finish()
    }
}

impl<'a, T: 'a, P: FnMut(&T) -> bool> RSplitMut<'a, T, P> {
    #[inline]
    fn finish_all(&mut self) -> Option<&'a mut [T]> {
        self.inner.finish()
    }
}

splitn_iter!(
    /// An iterator over subslices separated by elements that match a predicate
    /// function, limited to a given number of splits.
    SplitN, Split, &'a [T], finish_all
);
splitn_iter!(
    /// An iterator over subslices separated by elements that match a predicate
    /// function, limited to a given number of splits, starting from the end of the slice.
    RSplitN, RSplit, &'a [T], finish_all
);
splitn_iter!(
    /// An iterator over subslices separated by elements that match a predicate
    /// function, limited to a given number of splits.
    SplitNMut, SplitMut, &'a mut [T], finish_all
);
splitn_iter!(
    /// An iterator over subslices separated by elements that match a
    /// predicate function, limited to a given number of splits, starting
    /// from the end of the slice.
    RSplitNMut, RSplitMut, &'a mut [T], finish_all
);

// ---------------------------------------------------------------------------
// RChunksMut / RChunksExact / RChunksExactMut

/// An iterator over a slice in (non-overlapping) mutable chunks (`chunk_size`
/// elements at a time), starting at the end of the slice.
#[derive(Debug)]
#[must_use]
pub struct RChunksMut<'a, T: 'a> {
    ptr: *mut T,
    len: usize,
    chunk_size: usize,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T> RChunksMut<'a, T> {
    #[inline]
    pub(crate) fn new(slice: &'a mut [T], size: usize) -> Self {
        RChunksMut { ptr: slice.as_mut_ptr(), len: slice.len(), chunk_size: size, _marker: PhantomData }
    }
}

impl<'a, T> Iterator for RChunksMut<'a, T> {
    type Item = &'a mut [T];
    #[inline]
    fn next(&mut self) -> Option<&'a mut [T]> {
        let len = self.len;
        if len == 0 {
            None
        } else {
            let sz = if len < self.chunk_size { len } else { self.chunk_size };
            unsafe {
                let snd = sub_mut(self.ptr, len - sz, len);
                self.len = len - sz;
                Some(snd)
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.len + self.chunk_size - 1) / self.chunk_size;
        (n, Some(n))
    }
}

impl<'a, T> DoubleEndedIterator for RChunksMut<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a mut [T]> {
        let len = self.len;
        if len == 0 {
            None
        } else {
            let remainder = len % self.chunk_size;
            let sz = if remainder != 0 { remainder } else { self.chunk_size };
            unsafe {
                let fst = sub_mut(self.ptr, 0, sz);
                self.ptr = intrinsics::ptr_add_mut(self.ptr, sz);
                self.len = len - sz;
                Some(fst)
            }
        }
    }
}

impl<'a, T> ExactSizeIterator for RChunksMut<'a, T> {}
impl<'a, T> FusedIterator for RChunksMut<'a, T> {}

/// An iterator over a slice in (non-overlapping) chunks (`chunk_size` elements
/// at a time), starting at the end of the slice. The last up to
/// `chunk_size-1` elements are available from `remainder`.
#[derive(Debug)]
#[must_use]
pub struct RChunksExact<'a, T: 'a> {
    v: &'a [T],
    rem: &'a [T],
    chunk_size: usize,
}

impl<'a, T> RChunksExact<'a, T> {
    #[inline]
    pub(crate) fn new(slice: &'a [T], chunk_size: usize) -> Self {
        let rem = slice.len() % chunk_size;
        let (fst, snd) = slice.split_at(rem);
        RChunksExact { v: snd, rem: fst, chunk_size }
    }

    /// Returns the remainder of the original slice that is not going to be
    /// returned by the iterator.
    #[inline]
    pub fn remainder(&self) -> &'a [T] {
        self.rem
    }
}

impl<'a, T> Clone for RChunksExact<'a, T> {
    fn clone(&self) -> Self {
        RChunksExact { v: self.v, rem: self.rem, chunk_size: self.chunk_size }
    }
}

impl<'a, T> Iterator for RChunksExact<'a, T> {
    type Item = &'a [T];
    #[inline]
    fn next(&mut self) -> Option<&'a [T]> {
        let len = self.v.len();
        if len < self.chunk_size {
            None
        } else {
            unsafe {
                let snd = sub(self.v, len - self.chunk_size, len);
                self.v = sub(self.v, 0, len - self.chunk_size);
                Some(snd)
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.v.len() / self.chunk_size;
        (n, Some(n))
    }
}

impl<'a, T> DoubleEndedIterator for RChunksExact<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a [T]> {
        let len = self.v.len();
        if len < self.chunk_size {
            None
        } else {
            unsafe {
                let fst = sub(self.v, 0, self.chunk_size);
                self.v = sub(self.v, self.chunk_size, len);
                Some(fst)
            }
        }
    }
}

impl<'a, T> ExactSizeIterator for RChunksExact<'a, T> {}
impl<'a, T> FusedIterator for RChunksExact<'a, T> {}

/// An iterator over a slice in (non-overlapping) mutable chunks (`chunk_size`
/// elements at a time), starting at the end of the slice.
#[derive(Debug)]
#[must_use]
pub struct RChunksExactMut<'a, T: 'a> {
    ptr: *mut T,
    len: usize,
    rem_ptr: *mut T,
    rem_len: usize,
    chunk_size: usize,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T> RChunksExactMut<'a, T> {
    #[inline]
    pub(crate) fn new(slice: &'a mut [T], chunk_size: usize) -> Self {
        let len = slice.len();
        let rem = len % chunk_size;
        let p = slice.as_mut_ptr();
        RChunksExactMut {
            ptr: unsafe { intrinsics::ptr_add_mut(p, rem) },
            len: len - rem,
            rem_ptr: p,
            rem_len: rem,
            chunk_size,
            _marker: PhantomData,
        }
    }

    /// Returns the remainder of the original slice that is not going to be
    /// returned by the iterator.
    #[inline]
    pub fn into_remainder(self) -> &'a mut [T] {
        unsafe { sub_mut(self.rem_ptr, 0, self.rem_len) }
    }
}

impl<'a, T> Iterator for RChunksExactMut<'a, T> {
    type Item = &'a mut [T];
    #[inline]
    fn next(&mut self) -> Option<&'a mut [T]> {
        let len = self.len;
        if len < self.chunk_size {
            None
        } else {
            unsafe {
                let snd = sub_mut(self.ptr, len - self.chunk_size, len);
                self.len = len - self.chunk_size;
                Some(snd)
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.len / self.chunk_size;
        (n, Some(n))
    }
}

impl<'a, T> DoubleEndedIterator for RChunksExactMut<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a mut [T]> {
        let len = self.len;
        if len < self.chunk_size {
            None
        } else {
            unsafe {
                let fst = sub_mut(self.ptr, 0, self.chunk_size);
                self.ptr = intrinsics::ptr_add_mut(self.ptr, self.chunk_size);
                self.len = len - self.chunk_size;
                Some(fst)
            }
        }
    }
}

impl<'a, T> ExactSizeIterator for RChunksExactMut<'a, T> {}
impl<'a, T> FusedIterator for RChunksExactMut<'a, T> {}

// ---------------------------------------------------------------------------
// ChunkBy / ChunkByMut

/// An iterator over slice in (non-overlapping) chunks separated by a predicate.
#[must_use]
pub struct ChunkBy<'a, T: 'a, P> {
    slice: &'a [T],
    predicate: P,
}

impl<'a, T: 'a, P> ChunkBy<'a, T, P> {
    #[inline]
    pub(crate) fn new(slice: &'a [T], predicate: P) -> Self {
        ChunkBy { slice, predicate }
    }
}

impl<'a, T: 'a + fmt::Debug, P> fmt::Debug for ChunkBy<'a, T, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChunkBy").field("slice", &self.slice).finish()
    }
}

impl<'a, T: 'a, P> Iterator for ChunkBy<'a, T, P>
where
    P: FnMut(&T, &T) -> bool,
{
    type Item = &'a [T];
    #[inline]
    fn next(&mut self) -> Option<&'a [T]> {
        let len = self.slice.len();
        if len == 0 {
            None
        } else {
            let mut n = 1;
            while n < len {
                if !(self.predicate)(&self.slice[n - 1], &self.slice[n]) {
                    break;
                }
                n += 1;
            }
            unsafe {
                let head = sub(self.slice, 0, n);
                self.slice = sub(self.slice, n, len);
                Some(head)
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.slice.is_empty() { (0, Some(0)) } else { (1, Some(self.slice.len())) }
    }
}

impl<'a, T: 'a, P> DoubleEndedIterator for ChunkBy<'a, T, P>
where
    P: FnMut(&T, &T) -> bool,
{
    #[inline]
    fn next_back(&mut self) -> Option<&'a [T]> {
        let len = self.slice.len();
        if len == 0 {
            None
        } else {
            let mut n = 1;
            while n < len {
                if !(self.predicate)(&self.slice[len - n - 1], &self.slice[len - n]) {
                    break;
                }
                n += 1;
            }
            unsafe {
                let tail = sub(self.slice, len - n, len);
                self.slice = sub(self.slice, 0, len - n);
                Some(tail)
            }
        }
    }
}

impl<'a, T: 'a, P> FusedIterator for ChunkBy<'a, T, P> where P: FnMut(&T, &T) -> bool {}

/// An iterator over slice in (non-overlapping) mutable chunks separated by a predicate.
#[must_use]
pub struct ChunkByMut<'a, T: 'a, P> {
    ptr: *mut T,
    len: usize,
    predicate: P,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T: 'a, P> ChunkByMut<'a, T, P> {
    #[inline]
    pub(crate) fn new(slice: &'a mut [T], predicate: P) -> Self {
        ChunkByMut { ptr: slice.as_mut_ptr(), len: slice.len(), predicate, _marker: PhantomData }
    }
}

impl<'a, T: 'a + fmt::Debug, P> fmt::Debug for ChunkByMut<'a, T, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChunkByMut").field("len", &self.len).finish()
    }
}

impl<'a, T: 'a, P> Iterator for ChunkByMut<'a, T, P>
where
    P: FnMut(&T, &T) -> bool,
{
    type Item = &'a mut [T];
    #[inline]
    fn next(&mut self) -> Option<&'a mut [T]> {
        let len = self.len;
        if len == 0 {
            None
        } else {
            let mut n = 1;
            while n < len {
                let keep = unsafe {
                    (self.predicate)(&*intrinsics::ptr_add_mut(self.ptr, n - 1), &*intrinsics::ptr_add_mut(self.ptr, n))
                };
                if !keep {
                    break;
                }
                n += 1;
            }
            unsafe {
                let head = sub_mut(self.ptr, 0, n);
                self.ptr = intrinsics::ptr_add_mut(self.ptr, n);
                self.len = len - n;
                Some(head)
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.len == 0 { (0, Some(0)) } else { (1, Some(self.len)) }
    }
}

impl<'a, T: 'a, P> DoubleEndedIterator for ChunkByMut<'a, T, P>
where
    P: FnMut(&T, &T) -> bool,
{
    #[inline]
    fn next_back(&mut self) -> Option<&'a mut [T]> {
        let len = self.len;
        if len == 0 {
            None
        } else {
            let mut n = 1;
            while n < len {
                let keep = unsafe {
                    (self.predicate)(
                        &*intrinsics::ptr_add_mut(self.ptr, len - n - 1),
                        &*intrinsics::ptr_add_mut(self.ptr, len - n),
                    )
                };
                if !keep {
                    break;
                }
                n += 1;
            }
            unsafe {
                let tail = sub_mut(self.ptr, len - n, len);
                self.len = len - n;
                Some(tail)
            }
        }
    }
}

impl<'a, T: 'a, P> FusedIterator for ChunkByMut<'a, T, P> where P: FnMut(&T, &T) -> bool {}
