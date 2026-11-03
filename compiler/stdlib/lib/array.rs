//! Utilities for the array primitive type.

use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::intrinsics;
use crate::iter::{DoubleEndedIterator, ExactSizeIterator, FusedIterator, IntoIterator, Iterator};
use crate::mem::ManuallyDrop;
use crate::ops::{Index, IndexMut};

/// Creates an array of type `[T; N]` where each element `T` is the returned value from `cb` using that element's index.
#[inline]
pub fn from_fn<T, const N: usize, F>(cb: F) -> [T; N]
where
    F: FnMut(usize) -> T,
{
    let mut cb = cb;
    unsafe {
        let mut arr: ManuallyDrop<[T; N]> = ManuallyDrop::new(intrinsics::uninit::<[T; N]>());
        let p = &mut *arr as *mut [T; N] as *mut T;
        let mut i = 0;
        while i < N {
            intrinsics::write(intrinsics::ptr_add_mut(p, i), cb(i));
            i += 1;
        }
        ManuallyDrop::into_inner(arr)
    }
}

/// Creates an array of type `[T; N]` by repeatedly cloning a value.
#[inline]
pub fn repeat<T: Clone, const N: usize>(val: T) -> [T; N] {
    from_fn(|_| val.clone())
}

/// Converts a reference to `T` into a reference to an array of length 1 (without copying).
#[inline]
pub fn from_ref<T>(s: &T) -> &[T; 1] {
    unsafe { &*(s as *const T as *const [T; 1]) }
}

/// Converts a mutable reference to `T` into a mutable reference to an array of length 1 (without copying).
#[inline]
pub fn from_mut<T>(s: &mut T) -> &mut [T; 1] {
    unsafe { &mut *(s as *mut T as *mut [T; 1]) }
}

/// The error type returned when a conversion from a slice to an array fails.
#[derive(Debug, Copy, Clone)]
pub struct TryFromSliceError(());

impl fmt::Display for TryFromSliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("could not convert slice to array")
    }
}

impl crate::error::Error for TryFromSliceError {}

impl<T, const N: usize> [T; N] {
    /// Returns an array of the same size as `self`, with function `f` applied to each element in order.
    pub fn map<F, U>(self, f: F) -> [U; N]
    where
        F: FnMut(T) -> U,
    {
        let mut f = f;
        let src = ManuallyDrop::new(self);
        let p = &*src as *const [T; N] as *const T;
        from_fn(|i| f(unsafe { intrinsics::read(intrinsics::ptr_add(p, i)) }))
    }

    /// Returns a slice containing the entire array. Equivalent to `&s[..]`.
    #[inline]
    pub const fn as_slice(&self) -> &[T] {
        unsafe { &*intrinsics::slice_from_raw_parts(self as *const [T; N] as *const T, N) }
    }

    /// Returns a mutable slice containing the entire array. Equivalent to `&mut s[..]`.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { &mut *intrinsics::slice_from_raw_parts_mut(self as *mut [T; N] as *mut T, N) }
    }

    /// Borrows each element and returns an array of references with the same size as `self`.
    pub fn each_ref(&self) -> [&T; N] {
        let p = self as *const [T; N] as *const T;
        from_fn(|i| unsafe { &*intrinsics::ptr_add(p, i) })
    }

    /// Borrows each element mutably and returns an array of mutable references.
    pub fn each_mut(&mut self) -> [&mut T; N] {
        let p = self as *mut [T; N] as *mut T;
        from_fn(|i| unsafe { &mut *intrinsics::ptr_add_mut(p, i) })
    }
}

impl<T: Clone, const N: usize> Clone for [T; N] {
    #[inline]
    fn clone(&self) -> Self {
        let p = self as *const [T; N] as *const T;
        from_fn(|i| unsafe { (*intrinsics::ptr_add(p, i)).clone() })
    }
}

impl<T: Default, const N: usize> Default for [T; N] {
    #[inline]
    fn default() -> Self {
        from_fn(|_| T::default())
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for [T; N] {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_slice(), f)
    }
}

impl<T: Hash, const N: usize> Hash for [T; N] {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(self.as_slice(), state)
    }
}

impl<T, I, const N: usize> Index<I> for [T; N]
where
    [T]: Index<I>,
{
    type Output = <[T] as Index<I>>::Output;
    #[inline]
    #[track_caller]
    fn index(&self, index: I) -> &Self::Output {
        Index::index(self.as_slice(), index)
    }
}

impl<T, I, const N: usize> IndexMut<I> for [T; N]
where
    [T]: IndexMut<I>,
{
    #[inline]
    #[track_caller]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(self.as_mut_slice(), index)
    }
}

// ---------------------------------------------------------------------------
// Comparisons

impl<A, B, const N: usize> PartialEq<[B; N]> for [A; N]
where
    A: PartialEq<B>,
{
    #[inline]
    fn eq(&self, other: &[B; N]) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<A, B, const N: usize> PartialEq<[B]> for [A; N]
where
    A: PartialEq<B>,
{
    #[inline]
    fn eq(&self, other: &[B]) -> bool {
        self.as_slice() == other
    }
}

impl<A, B, const N: usize> PartialEq<[A; N]> for [B]
where
    B: PartialEq<A>,
{
    #[inline]
    fn eq(&self, other: &[A; N]) -> bool {
        self == other.as_slice()
    }
}

impl<'b, A, B, const N: usize> PartialEq<&'b [B]> for [A; N]
where
    A: PartialEq<B>,
{
    #[inline]
    fn eq(&self, other: &&'b [B]) -> bool {
        self.as_slice() == *other
    }
}

impl<'b, A, B, const N: usize> PartialEq<[A; N]> for &'b [B]
where
    B: PartialEq<A>,
{
    #[inline]
    fn eq(&self, other: &[A; N]) -> bool {
        *self == other.as_slice()
    }
}

impl<'b, A, B, const N: usize> PartialEq<&'b mut [B]> for [A; N]
where
    A: PartialEq<B>,
{
    #[inline]
    fn eq(&self, other: &&'b mut [B]) -> bool {
        self.as_slice() == &**other
    }
}

impl<'b, A, B, const N: usize> PartialEq<[A; N]> for &'b mut [B]
where
    B: PartialEq<A>,
{
    #[inline]
    fn eq(&self, other: &[A; N]) -> bool {
        &**self == other.as_slice()
    }
}

impl<T: Eq, const N: usize> Eq for [T; N] {}

impl<T: PartialOrd, const N: usize> PartialOrd for [T; N] {
    #[inline]
    fn partial_cmp(&self, other: &[T; N]) -> Option<Ordering> {
        PartialOrd::partial_cmp(self.as_slice(), other.as_slice())
    }
}

impl<T: Ord, const N: usize> Ord for [T; N] {
    #[inline]
    fn cmp(&self, other: &[T; N]) -> Ordering {
        Ord::cmp(self.as_slice(), other.as_slice())
    }
}

// ---------------------------------------------------------------------------
// Conversions

impl<T: Copy, const N: usize> TryFrom<&[T]> for [T; N] {
    type Error = TryFromSliceError;
    #[inline]
    fn try_from(slice: &[T]) -> Result<[T; N], TryFromSliceError> {
        if slice.len() == N {
            let p = slice.as_ptr();
            Ok(from_fn(|i| unsafe { *intrinsics::ptr_add(p, i) }))
        } else {
            Err(TryFromSliceError(()))
        }
    }
}

impl<T: Copy, const N: usize> TryFrom<&mut [T]> for [T; N] {
    type Error = TryFromSliceError;
    #[inline]
    fn try_from(slice: &mut [T]) -> Result<[T; N], TryFromSliceError> {
        if slice.len() == N {
            let p = slice.as_ptr();
            Ok(from_fn(|i| unsafe { *intrinsics::ptr_add(p, i) }))
        } else {
            Err(TryFromSliceError(()))
        }
    }
}

impl<'a, T, const N: usize> TryFrom<&'a [T]> for &'a [T; N] {
    type Error = TryFromSliceError;
    #[inline]
    fn try_from(slice: &'a [T]) -> Result<&'a [T; N], TryFromSliceError> {
        if slice.len() == N {
            Ok(unsafe { &*(slice.as_ptr() as *const [T; N]) })
        } else {
            Err(TryFromSliceError(()))
        }
    }
}

impl<'a, T, const N: usize> TryFrom<&'a mut [T]> for &'a mut [T; N] {
    type Error = TryFromSliceError;
    #[inline]
    fn try_from(slice: &'a mut [T]) -> Result<&'a mut [T; N], TryFromSliceError> {
        if slice.len() == N {
            Ok(unsafe { &mut *(slice.as_mut_ptr() as *mut [T; N]) })
        } else {
            Err(TryFromSliceError(()))
        }
    }
}

// ---------------------------------------------------------------------------
// Iteration

/// A by-value array iterator.
pub struct IntoIter<T, const N: usize> {
    data: ManuallyDrop<[T; N]>,
    start: usize,
    end: usize,
}

impl<T, const N: usize> IntoIter<T, N> {
    /// Creates a new iterator over the given `array`.
    #[inline]
    pub fn new(array: [T; N]) -> Self {
        IntoIter { data: ManuallyDrop::new(array), start: 0, end: N }
    }

    #[inline]
    fn ptr(&self) -> *const T {
        &*self.data as *const [T; N] as *const T
    }

    /// Returns an immutable slice of all elements that have not been yielded yet.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        unsafe { crate::slice::from_raw_parts(intrinsics::ptr_add(self.ptr(), self.start), self.end - self.start) }
    }

    /// Returns a mutable slice of all elements that have not been yielded yet.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        let p = &mut *self.data as *mut [T; N] as *mut T;
        unsafe { crate::slice::from_raw_parts_mut(intrinsics::ptr_add_mut(p, self.start), self.end - self.start) }
    }
}

impl<T, const N: usize> Iterator for IntoIter<T, N> {
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        if self.start < self.end {
            let i = self.start;
            self.start = i + 1;
            unsafe { Some(intrinsics::read(intrinsics::ptr_add(self.ptr(), i))) }
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

impl<T, const N: usize> DoubleEndedIterator for IntoIter<T, N> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        if self.start < self.end {
            self.end -= 1;
            unsafe { Some(intrinsics::read(intrinsics::ptr_add(self.ptr(), self.end))) }
        } else {
            None
        }
    }
}

impl<T, const N: usize> ExactSizeIterator for IntoIter<T, N> {
    #[inline]
    fn len(&self) -> usize {
        self.end - self.start
    }
}

impl<T, const N: usize> FusedIterator for IntoIter<T, N> {}

impl<T, const N: usize> Drop for IntoIter<T, N> {
    fn drop(&mut self) {
        unsafe {
            intrinsics::drop_in_place(self.as_mut_slice() as *mut [T]);
        }
    }
}

impl<T: Clone, const N: usize> Clone for IntoIter<T, N> {
    fn clone(&self) -> Self {
        // Clone the remaining elements into a fresh array, placed at the front.
        let mut new: IntoIter<T, N> = IntoIter {
            data: ManuallyDrop::new(unsafe { intrinsics::uninit::<[T; N]>() }),
            start: 0,
            end: 0,
        };
        let src = self.as_slice();
        let dst = &mut *new.data as *mut [T; N] as *mut T;
        let n = src.len();
        let mut i = 0;
        while i < n {
            unsafe { intrinsics::write(intrinsics::ptr_add_mut(dst, i), src[i].clone()) };
            new.end = i + 1;
            i += 1;
        }
        new
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for IntoIter<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoIter").field(&self.as_slice()).finish()
    }
}

impl<T, const N: usize> IntoIterator for [T; N] {
    #[inline]
    fn __flatten_bounds() -> (usize, Option<usize>) {
        (N, Some(N))
    }
    type Item = T;
    type IntoIter = IntoIter<T, N>;
    #[inline]
    fn into_iter(self) -> IntoIter<T, N> {
        IntoIter::new(self)
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a [T; N] {
    #[inline]
    fn __flatten_bounds() -> (usize, Option<usize>) {
        (N, Some(N))
    }
    type Item = &'a T;
    type IntoIter = crate::slice::Iter<'a, T>;
    #[inline]
    fn into_iter(self) -> crate::slice::Iter<'a, T> {
        self.as_slice().iter()
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a mut [T; N] {
    #[inline]
    fn __flatten_bounds() -> (usize, Option<usize>) {
        (N, Some(N))
    }
    type Item = &'a mut T;
    type IntoIter = crate::slice::IterMut<'a, T>;
    #[inline]
    fn into_iter(self) -> crate::slice::IterMut<'a, T> {
        self.as_mut_slice().iter_mut()
    }
}
