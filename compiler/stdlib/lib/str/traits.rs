//! Trait implementations for `str`.

use crate::cmp::Ordering;
use crate::fmt;
use crate::intrinsics;
use crate::ops::{Index, IndexMut, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};
use crate::slice::SliceIndex;

use super::{from_raw_parts_mut, slice_error_fail};

/// Parse a value from a string.
pub trait FromStr: Sized {
    /// The associated error which can be returned from parsing.
    type Err;
    /// Parses a string `s` to return a value of this type.
    fn from_str(s: &str) -> Result<Self, Self::Err>;
}

/// An error returned when parsing a `bool` using `from_str` fails.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseBoolError;

impl fmt::Display for ParseBoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("provided string was not `true` or `false`")
    }
}

impl crate::error::Error for ParseBoolError {}

impl FromStr for bool {
    type Err = ParseBoolError;
    #[inline]
    fn from_str(s: &str) -> Result<bool, ParseBoolError> {
        if s == "true" {
            Ok(true)
        } else if s == "false" {
            Ok(false)
        } else {
            Err(ParseBoolError)
        }
    }
}

// ---------------------------------------------------------------------------
// Comparison

impl PartialEq for str {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        let n = self.len();
        n == other.len() && unsafe { intrinsics::compare_bytes(self.as_ptr(), other.as_ptr(), n) == 0 }
    }
}

impl Eq for str {}

impl PartialOrd for str {
    #[inline]
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for str {
    #[inline]
    fn cmp(&self, other: &str) -> Ordering {
        let a = self.len();
        let b = other.len();
        let n = if a < b { a } else { b };
        let r = unsafe { intrinsics::compare_bytes(self.as_ptr(), other.as_ptr(), n) };
        if r < 0 {
            Ordering::Less
        } else if r > 0 {
            Ordering::Greater
        } else {
            a.cmp(&b)
        }
    }
}

impl Default for &mut str {
    fn default() -> Self {
        unsafe { from_raw_parts_mut(1usize as *mut u8, 0) }
    }
}

// ---------------------------------------------------------------------------
// Indexing

#[inline(never)]
#[track_caller]

impl SliceIndex<str> for RangeFull {
    type Output = str;
    #[inline]
    fn get(self, slice: &str) -> Option<&str> {
        Some(slice)
    }
    #[inline]
    fn get_mut(self, slice: &mut str) -> Option<&mut str> {
        Some(slice)
    }
    #[inline]
    unsafe fn get_unchecked(self, slice: *const str) -> *const str {
        slice
    }
    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut str) -> *mut str {
        slice
    }
    #[inline]
    fn index(self, slice: &str) -> &str {
        slice
    }
    #[inline]
    fn index_mut(self, slice: &mut str) -> &mut str {
        slice
    }
}

impl SliceIndex<str> for Range<usize> {
    type Output = str;
    #[inline]
    fn get(self, slice: &str) -> Option<&str> {
        if self.start <= self.end && slice.is_char_boundary(self.start) && slice.is_char_boundary(self.end) {
            Some(unsafe { &*self.get_unchecked(slice as *const str) })
        } else {
            None
        }
    }
    #[inline]
    fn get_mut(self, slice: &mut str) -> Option<&mut str> {
        if self.start <= self.end && slice.is_char_boundary(self.start) && slice.is_char_boundary(self.end) {
            Some(unsafe { &mut *self.get_unchecked_mut(slice as *mut str) })
        } else {
            None
        }
    }
    #[inline]
    unsafe fn get_unchecked(self, slice: *const str) -> *const str {
        let p = intrinsics::str_ptr(slice);
        intrinsics::str_from_raw_parts(intrinsics::ptr_add(p, self.start), self.end - self.start)
    }
    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut str) -> *mut str {
        let p = intrinsics::str_ptr(slice as *const str);
        intrinsics::str_from_raw_parts(intrinsics::ptr_add(p, self.start), self.end - self.start) as *mut str
    }
    #[inline]
    #[track_caller]
    fn index(self, slice: &str) -> &str {
        let (start, end) = (self.start, self.end);
        match self.get(slice) {
            Some(s) => s,
            None => slice_error_fail(slice, start, end),
        }
    }
    #[inline]
    #[track_caller]
    fn index_mut(self, slice: &mut str) -> &mut str {
        let (start, end) = (self.start, self.end);
        if start <= end && slice.is_char_boundary(start) && slice.is_char_boundary(end) {
            unsafe { &mut *self.get_unchecked_mut(slice as *mut str) }
        } else {
            slice_error_fail(slice, start, end)
        }
    }
}

impl SliceIndex<str> for RangeTo<usize> {
    type Output = str;
    #[inline]
    fn get(self, slice: &str) -> Option<&str> {
        (0..self.end).get(slice)
    }
    #[inline]
    fn get_mut(self, slice: &mut str) -> Option<&mut str> {
        (0..self.end).get_mut(slice)
    }
    #[inline]
    unsafe fn get_unchecked(self, slice: *const str) -> *const str {
        (0..self.end).get_unchecked(slice)
    }
    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut str) -> *mut str {
        (0..self.end).get_unchecked_mut(slice)
    }
    #[inline]
    #[track_caller]
    fn index(self, slice: &str) -> &str {
        (0..self.end).index(slice)
    }
    #[inline]
    #[track_caller]
    fn index_mut(self, slice: &mut str) -> &mut str {
        (0..self.end).index_mut(slice)
    }
}

impl SliceIndex<str> for RangeFrom<usize> {
    type Output = str;
    #[inline]
    fn get(self, slice: &str) -> Option<&str> {
        let len = slice.len();
        (self.start..len).get(slice)
    }
    #[inline]
    fn get_mut(self, slice: &mut str) -> Option<&mut str> {
        let len = slice.len();
        (self.start..len).get_mut(slice)
    }
    #[inline]
    unsafe fn get_unchecked(self, slice: *const str) -> *const str {
        let len = intrinsics::str_len(slice);
        (self.start..len).get_unchecked(slice)
    }
    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut str) -> *mut str {
        let len = intrinsics::str_len(slice as *const str);
        (self.start..len).get_unchecked_mut(slice)
    }
    #[inline]
    #[track_caller]
    fn index(self, slice: &str) -> &str {
        let len = slice.len();
        (self.start..len).index(slice)
    }
    #[inline]
    #[track_caller]
    fn index_mut(self, slice: &mut str) -> &mut str {
        let len = slice.len();
        (self.start..len).index_mut(slice)
    }
}

impl SliceIndex<str> for RangeInclusive<usize> {
    type Output = str;
    #[inline]
    fn get(self, slice: &str) -> Option<&str> {
        if self.end == usize::MAX {
            None
        } else {
            (self.start..self.end + 1).get(slice)
        }
    }
    #[inline]
    fn get_mut(self, slice: &mut str) -> Option<&mut str> {
        if self.end == usize::MAX {
            None
        } else {
            (self.start..self.end + 1).get_mut(slice)
        }
    }
    #[inline]
    unsafe fn get_unchecked(self, slice: *const str) -> *const str {
        (self.start..self.end + 1).get_unchecked(slice)
    }
    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut str) -> *mut str {
        (self.start..self.end + 1).get_unchecked_mut(slice)
    }
    #[inline]
    #[track_caller]
    fn index(self, slice: &str) -> &str {
        let RangeInclusive { mut start, mut end, exhausted } = self;
        let len = slice.len();
        if end < len {
            end = end + 1;
            start = if exhausted { end } else { start };
            if start <= end && slice.is_char_boundary(start) && slice.is_char_boundary(end) {
                return unsafe { &*(start..end).get_unchecked(slice as *const str) };
            }
        }
        slice_error_fail(slice, start, end)
    }
    #[inline]
    #[track_caller]
    fn index_mut(self, slice: &mut str) -> &mut str {
        let RangeInclusive { mut start, mut end, exhausted } = self;
        let len = slice.len();
        if end < len {
            end = end + 1;
            start = if exhausted { end } else { start };
            if start <= end && slice.is_char_boundary(start) && slice.is_char_boundary(end) {
                return unsafe { &mut *(start..end).get_unchecked_mut(slice as *mut str) };
            }
        }
        slice_error_fail(slice, start, end)
    }
}

impl SliceIndex<str> for RangeToInclusive<usize> {
    type Output = str;
    #[inline]
    fn get(self, slice: &str) -> Option<&str> {
        (0..=self.end).get(slice)
    }
    #[inline]
    fn get_mut(self, slice: &mut str) -> Option<&mut str> {
        (0..=self.end).get_mut(slice)
    }
    #[inline]
    unsafe fn get_unchecked(self, slice: *const str) -> *const str {
        (0..=self.end).get_unchecked(slice)
    }
    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut str) -> *mut str {
        (0..=self.end).get_unchecked_mut(slice)
    }
    #[inline]
    #[track_caller]
    fn index(self, slice: &str) -> &str {
        (0..=self.end).index(slice)
    }
    #[inline]
    #[track_caller]
    fn index_mut(self, slice: &mut str) -> &mut str {
        (0..=self.end).index_mut(slice)
    }
}

impl SliceIndex<str> for (crate::ops::Bound<usize>, crate::ops::Bound<usize>) {
    type Output = str;
    #[inline]
    fn get(self, slice: &str) -> Option<&str> {
        crate::slice::bounds_into_range(slice.len(), self)?.get(slice)
    }
    #[inline]
    fn get_mut(self, slice: &mut str) -> Option<&mut str> {
        crate::slice::bounds_into_range(slice.len(), self)?.get_mut(slice)
    }
    #[inline]
    unsafe fn get_unchecked(self, slice: *const str) -> *const str {
        let len = intrinsics::slice_len(slice as *const [u8]);
        crate::slice::bounds_into_range_unchecked(len, self).get_unchecked(slice)
    }
    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut str) -> *mut str {
        let len = intrinsics::slice_len(slice as *const [u8]);
        crate::slice::bounds_into_range_unchecked(len, self).get_unchecked_mut(slice)
    }
    #[inline]
    #[track_caller]
    fn index(self, slice: &str) -> &str {
        crate::slice::bounds_into_slice_range(slice.len(), self).index(slice)
    }
    #[inline]
    #[track_caller]
    fn index_mut(self, slice: &mut str) -> &mut str {
        crate::slice::bounds_into_slice_range(slice.len(), self).index_mut(slice)
    }
}

impl<I: SliceIndex<str>> Index<I> for str {
    type Output = I::Output;
    #[inline]
    #[track_caller]
    fn index(&self, index: I) -> &I::Output {
        index.index(self)
    }
}

impl<I: SliceIndex<str>> IndexMut<I> for str {
    #[inline]
    #[track_caller]
    fn index_mut(&mut self, index: I) -> &mut I::Output {
        index.index_mut(self)
    }
}
