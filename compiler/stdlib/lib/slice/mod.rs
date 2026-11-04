//! Slice management and manipulation.

mod cmp;
mod index;
mod iter;
mod sort;
mod unstable_sort;

pub use self::index::{range, SliceIndex};
pub(crate) use self::index::{bounds_into_range, bounds_into_range_unchecked, bounds_into_slice_range};
pub use self::iter::{
    ChunkBy, ChunkByMut, Chunks, ChunksExact, ChunksExactMut, ChunksMut, Iter, IterMut, RChunks, RChunksExact,
    RChunksExactMut, RChunksMut, RSplit, RSplitMut, RSplitN, RSplitNMut, Split, SplitInclusive, SplitInclusiveMut,
    SplitMut, SplitN, SplitNMut, Windows,
};
pub use crate::ascii::EscapeAscii;

use crate::cmp::Ordering;
use crate::intrinsics;

/// Forms a slice from a pointer and a length.
#[inline]
pub const unsafe fn from_raw_parts<'a, T>(data: *const T, len: usize) -> &'a [T] {
    &*intrinsics::slice_from_raw_parts(data, len)
}

/// Forms a mutable slice from a pointer and a length.
#[inline]
pub unsafe fn from_raw_parts_mut<'a, T>(data: *mut T, len: usize) -> &'a mut [T] {
    &mut *intrinsics::slice_from_raw_parts_mut(data, len)
}

/// Converts a reference to T into a slice of length 1 (without copying).
#[inline]
pub fn from_ref<T>(s: &T) -> &[T] {
    unsafe { from_raw_parts(s as *const T, 1) }
}

/// Converts a mutable reference to T into a slice of length 1 (without copying).
#[inline]
pub fn from_mut<T>(s: &mut T) -> &mut [T] {
    unsafe { from_raw_parts_mut(s as *mut T, 1) }
}

/// The error type returned by [`get_disjoint_mut`][`slice::get_disjoint_mut`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GetDisjointMutError {
    /// An index provided was out-of-bounds for the slice.
    IndexOutOfBounds,
    /// Two indices provided were overlapping.
    OverlappingIndices,
}

impl crate::fmt::Display for GetDisjointMutError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        let msg = match self {
            GetDisjointMutError::IndexOutOfBounds => "an index is out of bounds",
            GetDisjointMutError::OverlappingIndices => "there were overlapping indices",
        };
        crate::fmt::Display::fmt(msg, f)
    }
}

impl crate::error::Error for GetDisjointMutError {}

#[inline(never)]
#[track_caller]
fn split_at_fail(mid: usize, len: usize) -> ! {
    crate::rt::panic_fmt(format_args!("mid > len"), file!(), line!(), column!())
}

#[inline(never)]
#[track_caller]
fn chunk_size_zero() -> ! {
    crate::rt::panic_str("chunk size must be non-zero", file!(), line!(), column!())
}

impl<T> [T] {
    /// Returns the number of elements in the slice.
    #[inline]
    pub const fn len(&self) -> usize {
        unsafe { intrinsics::slice_len(self as *const [T]) }
    }

    /// Returns `true` if the slice has a length of 0.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the first element of the slice, or `None` if it is empty.
    #[inline]
    pub fn first(&self) -> Option<&T> {
        if self.is_empty() {
            None
        } else {
            Some(&self[0])
        }
    }

    /// Returns a mutable reference to the first element of the slice, or `None` if it is empty.
    #[inline]
    pub fn first_mut(&mut self) -> Option<&mut T> {
        if self.is_empty() {
            None
        } else {
            Some(&mut self[0])
        }
    }

    /// Removes the subslice corresponding to the given range and returns a reference to it.
    /// Returns `None` (leaving the slice unchanged) if the range is out of bounds.
    /// Only ranges of the form `..n` or `n..` are supported, as in the real API.
    pub fn split_off<'a, R: crate::ops::RangeBounds<usize>>(self: &mut &'a Self, range: R) -> Option<&'a Self> {
        use crate::ops::Bound;
        let len = self.len();
        let s: &'a [T] = *self;
        match (range.start_bound(), range.end_bound()) {
            (Bound::Unbounded, Bound::Excluded(&n)) => {
                if n > len { return None; }
                let (a, b) = s.split_at(n);
                *self = b;
                Some(a)
            }
            (Bound::Unbounded, Bound::Included(&n)) => {
                if n >= len { return None; }
                let (a, b) = s.split_at(n + 1);
                *self = b;
                Some(a)
            }
            (Bound::Included(&n), Bound::Unbounded) => {
                if n > len { return None; }
                let (a, b) = s.split_at(n);
                *self = a;
                Some(b)
            }
            (Bound::Excluded(&n), Bound::Unbounded) => {
                if n >= len { return None; }
                let (a, b) = s.split_at(n + 1);
                *self = a;
                Some(b)
            }
            _ => None,
        }
    }

    /// Removes the subslice corresponding to the given range and returns a mutable reference to it.
    pub fn split_off_mut<'a, R: crate::ops::RangeBounds<usize>>(self: &mut &'a mut Self, range: R) -> Option<&'a mut Self> {
        use crate::ops::Bound;
        let len = self.len();
        let (cut, take_front) = match (range.start_bound(), range.end_bound()) {
            (Bound::Unbounded, Bound::Excluded(&n)) => (n, true),
            (Bound::Unbounded, Bound::Included(&n)) => (n.wrapping_add(1), true),
            (Bound::Included(&n), Bound::Unbounded) => (n, false),
            (Bound::Excluded(&n), Bound::Unbounded) => (n.wrapping_add(1), false),
            _ => return None,
        };
        if cut > len || cut == 0 && matches!(range.end_bound(), Bound::Included(_)) {
            return None;
        }
        let s: &'a mut [T] = crate::mem::take(self);
        let (a, b) = s.split_at_mut(cut);
        if take_front {
            *self = b;
            Some(a)
        } else {
            *self = a;
            Some(b)
        }
    }

    /// Removes the first element of the slice and returns a reference to it.
    pub fn split_off_first<'a>(self: &mut &'a Self) -> Option<&'a T> {
        let s: &'a [T] = *self;
        let (first, rem) = s.split_first()?;
        *self = rem;
        Some(first)
    }

    /// Removes the first element of the slice and returns a mutable reference to it.
    pub fn split_off_first_mut<'a>(self: &mut &'a mut Self) -> Option<&'a mut T> {
        let s: &'a mut [T] = crate::mem::take(self);
        match s.split_first_mut() {
            Some((first, rem)) => {
                *self = rem;
                Some(first)
            }
            None => None,
        }
    }

    /// Removes the last element of the slice and returns a reference to it.
    pub fn split_off_last<'a>(self: &mut &'a Self) -> Option<&'a T> {
        let s: &'a [T] = *self;
        let (last, rem) = s.split_last()?;
        *self = rem;
        Some(last)
    }

    /// Removes the last element of the slice and returns a mutable reference to it.
    pub fn split_off_last_mut<'a>(self: &mut &'a mut Self) -> Option<&'a mut T> {
        let s: &'a mut [T] = crate::mem::take(self);
        match s.split_last_mut() {
            Some((last, rem)) => {
                *self = rem;
                Some(last)
            }
            None => None,
        }
    }

    /// Splits the slice into a slice of `N`-element arrays, starting at the end, and a remainder (mutable).
    #[inline]
    #[track_caller]
    pub fn as_rchunks_mut<const N: usize>(&mut self) -> (&mut [T], &mut [[T; N]]) {
        if N == 0 {
            crate::rt::panic_str("chunk size must be non-zero", file!(), line!(), column!());
        }
        let len = self.len();
        let n = len / N;
        let r = len - n * N;
        let p = self.as_mut_ptr();
        unsafe { (from_raw_parts_mut(p, r), from_raw_parts_mut(p.add(r) as *mut [T; N], n)) }
    }

    /// Splits the slice into a slice of `N`-element arrays, assuming that there's no remainder (mutable).
    #[inline]
    pub unsafe fn as_chunks_unchecked_mut<const N: usize>(&mut self) -> &mut [[T; N]] {
        let len = self.len() / N;
        from_raw_parts_mut(self.as_mut_ptr() as *mut [T; N], len)
    }

    /// Returns the first and all the rest of the elements of the slice, or `None` if it is empty.
    #[inline]
    pub fn split_first(&self) -> Option<(&T, &[T])> {
        if self.is_empty() {
            None
        } else {
            let len = self.len();
            unsafe { Some((&self[0], from_raw_parts(self.as_ptr().add(1), len - 1))) }
        }
    }

    /// Returns the first and all the rest of the elements of the slice, or `None` if it is empty.
    #[inline]
    pub fn split_first_mut(&mut self) -> Option<(&mut T, &mut [T])> {
        if self.is_empty() {
            None
        } else {
            let len = self.len();
            let p = self.as_mut_ptr();
            unsafe { Some((&mut *p, from_raw_parts_mut(p.add(1), len - 1))) }
        }
    }

    /// Returns the last and all the rest of the elements of the slice, or `None` if it is empty.
    #[inline]
    pub fn split_last(&self) -> Option<(&T, &[T])> {
        if self.is_empty() {
            None
        } else {
            let len = self.len();
            unsafe { Some((&self[len - 1], from_raw_parts(self.as_ptr(), len - 1))) }
        }
    }

    /// Returns the last and all the rest of the elements of the slice, or `None` if it is empty.
    #[inline]
    pub fn split_last_mut(&mut self) -> Option<(&mut T, &mut [T])> {
        if self.is_empty() {
            None
        } else {
            let len = self.len();
            let p = self.as_mut_ptr();
            unsafe { Some((&mut *p.add(len - 1), from_raw_parts_mut(p, len - 1))) }
        }
    }

    /// Returns the last element of the slice, or `None` if it is empty.
    #[inline]
    pub fn last(&self) -> Option<&T> {
        if self.is_empty() {
            None
        } else {
            Some(&self[self.len() - 1])
        }
    }

    /// Returns a mutable reference to the last item in the slice, or `None` if it is empty.
    #[inline]
    pub fn last_mut(&mut self) -> Option<&mut T> {
        if self.is_empty() {
            None
        } else {
            let i = self.len() - 1;
            Some(&mut self[i])
        }
    }

    /// Returns a reference to an element or subslice depending on the type of index.
    #[inline]
    pub fn get<I: SliceIndex<[T]>>(&self, index: I) -> Option<&I::Output> {
        index.get(self)
    }

    /// Returns a mutable reference to an element or subslice depending on the type of index.
    #[inline]
    pub fn get_mut<I: SliceIndex<[T]>>(&mut self, index: I) -> Option<&mut I::Output> {
        index.get_mut(self)
    }

    /// Returns a reference to an element or subslice, without doing bounds checking.
    #[inline]
    pub unsafe fn get_unchecked<I: SliceIndex<[T]>>(&self, index: I) -> &I::Output {
        &*index.get_unchecked(self as *const [T])
    }

    /// Returns a mutable reference to an element or subslice, without doing bounds checking.
    #[inline]
    pub unsafe fn get_unchecked_mut<I: SliceIndex<[T]>>(&mut self, index: I) -> &mut I::Output {
        &mut *index.get_unchecked_mut(self as *mut [T])
    }

    /// Returns a raw pointer to the slice's buffer.
    #[inline]
    pub const fn as_ptr(&self) -> *const T {
        unsafe { intrinsics::slice_ptr(self as *const [T]) }
    }

    /// Returns an unsafe mutable pointer to the slice's buffer.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        unsafe { intrinsics::slice_ptr(self as *mut [T] as *const [T]) as *mut T }
    }

    /// Swaps two elements in the slice.
    #[inline]
    #[track_caller]
    pub fn swap(&mut self, a: usize, b: usize) {
        let len = self.len();
        if a >= len {
            index::index_out_of_bounds(a, len);
        }
        if b >= len {
            index::index_out_of_bounds(b, len);
        }
        if a != b {
            unsafe {
                let p = self.as_mut_ptr();
                crate::ptr::swap(p.add(a), p.add(b));
            }
        }
    }

    /// Reverses the order of elements in the slice, in place.
    #[inline]
    pub fn reverse(&mut self) {
        let len = self.len();
        if len < 2 {
            return;
        }
        let p = self.as_mut_ptr();
        let mut i = 0;
        let mut j = len - 1;
        while i < j {
            unsafe {
                crate::ptr::swap(p.add(i), p.add(j));
            }
            i += 1;
            j -= 1;
        }
    }

    /// Returns an iterator over the slice.
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter::new(self)
    }

    /// Returns an iterator that allows modifying each value.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut::new(self)
    }

    /// Returns an iterator over all contiguous windows of length `size`.
    #[inline]
    #[track_caller]
    pub fn windows(&self, size: usize) -> Windows<'_, T> {
        if size == 0 {
            crate::rt::panic_str("window size must be non-zero", file!(), line!(), column!());
        }
        Windows::new(self, size)
    }

    /// Returns an iterator over `chunk_size` elements of the slice at a time.
    #[inline]
    #[track_caller]
    pub fn chunks(&self, chunk_size: usize) -> Chunks<'_, T> {
        if chunk_size == 0 {
            chunk_size_zero();
        }
        Chunks::new(self, chunk_size)
    }

    /// Returns an iterator over `chunk_size` elements of the slice at a time (mutable).
    #[inline]
    #[track_caller]
    pub fn chunks_mut(&mut self, chunk_size: usize) -> ChunksMut<'_, T> {
        if chunk_size == 0 {
            chunk_size_zero();
        }
        ChunksMut::new(self, chunk_size)
    }

    /// Returns an iterator over exactly `chunk_size` elements of the slice at a time.
    #[inline]
    #[track_caller]
    pub fn chunks_exact(&self, chunk_size: usize) -> ChunksExact<'_, T> {
        if chunk_size == 0 {
            chunk_size_zero();
        }
        ChunksExact::new(self, chunk_size)
    }

    /// Returns an iterator over exactly `chunk_size` elements of the slice at a time (mutable).
    #[inline]
    #[track_caller]
    pub fn chunks_exact_mut(&mut self, chunk_size: usize) -> ChunksExactMut<'_, T> {
        if chunk_size == 0 {
            chunk_size_zero();
        }
        ChunksExactMut::new(self, chunk_size)
    }

    /// Returns an iterator over `chunk_size` elements of the slice at a time, starting at the end of the slice.
    #[inline]
    #[track_caller]
    pub fn rchunks(&self, chunk_size: usize) -> RChunks<'_, T> {
        if chunk_size == 0 {
            chunk_size_zero();
        }
        RChunks::new(self, chunk_size)
    }

    /// Divides one slice into two at an index.
    #[inline]
    #[track_caller]
    pub fn split_at(&self, mid: usize) -> (&[T], &[T]) {
        let len = self.len();
        if mid > len {
            split_at_fail(mid, len);
        }
        let p = self.as_ptr();
        unsafe { (from_raw_parts(p, mid), from_raw_parts(p.add(mid), len - mid)) }
    }

    /// Divides one mutable slice into two at an index.
    #[inline]
    #[track_caller]
    pub fn split_at_mut(&mut self, mid: usize) -> (&mut [T], &mut [T]) {
        let len = self.len();
        if mid > len {
            split_at_fail(mid, len);
        }
        let p = self.as_mut_ptr();
        unsafe { (from_raw_parts_mut(p, mid), from_raw_parts_mut(p.add(mid), len - mid)) }
    }

    /// Divides one slice into two at an index, returning `None` if the slice is too short.
    #[inline]
    pub fn split_at_checked(&self, mid: usize) -> Option<(&[T], &[T])> {
        if mid <= self.len() {
            Some(self.split_at(mid))
        } else {
            None
        }
    }

    /// Divides one mutable slice into two at an index, returning `None` if the slice is too short.
    #[inline]
    pub fn split_at_mut_checked(&mut self, mid: usize) -> Option<(&mut [T], &mut [T])> {
        if mid <= self.len() {
            Some(self.split_at_mut(mid))
        } else {
            None
        }
    }

    /// Divides one slice into two at an index, without doing bounds checking.
    #[inline]
    pub unsafe fn split_at_unchecked(&self, mid: usize) -> (&[T], &[T]) {
        let len = self.len();
        let p = self.as_ptr();
        (from_raw_parts(p, mid), from_raw_parts(p.add(mid), len - mid))
    }

    /// Divides one mutable slice into two at an index, without doing bounds checking.
    #[inline]
    pub unsafe fn split_at_mut_unchecked(&mut self, mid: usize) -> (&mut [T], &mut [T]) {
        let len = self.len();
        let p = self.as_mut_ptr();
        (from_raw_parts_mut(p, mid), from_raw_parts_mut(p.add(mid), len - mid))
    }

    /// Gets a reference to the underlying array if the slice has exactly `N` elements.
    #[inline]
    pub fn as_array<const N: usize>(&self) -> Option<&[T; N]> {
        if self.len() == N {
            Some(unsafe { &*(self.as_ptr() as *const [T; N]) })
        } else {
            None
        }
    }

    /// Gets a mutable reference to the underlying array if the slice has exactly `N` elements.
    #[inline]
    pub fn as_mut_array<const N: usize>(&mut self) -> Option<&mut [T; N]> {
        if self.len() == N {
            Some(unsafe { &mut *(self.as_mut_ptr() as *mut [T; N]) })
        } else {
            None
        }
    }

    /// Returns an array reference to the first `N` items in the slice.
    #[inline]
    pub fn first_chunk<const N: usize>(&self) -> Option<&[T; N]> {
        if self.len() < N {
            None
        } else {
            Some(unsafe { &*(self.as_ptr() as *const [T; N]) })
        }
    }

    /// Returns a mutable array reference to the first `N` items in the slice.
    #[inline]
    pub fn first_chunk_mut<const N: usize>(&mut self) -> Option<&mut [T; N]> {
        if self.len() < N {
            None
        } else {
            Some(unsafe { &mut *(self.as_mut_ptr() as *mut [T; N]) })
        }
    }

    /// Returns an array reference to the first `N` items in the slice and the remaining slice.
    #[inline]
    pub fn split_first_chunk<const N: usize>(&self) -> Option<(&[T; N], &[T])> {
        if self.len() < N {
            None
        } else {
            let len = self.len();
            let p = self.as_ptr();
            unsafe { Some((&*(p as *const [T; N]), from_raw_parts(p.add(N), len - N))) }
        }
    }

    /// Returns a mutable array reference to the first `N` items in the slice and the remaining slice.
    #[inline]
    pub fn split_first_chunk_mut<const N: usize>(&mut self) -> Option<(&mut [T; N], &mut [T])> {
        if self.len() < N {
            None
        } else {
            let len = self.len();
            let p = self.as_mut_ptr();
            unsafe { Some((&mut *(p as *mut [T; N]), from_raw_parts_mut(p.add(N), len - N))) }
        }
    }

    /// Returns an array reference to the last `N` items in the slice and the remaining slice.
    #[inline]
    pub fn split_last_chunk<const N: usize>(&self) -> Option<(&[T], &[T; N])> {
        if self.len() < N {
            None
        } else {
            let len = self.len();
            let p = self.as_ptr();
            unsafe { Some((from_raw_parts(p, len - N), &*(p.add(len - N) as *const [T; N]))) }
        }
    }

    /// Returns a mutable array reference to the last `N` items in the slice and the remaining slice.
    #[inline]
    pub fn split_last_chunk_mut<const N: usize>(&mut self) -> Option<(&mut [T], &mut [T; N])> {
        if self.len() < N {
            None
        } else {
            let len = self.len();
            let p = self.as_mut_ptr();
            unsafe { Some((from_raw_parts_mut(p, len - N), &mut *(p.add(len - N) as *mut [T; N]))) }
        }
    }

    /// Returns an array reference to the last `N` items in the slice.
    #[inline]
    pub fn last_chunk<const N: usize>(&self) -> Option<&[T; N]> {
        if self.len() < N {
            None
        } else {
            let len = self.len();
            Some(unsafe { &*(self.as_ptr().add(len - N) as *const [T; N]) })
        }
    }

    /// Returns a mutable array reference to the last `N` items in the slice.
    #[inline]
    pub fn last_chunk_mut<const N: usize>(&mut self) -> Option<&mut [T; N]> {
        if self.len() < N {
            None
        } else {
            let len = self.len();
            Some(unsafe { &mut *(self.as_mut_ptr().add(len - N) as *mut [T; N]) })
        }
    }

    /// Splits the slice into a slice of `N`-element arrays, starting at the beginning
    /// of the slice, and a remainder slice with length strictly less than `N`.
    #[inline]
    #[track_caller]
    pub fn as_chunks<const N: usize>(&self) -> (&[[T; N]], &[T]) {
        if N == 0 {
            crate::rt::panic_str("chunk size must be non-zero", file!(), line!(), column!());
        }
        let len = self.len();
        let n = len / N;
        let p = self.as_ptr();
        unsafe { (from_raw_parts(p as *const [T; N], n), from_raw_parts(p.add(n * N), len - n * N)) }
    }

    /// Splits the slice into a slice of `N`-element arrays, starting at the end
    /// of the slice, and a remainder slice with length strictly less than `N`.
    #[inline]
    #[track_caller]
    pub fn as_rchunks<const N: usize>(&self) -> (&[T], &[[T; N]]) {
        if N == 0 {
            crate::rt::panic_str("chunk size must be non-zero", file!(), line!(), column!());
        }
        let len = self.len();
        let n = len / N;
        let r = len - n * N;
        let p = self.as_ptr();
        unsafe { (from_raw_parts(p, r), from_raw_parts(p.add(r) as *const [T; N], n)) }
    }

    /// Splits the slice into a slice of `N`-element arrays (mutable), and a remainder slice.
    #[inline]
    #[track_caller]
    pub fn as_chunks_mut<const N: usize>(&mut self) -> (&mut [[T; N]], &mut [T]) {
        if N == 0 {
            crate::rt::panic_str("chunk size must be non-zero", file!(), line!(), column!());
        }
        let len = self.len();
        let n = len / N;
        let p = self.as_mut_ptr();
        unsafe { (from_raw_parts_mut(p as *mut [T; N], n), from_raw_parts_mut(p.add(n * N), len - n * N)) }
    }

    /// Splits the slice into a slice of `N`-element arrays, assuming that there's no remainder.
    #[inline]
    pub unsafe fn as_chunks_unchecked<const N: usize>(&self) -> &[[T; N]] {
        from_raw_parts(self.as_ptr() as *const [T; N], self.len() / N)
    }

    /// Returns the two raw pointers spanning the slice.
    #[inline]
    pub fn as_ptr_range(&self) -> crate::ops::Range<*const T> {
        let p = self.as_ptr();
        unsafe { p..p.add(self.len()) }
    }

    /// Returns the two unsafe mutable pointers spanning the slice.
    #[inline]
    pub fn as_mut_ptr_range(&mut self) -> crate::ops::Range<*mut T> {
        let len = self.len();
        let p = self.as_mut_ptr();
        unsafe { p..p.add(len) }
    }

    /// Swaps two elements in the slice, without doing bounds checking.
    #[inline]
    pub unsafe fn swap_unchecked(&mut self, a: usize, b: usize) {
        let p = self.as_mut_ptr();
        crate::ptr::swap(p.add(a), p.add(b));
    }

    /// Returns mutable references to many indices at once.
    pub fn get_disjoint_mut<const N: usize>(
        &mut self,
        indices: [usize; N],
    ) -> Result<[&mut T; N], GetDisjointMutError> {
        let len = self.len();
        let mut i = 0;
        while i < N {
            if indices[i] >= len {
                return Err(GetDisjointMutError::IndexOutOfBounds);
            }
            let mut j = 0;
            while j < i {
                if indices[i] == indices[j] {
                    return Err(GetDisjointMutError::OverlappingIndices);
                }
                j += 1;
            }
            i += 1;
        }
        let p = self.as_mut_ptr();
        Ok(crate::array::from_fn(|k| unsafe { &mut *p.add(indices[k]) }))
    }

    /// Returns an iterator over subslices separated by elements that match `pred`.
    #[inline]
    pub fn split<F>(&self, pred: F) -> Split<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        Split::new(self, pred)
    }

    /// Returns an iterator over mutable subslices separated by elements that match `pred`.
    #[inline]
    pub fn split_mut<F>(&mut self, pred: F) -> SplitMut<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        SplitMut::new(self, pred)
    }

    /// Returns an iterator over subslices separated by elements that match `pred`.
    /// The matched element is contained in the end of the previous subslice as a terminator.
    #[inline]
    pub fn split_inclusive<F>(&self, pred: F) -> SplitInclusive<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        SplitInclusive::new(self, pred)
    }

    /// Returns an iterator over mutable subslices separated by elements that match `pred`,
    /// including the terminator.
    #[inline]
    pub fn split_inclusive_mut<F>(&mut self, pred: F) -> SplitInclusiveMut<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        SplitInclusiveMut::new(self, pred)
    }

    /// Returns an iterator over subslices separated by elements that match `pred`,
    /// starting at the end of the slice and working backwards.
    #[inline]
    pub fn rsplit<F>(&self, pred: F) -> RSplit<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        RSplit::new(self, pred)
    }

    /// Returns an iterator over mutable subslices separated by elements that match `pred`,
    /// starting at the end of the slice and working backwards.
    #[inline]
    pub fn rsplit_mut<F>(&mut self, pred: F) -> RSplitMut<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        RSplitMut::new(self, pred)
    }

    /// Returns an iterator over subslices separated by elements that match `pred`,
    /// limited to returning at most `n` items.
    #[inline]
    pub fn splitn<F>(&self, n: usize, pred: F) -> SplitN<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        SplitN::wrap(Split::new(self, pred), n)
    }

    /// Returns an iterator over mutable subslices separated by elements that match `pred`,
    /// limited to returning at most `n` items.
    #[inline]
    pub fn splitn_mut<F>(&mut self, n: usize, pred: F) -> SplitNMut<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        SplitNMut::wrap(SplitMut::new(self, pred), n)
    }

    /// Returns an iterator over subslices separated by elements that match `pred`
    /// limited to returning at most `n` items, starting from the end of the slice.
    #[inline]
    pub fn rsplitn<F>(&self, n: usize, pred: F) -> RSplitN<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        RSplitN::wrap(RSplit::new(self, pred), n)
    }

    /// Returns an iterator over mutable subslices separated by elements that match `pred`
    /// limited to returning at most `n` items, starting from the end of the slice.
    #[inline]
    pub fn rsplitn_mut<F>(&mut self, n: usize, pred: F) -> RSplitNMut<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        RSplitNMut::wrap(RSplitMut::new(self, pred), n)
    }

    /// Returns an iterator over `chunk_size` elements of the slice at a time (mutable),
    /// starting at the end of the slice.
    #[inline]
    #[track_caller]
    pub fn rchunks_mut(&mut self, chunk_size: usize) -> RChunksMut<'_, T> {
        if chunk_size == 0 {
            chunk_size_zero();
        }
        RChunksMut::new(self, chunk_size)
    }

    /// Returns an iterator over exactly `chunk_size` elements of the slice at a time,
    /// starting at the end of the slice.
    #[inline]
    #[track_caller]
    pub fn rchunks_exact(&self, chunk_size: usize) -> RChunksExact<'_, T> {
        if chunk_size == 0 {
            chunk_size_zero();
        }
        RChunksExact::new(self, chunk_size)
    }

    /// Returns an iterator over exactly `chunk_size` elements of the slice at a time (mutable),
    /// starting at the end of the slice.
    #[inline]
    #[track_caller]
    pub fn rchunks_exact_mut(&mut self, chunk_size: usize) -> RChunksExactMut<'_, T> {
        if chunk_size == 0 {
            chunk_size_zero();
        }
        RChunksExactMut::new(self, chunk_size)
    }

    /// Returns an iterator over the slice producing non-overlapping runs of elements
    /// using the predicate to separate them.
    #[inline]
    pub fn chunk_by<F>(&self, pred: F) -> ChunkBy<'_, T, F>
    where
        F: FnMut(&T, &T) -> bool,
    {
        ChunkBy::new(self, pred)
    }

    /// Returns an iterator over the slice producing non-overlapping mutable runs of
    /// elements using the predicate to separate them.
    #[inline]
    pub fn chunk_by_mut<F>(&mut self, pred: F) -> ChunkByMut<'_, T, F>
    where
        F: FnMut(&T, &T) -> bool,
    {
        ChunkByMut::new(self, pred)
    }

    /// Transmutes the slice to a slice of another type, ensuring alignment of the types is maintained.
    pub unsafe fn align_to<U>(&self) -> (&[T], &[U], &[T]) {
        let tsize = crate::mem::size_of::<T>();
        let usize_ = crate::mem::size_of::<U>();
        if usize_ == 0 || tsize == 0 {
            return (self, &[], &[]);
        }
        let ptr = self.as_ptr();
        let offset = ptr.align_offset(crate::mem::align_of::<U>());
        let len = self.len();
        if offset > len {
            return (self, &[], &[]);
        }
        let rest_bytes = (len - offset) * tsize;
        let us_len = rest_bytes / usize_;
        let used = us_len * usize_;
        if used % tsize != 0 {
            // The middle part must consist of whole `T`s on both ends.
            let mut n = us_len;
            while n > 0 && (n * usize_) % tsize != 0 {
                n -= 1;
            }
            let ts_used = n * usize_ / tsize;
            return (
                from_raw_parts(ptr, offset),
                from_raw_parts(ptr.add(offset) as *const U, n),
                from_raw_parts(ptr.add(offset + ts_used), len - offset - ts_used),
            );
        }
        let ts_used = used / tsize;
        (
            from_raw_parts(ptr, offset),
            from_raw_parts(ptr.add(offset) as *const U, us_len),
            from_raw_parts(ptr.add(offset + ts_used), len - offset - ts_used),
        )
    }

    /// Transmutes the mutable slice to a mutable slice of another type, ensuring alignment of the types is maintained.
    pub unsafe fn align_to_mut<U>(&mut self) -> (&mut [T], &mut [U], &mut [T]) {
        let (a, b, c) = self.align_to::<U>();
        (
            from_raw_parts_mut(a.as_ptr() as *mut T, a.len()),
            from_raw_parts_mut(b.as_ptr() as *mut U, b.len()),
            from_raw_parts_mut(c.as_ptr() as *mut T, c.len()),
        )
    }

    /// Returns `true` if the slice contains an element with the given value.
    #[inline]
    pub fn contains(&self, x: &T) -> bool
    where
        T: PartialEq,
    {
        let n = self.len();
        let mut i = 0;
        while i < n {
            if self[i] == *x {
                return true;
            }
            i += 1;
        }
        false
    }

    /// Returns `true` if `needle` is a prefix of the slice.
    pub fn starts_with(&self, needle: &[T]) -> bool
    where
        T: PartialEq,
    {
        let n = needle.len();
        self.len() >= n && needle == &self[..n]
    }

    /// Returns `true` if `needle` is a suffix of the slice.
    pub fn ends_with(&self, needle: &[T]) -> bool
    where
        T: PartialEq,
    {
        let (m, n) = (self.len(), needle.len());
        m >= n && needle == &self[m - n..]
    }

    /// Returns a subslice with the prefix removed.
    pub fn strip_prefix(&self, prefix: &[T]) -> Option<&[T]>
    where
        T: PartialEq,
    {
        let n = prefix.len();
        if n <= self.len() {
            let (head, tail) = self.split_at(n);
            if head == prefix {
                return Some(tail);
            }
        }
        None
    }

    /// Returns a subslice with the suffix removed.
    pub fn strip_suffix(&self, suffix: &[T]) -> Option<&[T]>
    where
        T: PartialEq,
    {
        let (len, n) = (self.len(), suffix.len());
        if n <= len {
            let (head, tail) = self.split_at(len - n);
            if tail == suffix {
                return Some(head);
            }
        }
        None
    }

    /// Binary searches this slice for a given element.
    #[inline]
    pub fn binary_search(&self, x: &T) -> Result<usize, usize>
    where
        T: Ord,
    {
        self.binary_search_by(|p| p.cmp(x))
    }

    /// Binary searches this slice with a comparator function.
    pub fn binary_search_by<'a, F>(&'a self, f: F) -> Result<usize, usize>
    where
        F: FnMut(&'a T) -> Ordering,
    {
        let mut f = f;
        let mut size = self.len();
        if size == 0 {
            return Err(0);
        }
        let mut base = 0usize;
        while size > 1 {
            let half = size / 2;
            let mid = base + half;
            let cmp = f(unsafe { &*self.as_ptr().add(mid) });
            base = if cmp == Ordering::Greater { base } else { mid };
            size -= half;
        }
        let cmp = f(unsafe { &*self.as_ptr().add(base) });
        if cmp == Ordering::Equal {
            Ok(base)
        } else {
            let result = base + (cmp == Ordering::Less) as usize;
            Err(result)
        }
    }

    /// Binary searches this slice with a key extraction function.
    #[inline]
    pub fn binary_search_by_key<'a, B, F>(&'a self, b: &B, f: F) -> Result<usize, usize>
    where
        F: FnMut(&'a T) -> B,
        B: Ord,
    {
        let mut f = f;
        self.binary_search_by(|k| f(k).cmp(b))
    }

    /// Returns the index of the partition point according to the given predicate.
    pub fn partition_point<P>(&self, pred: P) -> usize
    where
        P: FnMut(&T) -> bool,
    {
        let mut pred = pred;
        match self.binary_search_by(|x| if pred(x) { Ordering::Less } else { Ordering::Greater }) {
            Ok(i) => i,
            Err(i) => i,
        }
    }

    /// Sorts the slice (stable).
    #[inline]
    pub fn sort(&mut self)
    where
        T: Ord,
    {
        sort::merge_sort(self, &mut |a: &T, b: &T| a.lt(b));
    }

    /// Sorts the slice with a comparison function (stable).
    #[inline]
    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        let mut compare = compare;
        sort::merge_sort(self, &mut |a: &T, b: &T| compare(a, b) == Ordering::Less);
    }

    /// Sorts the slice with a key extraction function (stable).
    #[inline]
    pub fn sort_by_key<K, F>(&mut self, f: F)
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        let mut f = f;
        sort::merge_sort(self, &mut |a: &T, b: &T| f(a).lt(&f(b)));
    }

    /// Sorts the slice with a key extraction function, calling it once per element.
    pub fn sort_by_cached_key<K, F>(&mut self, f: F)
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        let len = self.len();
        if len < 2 {
            return;
        }
        let mut f = f;
        let mut indices: Vec<(K, usize)> = Vec::with_capacity(len);
        let mut i = 0;
        while i < len {
            indices.push((f(&self[i]), i));
            i += 1;
        }
        indices.sort();
        let mut i = 0;
        while i < len {
            let mut index = indices[i].1;
            while index < i {
                index = indices[index].1;
            }
            indices[i].1 = index;
            self.swap(i, index);
            i += 1;
        }
    }

    /// Sorts the slice (the order of equal elements is unspecified).
    #[inline]
    pub fn sort_unstable(&mut self)
    where
        T: Ord,
    {
        unstable_sort::sort(self, &mut |a: &T, b: &T| a.lt(b));
    }

    /// Sorts the slice with a comparison function (the order of equal elements is unspecified).
    #[inline]
    pub fn sort_unstable_by<F>(&mut self, compare: F)
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        let mut compare = compare;
        unstable_sort::sort(self, &mut |a: &T, b: &T| compare(a, b) == Ordering::Less);
    }

    /// Sorts the slice with a key extraction function (the order of equal elements is unspecified).
    #[inline]
    pub fn sort_unstable_by_key<K, F>(&mut self, f: F)
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        let mut f = f;
        unstable_sort::sort(self, &mut |a: &T, b: &T| f(a).lt(&f(b)));
    }

    /// Reorders the slice such that the element at `index` is at its final sorted position.
    pub fn select_nth_unstable(&mut self, index: usize) -> (&mut [T], &mut T, &mut [T])
    where
        T: Ord,
    {
        let len = self.len();
        if index >= len {
            crate::rt::panic_fmt(
                format_args!("partition_at_index index {index} greater than length of slice {len}"),
                file!(),
                line!(),
                column!(),
            );
        }
        unstable_sort::partition_at_index(self, index, |a: &T, b: &T| a.lt(b))
    }

    /// Reorders the slice with a comparator function such that the element at `index` is at its final sorted position.
    pub fn select_nth_unstable_by<F>(&mut self, index: usize, compare: F) -> (&mut [T], &mut T, &mut [T])
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        let len = self.len();
        if index >= len {
            crate::rt::panic_fmt(
                format_args!("partition_at_index index {index} greater than length of slice {len}"),
                file!(),
                line!(),
                column!(),
            );
        }
        let mut compare = compare;
        unstable_sort::partition_at_index(self, index, |a: &T, b: &T| compare(a, b) == Ordering::Less)
    }

    /// Reorders the slice with a key extraction function such that the element at `index` is at its final sorted position.
    pub fn select_nth_unstable_by_key<K, F>(&mut self, index: usize, mut f: F) -> (&mut [T], &mut T, &mut [T])
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        let len = self.len();
        if index >= len {
            crate::rt::panic_fmt(
                format_args!("partition_at_index index {index} greater than length of slice {len}"),
                file!(),
                line!(),
                column!(),
            );
        }
        unstable_sort::partition_at_index(self, index, |a: &T, b: &T| f(a).lt(&f(b)))
    }

    /// Checks if the elements of this slice are sorted.
    pub fn is_sorted(&self) -> bool
    where
        T: PartialOrd,
    {
        self.is_sorted_by(|a, b| a <= b)
    }

    /// Checks if the elements of this slice are sorted using the given comparator function.
    pub fn is_sorted_by<'a, F>(&'a self, compare: F) -> bool
    where
        F: FnMut(&'a T, &'a T) -> bool,
    {
        let mut compare = compare;
        let n = self.len();
        let mut i = 1;
        while i < n {
            if !compare(&self[i - 1], &self[i]) {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Checks if the elements of this slice are sorted using the given key extraction function.
    pub fn is_sorted_by_key<'a, F, K>(&'a self, f: F) -> bool
    where
        F: FnMut(&'a T) -> K,
        K: PartialOrd,
    {
        let mut f = f;
        let n = self.len();
        let mut i = 1;
        while i < n {
            if !(f(&self[i - 1]) <= f(&self[i])) {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Rotates the slice in-place such that the first `mid` elements move to the end.
    #[track_caller]
    pub fn rotate_left(&mut self, mid: usize) {
        let len = self.len();
        if mid > len {
            crate::rt::panic_str("assertion failed: mid <= self.len()", file!(), line!(), column!());
        }
        self[..mid].reverse();
        self[mid..].reverse();
        self.reverse();
    }

    /// Rotates the slice in-place such that the last `k` elements move to the front.
    #[track_caller]
    pub fn rotate_right(&mut self, k: usize) {
        let len = self.len();
        if k > len {
            crate::rt::panic_str("assertion failed: k <= self.len()", file!(), line!(), column!());
        }
        let mid = len - k;
        self[..mid].reverse();
        self[mid..].reverse();
        self.reverse();
    }

    /// Fills `self` with elements by cloning `value`.
    pub fn fill(&mut self, value: T)
    where
        T: Clone,
    {
        let n = self.len();
        let mut i = 0;
        while i < n {
            self[i] = value.clone();
            i += 1;
        }
    }

    /// Fills `self` with elements returned by calling a closure repeatedly.
    pub fn fill_with<F>(&mut self, f: F)
    where
        F: FnMut() -> T,
    {
        let mut f = f;
        let n = self.len();
        let mut i = 0;
        while i < n {
            self[i] = f();
            i += 1;
        }
    }

    /// Copies the elements from `src` into `self`.
    #[track_caller]
    pub fn clone_from_slice(&mut self, src: &[T])
    where
        T: Clone,
    {
        if self.len() != src.len() {
            crate::rt::panic_str("destination and source slices have different lengths", file!(), line!(), column!());
        }
        let n = self.len();
        let mut i = 0;
        while i < n {
            self[i].clone_from(&src[i]);
            i += 1;
        }
    }

    /// Copies all elements from `src` into `self`, using a memcpy.
    #[track_caller]
    pub fn copy_from_slice(&mut self, src: &[T])
    where
        T: Copy,
    {
        let (a, b) = (self.len(), src.len());
        if a != b {
            crate::rt::panic_fmt(
                format_args!("copy_from_slice: source slice length ({b}) does not match destination slice length ({a})"),
                file!(),
                line!(),
                column!(),
            );
        }
        unsafe { intrinsics::copy_nonoverlapping(src.as_ptr(), self.as_mut_ptr(), a) }
    }

    /// Copies elements from one part of the slice to another part of itself, using a memmove.
    #[track_caller]
    pub fn copy_within<R: crate::ops::RangeBounds<usize>>(&mut self, src: R, dest: usize)
    where
        T: Copy,
    {
        let len = self.len();
        let r = range(src, len);
        let count = r.end - r.start;
        if dest > len - count {
            crate::rt::panic_str("dest is out of bounds", file!(), line!(), column!());
        }
        unsafe {
            let p = self.as_mut_ptr();
            intrinsics::copy(p.add(r.start) as *const T, p.add(dest), count);
        }
    }

    /// Swaps all elements in `self` with those in `other`.
    #[track_caller]
    pub fn swap_with_slice(&mut self, other: &mut [T]) {
        if self.len() != other.len() {
            crate::rt::panic_str("destination and source slices have different lengths", file!(), line!(), column!());
        }
        unsafe { crate::ptr::swap_nonoverlapping(self.as_mut_ptr(), other.as_mut_ptr(), self.len()) }
    }

    /// Copies `self` into a new `Vec`.
    #[inline]
    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        let n = self.len();
        let mut v = Vec::with_capacity(n);
        let mut i = 0;
        while i < n {
            v.push(self[i].clone());
            i += 1;
        }
        v
    }

    /// Converts `self` into a vector without clones or allocation.
    #[inline]
    pub fn into_vec(self: Box<Self>) -> Vec<T> {
        Vec::from(self)
    }

    /// Creates a vector by copying a slice `n` times.
    pub fn repeat(&self, n: usize) -> Vec<T>
    where
        T: Copy,
    {
        let len = self.len();
        let mut v = Vec::with_capacity(len * n);
        let mut i = 0;
        while i < n {
            v.extend_from_slice(self);
            i += 1;
        }
        v
    }

    /// Flattens a slice of `T` into a single value `Self::Output`.
    #[inline]
    pub fn concat(&self) -> <Self as Concat>::Output
    where
        Self: Concat,
    {
        Concat::concat(self)
    }

    /// Flattens a slice of `T` into a single value `Self::Output`, placing a given separator between each.
    #[inline]
    pub fn join<Separator>(&self, sep: Separator) -> <Self as Join<Separator>>::Output
    where
        Self: Join<Separator>,
    {
        Join::join(self, sep)
    }

    /// Deprecated alias of `join`.
    #[inline]
    pub fn connect<Separator>(&self, sep: Separator) -> <Self as Join<Separator>>::Output
    where
        Self: Join<Separator>,
    {
        Join::join(self, sep)
    }
}

impl<T, const N: usize> [[T; N]] {
    /// Takes a `&[[T; N]]`, and flattens it to a `&[T]`.
    #[inline]
    pub fn as_flattened(&self) -> &[T] {
        unsafe { from_raw_parts(self.as_ptr() as *const T, self.len() * N) }
    }

    /// Takes a `&mut [[T; N]]`, and flattens it to a `&mut [T]`.
    #[inline]
    pub fn as_flattened_mut(&mut self) -> &mut [T] {
        let len = self.len() * N;
        unsafe { from_raw_parts_mut(self.as_mut_ptr() as *mut T, len) }
    }
}

impl [u8] {
    /// Returns an iterator over chunks of valid UTF-8 and invalid bytes.
    pub fn utf8_chunks(&self) -> crate::str::Utf8Chunks<'_> {
        crate::str::Utf8Chunks::new(self)
    }

    /// Returns an iterator that produces an escaped version of this slice.
    pub fn escape_ascii(&self) -> crate::ascii::EscapeAscii<'_> {
        crate::ascii::escape_ascii(self)
    }

    /// Checks if all bytes in this slice are within the ASCII range.
    #[inline]
    pub fn is_ascii(&self) -> bool {
        let n = self.len();
        let mut i = 0;
        while i < n {
            if self[i] >= 0x80 {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Checks that two slices are an ASCII case-insensitive match.
    #[inline]
    pub fn eq_ignore_ascii_case(&self, other: &[u8]) -> bool {
        if self.len() != other.len() {
            return false;
        }
        let n = self.len();
        let mut i = 0;
        while i < n {
            if self[i].to_ascii_lowercase() != other[i].to_ascii_lowercase() {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Converts this slice to its ASCII upper case equivalent in-place.
    #[inline]
    pub fn make_ascii_uppercase(&mut self) {
        let n = self.len();
        let mut i = 0;
        while i < n {
            self[i] = self[i].to_ascii_uppercase();
            i += 1;
        }
    }

    /// Converts this slice to its ASCII lower case equivalent in-place.
    #[inline]
    pub fn make_ascii_lowercase(&mut self) {
        let n = self.len();
        let mut i = 0;
        while i < n {
            self[i] = self[i].to_ascii_lowercase();
            i += 1;
        }
    }

    /// Returns a vector containing a copy of this slice where each byte is mapped to its ASCII upper case equivalent.
    #[inline]
    pub fn to_ascii_uppercase(&self) -> Vec<u8> {
        let mut v = self.to_vec();
        v.make_ascii_uppercase();
        v
    }

    /// Returns a vector containing a copy of this slice where each byte is mapped to its ASCII lower case equivalent.
    #[inline]
    pub fn to_ascii_lowercase(&self) -> Vec<u8> {
        let mut v = self.to_vec();
        v.make_ascii_lowercase();
        v
    }

    /// Returns a byte slice with leading and trailing ASCII whitespace bytes removed.
    #[inline]
    pub fn trim_ascii(&self) -> &[u8] {
        self.trim_ascii_start().trim_ascii_end()
    }

    /// Returns a byte slice with leading ASCII whitespace bytes removed.
    #[inline]
    pub fn trim_ascii_start(&self) -> &[u8] {
        let n = self.len();
        let mut i = 0;
        while i < n && self[i].is_ascii_whitespace() {
            i += 1;
        }
        &self[i..]
    }

    /// Returns a byte slice with trailing ASCII whitespace bytes removed.
    #[inline]
    pub fn trim_ascii_end(&self) -> &[u8] {
        let mut j = self.len();
        while j > 0 && self[j - 1].is_ascii_whitespace() {
            j -= 1;
        }
        &self[..j]
    }
}

// ---------------------------------------------------------------------------
// concat / join

/// Helper trait for `[T]::concat`.
pub trait Concat {
    /// The resulting type after concatenation.
    type Output;
    /// Implementation of `[T]::concat`.
    fn concat(slice: &Self) -> Self::Output;
}

/// Helper trait for `[T]::join`.
pub trait Join<Separator> {
    /// The resulting type after concatenation.
    type Output;
    /// Implementation of `[T]::join`.
    fn join(slice: &Self, sep: Separator) -> Self::Output;
}

macro_rules! __std_slice_str_concat {
    ($($t:ty => $conv:ident),*) => {
        $(
            impl<'x> Concat for [$t] {
                type Output = String;
                fn concat(slice: &Self) -> String {
                    let n = slice.len();
                    let mut total = 0;
                    let mut i = 0;
                    while i < n {
                        total += slice[i].$conv().len();
                        i += 1;
                    }
                    let mut s = String::with_capacity(total);
                    let mut i = 0;
                    while i < n {
                        s.push_str(slice[i].$conv());
                        i += 1;
                    }
                    s
                }
            }

            impl<'x, 'y> Join<&'y str> for [$t] {
                type Output = String;
                fn join(slice: &Self, sep: &'y str) -> String {
                    let n = slice.len();
                    if n == 0 {
                        return String::new();
                    }
                    let mut total = sep.len() * (n - 1);
                    let mut i = 0;
                    while i < n {
                        total += slice[i].$conv().len();
                        i += 1;
                    }
                    let mut s = String::with_capacity(total);
                    let mut i = 0;
                    while i < n {
                        if i > 0 {
                            s.push_str(sep);
                        }
                        s.push_str(slice[i].$conv());
                        i += 1;
                    }
                    s
                }
            }

            impl<'x, 'y> Join<&'y String> for [$t] {
                type Output = String;
                fn join(slice: &Self, sep: &'y String) -> String {
                    Join::join(slice, sep.as_str())
                }
            }

            impl<'x> Join<char> for [$t] {
                type Output = String;
                fn join(slice: &Self, sep: char) -> String {
                    let mut buf = [0u8; 4];
                    let sep = sep.encode_utf8(&mut buf);
                    Join::join(slice, &*sep)
                }
            }
        )*
    };
}

#[inline]
fn __id_str(s: &str) -> &str {
    s
}

trait AsStrHelper {
    fn __as_str(&self) -> &str;
}

impl AsStrHelper for String {
    #[inline]
    fn __as_str(&self) -> &str {
        self.as_str()
    }
}

impl<'x> AsStrHelper for &'x str {
    #[inline]
    fn __as_str(&self) -> &str {
        __id_str(*self)
    }
}

impl<'x> AsStrHelper for &'x String {
    #[inline]
    fn __as_str(&self) -> &str {
        self.as_str()
    }
}

impl<'x> AsStrHelper for &'x mut str {
    #[inline]
    fn __as_str(&self) -> &str {
        &**self
    }
}

impl AsStrHelper for Box<str> {
    #[inline]
    fn __as_str(&self) -> &str {
        &**self
    }
}

impl<'x> AsStrHelper for crate::borrow::Cow<'x, str> {
    #[inline]
    fn __as_str(&self) -> &str {
        &**self
    }
}

__std_slice_str_concat! {
    String => __as_str,
    &'x str => __as_str,
    &'x String => __as_str,
    &'x mut str => __as_str,
    Box<str> => __as_str,
    crate::borrow::Cow<'x, str> => __as_str
}

macro_rules! __std_slice_vec_concat {
    ($($t:ty),*) => {
        $(
            impl<'x, T: Clone> Concat for [$t] {
                type Output = Vec<T>;
                fn concat(slice: &Self) -> Vec<T> {
                    let n = slice.len();
                    let mut total = 0;
                    let mut i = 0;
                    while i < n {
                        total += slice[i].len();
                        i += 1;
                    }
                    let mut v = Vec::with_capacity(total);
                    let mut i = 0;
                    while i < n {
                        v.extend_from_slice(&slice[i][..]);
                        i += 1;
                    }
                    v
                }
            }

            impl<'x, 'y, T: Clone> Join<&'y T> for [$t] {
                type Output = Vec<T>;
                fn join(slice: &Self, sep: &'y T) -> Vec<T> {
                    let n = slice.len();
                    let mut v = Vec::new();
                    let mut i = 0;
                    while i < n {
                        if i > 0 {
                            v.push(sep.clone());
                        }
                        v.extend_from_slice(&slice[i][..]);
                        i += 1;
                    }
                    v
                }
            }

            impl<'x, 'y, T: Clone> Join<&'y [T]> for [$t] {
                type Output = Vec<T>;
                fn join(slice: &Self, sep: &'y [T]) -> Vec<T> {
                    let n = slice.len();
                    let mut v = Vec::new();
                    let mut i = 0;
                    while i < n {
                        if i > 0 {
                            v.extend_from_slice(sep);
                        }
                        v.extend_from_slice(&slice[i][..]);
                        i += 1;
                    }
                    v
                }
            }
        )*
    };
}

__std_slice_vec_concat! { Vec<T>, &'x [T], &'x mut [T], &'x Vec<T>, Box<[T]>, crate::borrow::Cow<'x, [T]>, crate::rc::Rc<[T]>, crate::sync::Arc<[T]> }

impl<T: Clone, const N: usize> Concat for [[T; N]] {
    type Output = Vec<T>;
    fn concat(slice: &Self) -> Vec<T> {
        let n = slice.len();
        let mut v = Vec::with_capacity(n * N);
        let mut i = 0;
        while i < n {
            v.extend_from_slice(&slice[i][..]);
            i += 1;
        }
        v
    }
}

impl<'y, T: Clone, const N: usize> Join<&'y T> for [[T; N]] {
    type Output = Vec<T>;
    fn join(slice: &Self, sep: &'y T) -> Vec<T> {
        let n = slice.len();
        let mut v = Vec::new();
        let mut i = 0;
        while i < n {
            if i > 0 {
                v.push(sep.clone());
            }
            v.extend_from_slice(&slice[i][..]);
            i += 1;
        }
        v
    }
}
