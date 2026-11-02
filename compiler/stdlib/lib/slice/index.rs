//! Indexing slices: the `SliceIndex` trait.

use crate::intrinsics;
use crate::ops::{Bound, Index, IndexMut, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};

/// A helper trait used for indexing operations on slices and string slices.
pub trait SliceIndex<T: ?Sized> {
    /// The output type returned by methods.
    type Output: ?Sized;

    /// Returns a shared reference to the output at this location, if in bounds.
    fn get(self, slice: &T) -> Option<&Self::Output>;

    /// Returns a mutable reference to the output at this location, if in bounds.
    fn get_mut(self, slice: &mut T) -> Option<&mut Self::Output>;

    /// Returns a pointer to the output at this location, without performing any bounds checking.
    unsafe fn get_unchecked(self, slice: *const T) -> *const Self::Output;

    /// Returns a mutable pointer to the output at this location, without performing any bounds checking.
    unsafe fn get_unchecked_mut(self, slice: *mut T) -> *mut Self::Output;

    /// Returns a shared reference to the output at this location, panicking if out of bounds.
    fn index(self, slice: &T) -> &Self::Output;

    /// Returns a mutable reference to the output at this location, panicking if out of bounds.
    fn index_mut(self, slice: &mut T) -> &mut Self::Output;
}

#[inline(never)]
#[track_caller]
pub(crate) fn index_out_of_bounds(index: usize, len: usize) -> ! {
    crate::rt::panic_fmt(
        format_args!("index out of bounds: the len is {len} but the index is {index}"),
        file!(),
        line!(),
        column!(),
    )
}

#[inline(never)]
#[track_caller]
pub(crate) fn slice_start_index_len_fail(index: usize, len: usize) -> ! {
    crate::rt::panic_fmt(
        format_args!("range start index {index} out of range for slice of length {len}"),
        file!(),
        line!(),
        column!(),
    )
}

#[inline(never)]
#[track_caller]
pub(crate) fn slice_end_index_len_fail(index: usize, len: usize) -> ! {
    crate::rt::panic_fmt(
        format_args!("range end index {index} out of range for slice of length {len}"),
        file!(),
        line!(),
        column!(),
    )
}

#[inline(never)]
#[track_caller]
pub(crate) fn slice_index_order_fail(index: usize, end: usize) -> ! {
    crate::rt::panic_fmt(
        format_args!("slice index starts at {index} but ends at {end}"),
        file!(),
        line!(),
        column!(),
    )
}

#[inline(never)]
#[track_caller]
pub(crate) fn slice_end_index_overflow_fail() -> ! {
    crate::rt::panic_str("attempted to index slice up to maximum usize", file!(), line!(), column!())
}

#[inline]
unsafe fn sub_ptr<T>(slice: *const [T], start: usize, end: usize) -> *const [T] {
    let p = intrinsics::slice_ptr(slice);
    intrinsics::slice_from_raw_parts(intrinsics::ptr_add(p, start), end - start)
}

#[inline]
unsafe fn sub_ptr_mut<T>(slice: *mut [T], start: usize, end: usize) -> *mut [T] {
    let p = intrinsics::slice_ptr(slice as *const [T]) as *mut T;
    intrinsics::slice_from_raw_parts_mut(intrinsics::ptr_add_mut(p, start), end - start)
}

impl<T> SliceIndex<[T]> for usize {
    type Output = T;
    #[inline]
    fn get(self, slice: &[T]) -> Option<&T> {
        if self < slice.len() {
            unsafe { Some(&*self.get_unchecked(slice as *const [T])) }
        } else {
            None
        }
    }
    #[inline]
    fn get_mut(self, slice: &mut [T]) -> Option<&mut T> {
        if self < slice.len() {
            unsafe { Some(&mut *self.get_unchecked_mut(slice as *mut [T])) }
        } else {
            None
        }
    }
    #[inline]
    unsafe fn get_unchecked(self, slice: *const [T]) -> *const T {
        intrinsics::ptr_add(intrinsics::slice_ptr(slice), self)
    }
    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut [T]) -> *mut T {
        intrinsics::ptr_add_mut(intrinsics::slice_ptr(slice as *const [T]) as *mut T, self)
    }
    #[inline]
    #[track_caller]
    fn index(self, slice: &[T]) -> &T {
        let len = slice.len();
        if self < len {
            unsafe { &*self.get_unchecked(slice as *const [T]) }
        } else {
            index_out_of_bounds(self, len)
        }
    }
    #[inline]
    #[track_caller]
    fn index_mut(self, slice: &mut [T]) -> &mut T {
        let len = slice.len();
        if self < len {
            unsafe { &mut *self.get_unchecked_mut(slice as *mut [T]) }
        } else {
            index_out_of_bounds(self, len)
        }
    }
}

impl<T> SliceIndex<[T]> for Range<usize> {
    type Output = [T];
    #[inline]
    fn get(self, slice: &[T]) -> Option<&[T]> {
        if self.start > self.end || self.end > slice.len() {
            None
        } else {
            unsafe { Some(&*self.get_unchecked(slice as *const [T])) }
        }
    }
    #[inline]
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> {
        if self.start > self.end || self.end > slice.len() {
            None
        } else {
            unsafe { Some(&mut *self.get_unchecked_mut(slice as *mut [T])) }
        }
    }
    #[inline]
    unsafe fn get_unchecked(self, slice: *const [T]) -> *const [T] {
        sub_ptr(slice, self.start, self.end)
    }
    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut [T]) -> *mut [T] {
        sub_ptr_mut(slice, self.start, self.end)
    }
    #[inline]
    #[track_caller]
    fn index(self, slice: &[T]) -> &[T] {
        let len = slice.len();
        if self.start > self.end {
            slice_index_order_fail(self.start, self.end);
        } else if self.end > len {
            slice_end_index_len_fail(self.end, len);
        }
        unsafe { &*self.get_unchecked(slice as *const [T]) }
    }
    #[inline]
    #[track_caller]
    fn index_mut(self, slice: &mut [T]) -> &mut [T] {
        let len = slice.len();
        if self.start > self.end {
            slice_index_order_fail(self.start, self.end);
        } else if self.end > len {
            slice_end_index_len_fail(self.end, len);
        }
        unsafe { &mut *self.get_unchecked_mut(slice as *mut [T]) }
    }
}

impl<T> SliceIndex<[T]> for RangeTo<usize> {
    type Output = [T];
    #[inline]
    fn get(self, slice: &[T]) -> Option<&[T]> {
        (0..self.end).get(slice)
    }
    #[inline]
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> {
        (0..self.end).get_mut(slice)
    }
    #[inline]
    unsafe fn get_unchecked(self, slice: *const [T]) -> *const [T] {
        (0..self.end).get_unchecked(slice)
    }
    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut [T]) -> *mut [T] {
        (0..self.end).get_unchecked_mut(slice)
    }
    #[inline]
    #[track_caller]
    fn index(self, slice: &[T]) -> &[T] {
        (0..self.end).index(slice)
    }
    #[inline]
    #[track_caller]
    fn index_mut(self, slice: &mut [T]) -> &mut [T] {
        (0..self.end).index_mut(slice)
    }
}

impl<T> SliceIndex<[T]> for RangeFrom<usize> {
    type Output = [T];
    #[inline]
    fn get(self, slice: &[T]) -> Option<&[T]> {
        let len = slice.len();
        (self.start..len).get(slice)
    }
    #[inline]
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> {
        let len = slice.len();
        (self.start..len).get_mut(slice)
    }
    #[inline]
    unsafe fn get_unchecked(self, slice: *const [T]) -> *const [T] {
        let len = intrinsics::slice_len(slice);
        (self.start..len).get_unchecked(slice)
    }
    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut [T]) -> *mut [T] {
        let len = intrinsics::slice_len(slice as *const [T]);
        (self.start..len).get_unchecked_mut(slice)
    }
    #[inline]
    #[track_caller]
    fn index(self, slice: &[T]) -> &[T] {
        let len = slice.len();
        if self.start > len {
            slice_start_index_len_fail(self.start, len);
        }
        unsafe { &*self.get_unchecked(slice as *const [T]) }
    }
    #[inline]
    #[track_caller]
    fn index_mut(self, slice: &mut [T]) -> &mut [T] {
        let len = slice.len();
        if self.start > len {
            slice_start_index_len_fail(self.start, len);
        }
        unsafe { &mut *self.get_unchecked_mut(slice as *mut [T]) }
    }
}

impl<T> SliceIndex<[T]> for RangeFull {
    type Output = [T];
    #[inline]
    fn get(self, slice: &[T]) -> Option<&[T]> {
        Some(slice)
    }
    #[inline]
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> {
        Some(slice)
    }
    #[inline]
    unsafe fn get_unchecked(self, slice: *const [T]) -> *const [T] {
        slice
    }
    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut [T]) -> *mut [T] {
        slice
    }
    #[inline]
    fn index(self, slice: &[T]) -> &[T] {
        slice
    }
    #[inline]
    fn index_mut(self, slice: &mut [T]) -> &mut [T] {
        slice
    }
}

impl<T> SliceIndex<[T]> for RangeInclusive<usize> {
    type Output = [T];
    #[inline]
    fn get(self, slice: &[T]) -> Option<&[T]> {
        if self.end == usize::MAX {
            None
        } else {
            (self.start..self.end + 1).get(slice)
        }
    }
    #[inline]
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> {
        if self.end == usize::MAX {
            None
        } else {
            (self.start..self.end + 1).get_mut(slice)
        }
    }
    #[inline]
    unsafe fn get_unchecked(self, slice: *const [T]) -> *const [T] {
        (self.start..self.end + 1).get_unchecked(slice)
    }
    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut [T]) -> *mut [T] {
        (self.start..self.end + 1).get_unchecked_mut(slice)
    }
    #[inline]
    #[track_caller]
    fn index(self, slice: &[T]) -> &[T] {
        if self.end == usize::MAX {
            slice_end_index_overflow_fail();
        }
        (self.start..self.end + 1).index(slice)
    }
    #[inline]
    #[track_caller]
    fn index_mut(self, slice: &mut [T]) -> &mut [T] {
        if self.end == usize::MAX {
            slice_end_index_overflow_fail();
        }
        (self.start..self.end + 1).index_mut(slice)
    }
}

impl<T> SliceIndex<[T]> for RangeToInclusive<usize> {
    type Output = [T];
    #[inline]
    fn get(self, slice: &[T]) -> Option<&[T]> {
        (0..=self.end).get(slice)
    }
    #[inline]
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> {
        (0..=self.end).get_mut(slice)
    }
    #[inline]
    unsafe fn get_unchecked(self, slice: *const [T]) -> *const [T] {
        (0..=self.end).get_unchecked(slice)
    }
    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut [T]) -> *mut [T] {
        (0..=self.end).get_unchecked_mut(slice)
    }
    #[inline]
    #[track_caller]
    fn index(self, slice: &[T]) -> &[T] {
        (0..=self.end).index(slice)
    }
    #[inline]
    #[track_caller]
    fn index_mut(self, slice: &mut [T]) -> &mut [T] {
        (0..=self.end).index_mut(slice)
    }
}

impl<T, I: SliceIndex<[T]>> Index<I> for [T] {
    type Output = I::Output;
    #[inline]
    #[track_caller]
    fn index(&self, index: I) -> &I::Output {
        index.index(self)
    }
}

impl<T, I: SliceIndex<[T]>> IndexMut<I> for [T] {
    #[inline]
    #[track_caller]
    fn index_mut(&mut self, index: I) -> &mut I::Output {
        index.index_mut(self)
    }
}

/// Performs bounds checking of a range, converting any `RangeBounds<usize>` to a `Range<usize>`.
#[track_caller]
pub fn range<R: crate::ops::RangeBounds<usize>>(range: R, len: usize) -> Range<usize> {
    let start = match range.start_bound() {
        Bound::Included(&start) => start,
        Bound::Excluded(&start) => match start.checked_add(1) {
            Some(s) => s,
            None => crate::rt::panic_str("attempted to index slice from after maximum usize", file!(), line!(), column!()),
        },
        Bound::Unbounded => 0,
    };
    let end = match range.end_bound() {
        Bound::Included(&end) => match end.checked_add(1) {
            Some(e) => e,
            None => slice_end_index_overflow_fail(),
        },
        Bound::Excluded(&end) => end,
        Bound::Unbounded => len,
    };
    if start > end {
        slice_index_order_fail(start, end);
    }
    if end > len {
        slice_end_index_len_fail(end, len);
    }
    Range { start, end }
}
