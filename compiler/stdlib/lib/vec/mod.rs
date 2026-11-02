//! A contiguous growable array type, `Vec<T>`.

mod drain;
mod into_iter;

pub use self::drain::Drain;
pub use self::into_iter::IntoIter;

use crate::alloc::{alloc_array, capacity_overflow, dealloc_array, realloc_array};
use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::intrinsics;
use crate::iter::{FromIterator, IntoIterator, Iterator};
use crate::marker::PhantomData;
use crate::mem;
use crate::ops::{Deref, DerefMut, Index, IndexMut, RangeBounds};
use crate::slice::{self, SliceIndex};

/// A contiguous growable array type.
pub struct Vec<T> {
    ptr: *mut T,
    cap: usize,
    len: usize,
    _marker: PhantomData<T>,
}

/// Creates a `Vec` containing `n` clones of `elem` (used by `vec![elem; n]`).
pub fn from_elem<T: Clone>(elem: T, n: usize) -> Vec<T> {
    let mut v = Vec::with_capacity(n);
    if n > 0 {
        let mut i = 1;
        while i < n {
            v.push(elem.clone());
            i += 1;
        }
        v.push(elem);
    }
    v
}

#[inline]
fn is_zst<T>() -> bool {
    mem::size_of::<T>() == 0
}

#[inline(never)]
#[track_caller]
fn assert_failed_insert(index: usize, len: usize) -> ! {
    crate::rt::panic_fmt(
        format_args!("insertion index (is {index}) should be <= len (is {len})"),
        file!(),
        line!(),
        column!(),
    )
}

#[inline(never)]
#[track_caller]
fn assert_failed_remove(index: usize, len: usize) -> ! {
    crate::rt::panic_fmt(
        format_args!("removal index (is {index}) should be < len (is {len})"),
        file!(),
        line!(),
        column!(),
    )
}

impl<T> Vec<T> {
    /// Constructs a new, empty `Vec<T>`.
    #[inline]
    pub const fn new() -> Vec<T> {
        Vec { ptr: mem::align_of::<T>() as *mut T, cap: if mem::size_of::<T>() == 0 { usize::MAX } else { 0 }, len: 0, _marker: PhantomData }
    }

    /// Constructs a new, empty `Vec<T>` with at least the specified capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Vec<T> {
        if is_zst::<T>() || capacity == 0 {
            return Vec::new();
        }
        let ptr = unsafe { alloc_array::<T>(capacity) };
        Vec { ptr, cap: capacity, len: 0, _marker: PhantomData }
    }

    /// Creates a `Vec<T>` directly from a pointer, a length, and a capacity.
    #[inline]
    pub unsafe fn from_raw_parts(ptr: *mut T, length: usize, capacity: usize) -> Vec<T> {
        let cap = if is_zst::<T>() { usize::MAX } else { capacity };
        Vec { ptr, cap, len: length, _marker: PhantomData }
    }

    /// Returns the total number of elements the vector can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.cap
    }

    fn grow_to(&mut self, new_cap: usize) {
        if is_zst::<T>() {
            capacity_overflow();
        }
        unsafe {
            self.ptr = realloc_array(self.ptr, self.cap, new_cap);
        }
        self.cap = new_cap;
    }

    fn grow_amortized(&mut self, additional: usize) {
        let required = match self.len.checked_add(additional) {
            Some(r) => r,
            None => capacity_overflow(),
        };
        let min_non_zero_cap = if mem::size_of::<T>() == 1 {
            8
        } else if mem::size_of::<T>() <= 1024 {
            4
        } else {
            1
        };
        let mut cap = self.cap.saturating_mul(2);
        if cap < required {
            cap = required;
        }
        if cap < min_non_zero_cap {
            cap = min_non_zero_cap;
        }
        self.grow_to(cap);
    }

    /// Reserves capacity for at least `additional` more elements.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        if self.cap - self.len < additional {
            self.grow_amortized(additional);
        }
    }

    /// Reserves the minimum capacity for at least `additional` more elements.
    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        if self.cap - self.len < additional {
            let required = match self.len.checked_add(additional) {
                Some(r) => r,
                None => capacity_overflow(),
            };
            self.grow_to(required);
        }
    }

    /// Tries to reserve capacity for at least `additional` more elements.
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), crate::collections::TryReserveError> {
        self.reserve(additional);
        Ok(())
    }

    /// Shrinks the capacity of the vector as much as possible.
    pub fn shrink_to_fit(&mut self) {
        if !is_zst::<T>() && self.cap > self.len {
            unsafe {
                self.ptr = realloc_array(self.ptr, self.cap, self.len);
            }
            self.cap = self.len;
        }
    }

    /// Shrinks the capacity of the vector with a lower bound.
    pub fn shrink_to(&mut self, min_capacity: usize) {
        let target = if self.len > min_capacity { self.len } else { min_capacity };
        if !is_zst::<T>() && self.cap > target {
            unsafe {
                self.ptr = realloc_array(self.ptr, self.cap, target);
            }
            self.cap = target;
        }
    }

    /// Converts the vector into `Box<[T]>`.
    pub fn into_boxed_slice(self) -> Box<[T]> {
        let mut me = self;
        me.shrink_to_fit();
        let p = me.ptr;
        let len = me.len;
        mem::forget(me);
        Box { ptr: crate::ptr::slice_from_raw_parts_mut(p, len) }
    }

    /// Shortens the vector, keeping the first `len` elements and dropping the rest.
    pub fn truncate(&mut self, len: usize) {
        if len >= self.len {
            return;
        }
        let remaining = self.len - len;
        unsafe {
            let tail = slice::from_raw_parts_mut(self.ptr.add(len), remaining) as *mut [T];
            self.len = len;
            intrinsics::drop_in_place(tail);
        }
    }

    /// Extracts a slice containing the entire vector.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr as *const T, self.len) }
    }

    /// Extracts a mutable slice of the entire vector.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    /// Returns a raw pointer to the vector's buffer.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.ptr as *const T
    }

    /// Returns a raw mutable pointer to the vector's buffer.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr
    }

    /// Forces the length of the vector to `new_len`.
    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        self.len = new_len;
    }

    /// Removes an element from the vector and returns it, replacing it with the last element.
    #[inline]
    #[track_caller]
    pub fn swap_remove(&mut self, index: usize) -> T {
        let len = self.len;
        if index >= len {
            crate::rt::panic_fmt(
                format_args!("swap_remove index (is {index}) should be < len (is {len})"),
                file!(),
                line!(),
                column!(),
            );
        }
        unsafe {
            let value = intrinsics::read(self.ptr.add(index) as *const T);
            let base = self.ptr;
            intrinsics::copy(base.add(len - 1) as *const T, base.add(index), 1);
            self.len = len - 1;
            value
        }
    }

    /// Inserts an element at position `index` within the vector, shifting all elements after it to the right.
    #[track_caller]
    pub fn insert(&mut self, index: usize, element: T) {
        let len = self.len;
        if index > len {
            assert_failed_insert(index, len);
        }
        if len == self.cap {
            self.grow_amortized(1);
        }
        unsafe {
            let p = self.ptr.add(index);
            if index < len {
                intrinsics::copy(p as *const T, p.add(1), len - index);
            }
            intrinsics::write(p, element);
            self.len = len + 1;
        }
    }

    /// Removes and returns the element at position `index` within the vector, shifting all elements after it to the left.
    #[track_caller]
    pub fn remove(&mut self, index: usize) -> T {
        let len = self.len;
        if index >= len {
            assert_failed_remove(index, len);
        }
        unsafe {
            let p = self.ptr.add(index);
            let ret = intrinsics::read(p as *const T);
            intrinsics::copy(p.add(1) as *const T, p, len - index - 1);
            self.len = len - 1;
            ret
        }
    }

    /// Retains only the elements specified by the predicate.
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        let mut f = f;
        self.retain_mut(|elem| f(elem));
    }

    /// Retains only the elements specified by the predicate, passing a mutable reference to it.
    pub fn retain_mut<F>(&mut self, f: F)
    where
        F: FnMut(&mut T) -> bool,
    {
        let mut f = f;
        let len = self.len;
        let p = self.ptr;
        let mut deleted = 0;
        self.len = 0;
        let mut i = 0;
        while i < len {
            unsafe {
                let cur = p.add(i);
                if !f(&mut *cur) {
                    intrinsics::drop_in_place(cur);
                    deleted += 1;
                } else if deleted > 0 {
                    intrinsics::copy_nonoverlapping(cur as *const T, p.add(i - deleted), 1);
                }
            }
            i += 1;
        }
        self.len = len - deleted;
    }

    /// Removes all but the first of consecutive elements in the vector that resolve to the same key.
    #[inline]
    pub fn dedup_by_key<F, K>(&mut self, key: F)
    where
        F: FnMut(&mut T) -> K,
        K: PartialEq,
    {
        let mut key = key;
        self.dedup_by(|a, b| key(a) == key(b))
    }

    /// Removes all but the first of consecutive elements in the vector satisfying a given equality relation.
    pub fn dedup_by<F>(&mut self, same_bucket: F)
    where
        F: FnMut(&mut T, &mut T) -> bool,
    {
        let mut same_bucket = same_bucket;
        let len = self.len;
        if len <= 1 {
            return;
        }
        let p = self.ptr;
        let mut w = 1;
        let mut r = 1;
        self.len = 0;
        while r < len {
            unsafe {
                let read = p.add(r);
                let prev = p.add(w - 1);
                if same_bucket(&mut *read, &mut *prev) {
                    intrinsics::drop_in_place(read);
                } else {
                    if r != w {
                        intrinsics::copy_nonoverlapping(read as *const T, p.add(w), 1);
                    }
                    w += 1;
                }
            }
            r += 1;
        }
        self.len = w;
    }

    /// Appends an element to the back of a collection.
    #[inline]
    pub fn push(&mut self, value: T) {
        if self.len == self.cap {
            self.grow_amortized(1);
        }
        unsafe {
            intrinsics::write(self.ptr.add(self.len), value);
        }
        self.len += 1;
    }

    /// Appends an element if there is sufficient spare capacity, otherwise an error is returned with the element.
    #[inline]
    pub fn push_within_capacity(&mut self, value: T) -> Result<(), T> {
        if self.len == self.cap {
            return Err(value);
        }
        self.push(value);
        Ok(())
    }

    /// Removes the last element from a vector and returns it, or `None` if it is empty.
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(intrinsics::read(self.ptr.add(self.len) as *const T)) }
        }
    }

    /// Removes and returns the last element from a vector if the predicate returns `true`.
    pub fn pop_if<F>(&mut self, predicate: F) -> Option<T>
    where
        F: FnOnce(&mut T) -> bool,
    {
        if self.len == 0 {
            return None;
        }
        let last = unsafe { &mut *self.ptr.add(self.len - 1) };
        if predicate(last) {
            self.pop()
        } else {
            None
        }
    }

    /// Moves all the elements of `other` into `self`, leaving `other` empty.
    pub fn append(&mut self, other: &mut Vec<T>) {
        let count = other.len;
        self.reserve(count);
        unsafe {
            intrinsics::copy_nonoverlapping(other.ptr as *const T, self.ptr.add(self.len), count);
        }
        self.len += count;
        other.len = 0;
    }

    /// Removes the specified range from the vector in bulk, returning all removed elements as an iterator.
    #[track_caller]
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, T>
    where
        R: RangeBounds<usize>,
    {
        let len = self.len;
        let r = slice::range(range, len);
        Drain::new(self, r.start, r.end)
    }

    /// Clears the vector, removing all values.
    #[inline]
    pub fn clear(&mut self) {
        self.truncate(0)
    }

    /// Returns the number of elements in the vector.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the vector contains no elements.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Splits the collection into two at the given index.
    #[track_caller]
    pub fn split_off(&mut self, at: usize) -> Vec<T> {
        let len = self.len;
        if at > len {
            crate::rt::panic_fmt(
                format_args!("`at` split index (is {at}) should be <= len (is {len})"),
                file!(),
                line!(),
                column!(),
            );
        }
        let other_len = len - at;
        let mut other = Vec::with_capacity(other_len);
        unsafe {
            intrinsics::copy_nonoverlapping(self.ptr.add(at) as *const T, other.ptr, other_len);
            self.len = at;
            other.len = other_len;
        }
        other
    }

    /// Resizes the `Vec` in-place so that `len` is equal to `new_len`, filling with the result of calling `f`.
    pub fn resize_with<F>(&mut self, new_len: usize, f: F)
    where
        F: FnMut() -> T,
    {
        let mut f = f;
        let len = self.len;
        if new_len > len {
            self.reserve(new_len - len);
            let mut i = len;
            while i < new_len {
                self.push(f());
                i += 1;
            }
        } else {
            self.truncate(new_len);
        }
    }

    /// Consumes and leaks the `Vec`, returning a mutable reference to the contents.
    pub fn leak<'a>(self) -> &'a mut [T] {
        let p = self.ptr;
        let len = self.len;
        mem::forget(self);
        unsafe { slice::from_raw_parts_mut(p, len) }
    }

    /// Returns the remaining spare capacity of the vector as a slice of `MaybeUninit<T>`.
    pub fn spare_capacity_mut(&mut self) -> &mut [mem::MaybeUninit<T>] {
        unsafe { slice::from_raw_parts_mut(self.ptr.add(self.len) as *mut mem::MaybeUninit<T>, self.cap - self.len) }
    }
}

impl<T: Clone> Vec<T> {
    /// Resizes the `Vec` in-place so that `len` is equal to `new_len`, filling with clones of `value`.
    pub fn resize(&mut self, new_len: usize, value: T) {
        let len = self.len;
        if new_len > len {
            let n = new_len - len;
            self.reserve(n);
            let mut i = 1;
            while i < n {
                self.push(value.clone());
                i += 1;
            }
            self.push(value);
        } else {
            self.truncate(new_len);
        }
    }

    /// Clones and appends all elements in a slice to the `Vec`.
    pub fn extend_from_slice(&mut self, other: &[T]) {
        let n = other.len();
        self.reserve(n);
        let mut i = 0;
        while i < n {
            unsafe {
                intrinsics::write(self.ptr.add(self.len), other[i].clone());
            }
            self.len += 1;
            i += 1;
        }
    }

    /// Copies elements from `src` range to the end of the vector.
    #[track_caller]
    pub fn extend_from_within<R>(&mut self, src: R)
    where
        R: RangeBounds<usize>,
    {
        let r = slice::range(src, self.len);
        self.reserve(r.end - r.start);
        let mut i = r.start;
        while i < r.end {
            let v = unsafe { (*self.ptr.add(i)).clone() };
            self.push(v);
            i += 1;
        }
    }
}

impl<T: PartialEq> Vec<T> {
    /// Removes consecutive repeated elements in the vector according to the `PartialEq` trait implementation.
    #[inline]
    pub fn dedup(&mut self) {
        self.dedup_by(|a, b| *a == *b)
    }
}

impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        unsafe {
            intrinsics::drop_in_place(slice::from_raw_parts_mut(self.ptr, self.len) as *mut [T]);
            if !is_zst::<T>() {
                dealloc_array(self.ptr, self.cap);
            }
        }
    }
}

impl<T> Deref for Vec<T> {
    type Target = [T];
    #[inline]
    fn deref(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr as *const T, self.len) }
    }
}

impl<T> DerefMut for Vec<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl<T: Clone> Clone for Vec<T> {
    fn clone(&self) -> Vec<T> {
        self.as_slice().to_vec()
    }

    fn clone_from(&mut self, source: &Self) {
        self.clear();
        self.extend_from_slice(source.as_slice());
    }
}

impl<T: Hash> Hash for Vec<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(self.as_slice(), state)
    }
}

impl<T, I: SliceIndex<[T]>> Index<I> for Vec<T> {
    type Output = I::Output;
    #[inline]
    #[track_caller]
    fn index(&self, index: I) -> &I::Output {
        index.index(self.as_slice())
    }
}

impl<T, I: SliceIndex<[T]>> IndexMut<I> for Vec<T> {
    #[inline]
    #[track_caller]
    fn index_mut(&mut self, index: I) -> &mut I::Output {
        index.index_mut(self.as_mut_slice())
    }
}

impl<T> FromIterator<T> for Vec<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Vec<T> {
        let mut it = iter.into_iter();
        let (lower, _) = it.size_hint();
        let mut v = Vec::with_capacity(lower);
        while let Some(x) = it.next() {
            v.push(x);
        }
        v
    }
}

impl<T> IntoIterator for Vec<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    #[inline]
    fn into_iter(self) -> IntoIter<T> {
        let me = mem::ManuallyDrop::new(self);
        IntoIter::new(me.ptr, me.cap, me.len)
    }
}

impl<'a, T> IntoIterator for &'a Vec<T> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;
    #[inline]
    fn into_iter(self) -> slice::Iter<'a, T> {
        self.as_slice().iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Vec<T> {
    type Item = &'a mut T;
    type IntoIter = slice::IterMut<'a, T>;
    #[inline]
    fn into_iter(self) -> slice::IterMut<'a, T> {
        self.as_mut_slice().iter_mut()
    }
}

impl<T> crate::iter::Extend<T> for Vec<T> {
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let mut it = iter.into_iter();
        let (lower, _) = it.size_hint();
        self.reserve(lower);
        while let Some(x) = it.next() {
            self.push(x);
        }
    }
    #[inline]
    fn extend_one(&mut self, item: T) {
        self.push(item);
    }
    #[inline]
    fn extend_reserve(&mut self, additional: usize) {
        self.reserve(additional);
    }
}

impl<'a, T: Copy + 'a> crate::iter::Extend<&'a T> for Vec<T> {
    #[inline]
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        let mut it = iter.into_iter();
        let (lower, _) = it.size_hint();
        self.reserve(lower);
        while let Some(x) = it.next() {
            self.push(*x);
        }
    }
    #[inline]
    fn extend_one(&mut self, item: &'a T) {
        self.push(*item);
    }
}

// ---------------------------------------------------------------------------
// Comparison

macro_rules! __std_vec_eq {
    ([$($vars:tt)*] $lhs:ty, $rhs:ty) => {
        impl<$($vars)* T, U> PartialEq<$rhs> for $lhs
        where
            T: PartialEq<U>,
        {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                self[..] == other[..]
            }
        }
    };
}

__std_vec_eq! { [] Vec<T>, Vec<U> }
__std_vec_eq! { ['b,] Vec<T>, &'b [U] }
__std_vec_eq! { ['b,] Vec<T>, &'b mut [U] }
__std_vec_eq! { ['b,] &'b [T], Vec<U> }
__std_vec_eq! { ['b,] &'b mut [T], Vec<U> }
__std_vec_eq! { [] Vec<T>, [U] }
__std_vec_eq! { [] [T], Vec<U> }
__std_vec_eq! { [const N: usize,] Vec<T>, [U; N] }
__std_vec_eq! { ['b, const N: usize,] Vec<T>, &'b [U; N] }

impl<T: Eq> Eq for Vec<T> {}

impl<T: PartialOrd> PartialOrd for Vec<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        PartialOrd::partial_cmp(self.as_slice(), other.as_slice())
    }
}

impl<T: Ord> Ord for Vec<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(self.as_slice(), other.as_slice())
    }
}

impl<T> Default for Vec<T> {
    fn default() -> Vec<T> {
        Vec::new()
    }
}

impl<T: fmt::Debug> fmt::Debug for Vec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_slice(), f)
    }
}

impl<T> AsRef<Vec<T>> for Vec<T> {
    fn as_ref(&self) -> &Vec<T> {
        self
    }
}

impl<T> AsMut<Vec<T>> for Vec<T> {
    fn as_mut(&mut self) -> &mut Vec<T> {
        self
    }
}

impl<T> AsRef<[T]> for Vec<T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> AsMut<[T]> for Vec<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

// ---------------------------------------------------------------------------
// Conversions

impl<T: Clone> From<&[T]> for Vec<T> {
    fn from(s: &[T]) -> Vec<T> {
        s.to_vec()
    }
}

impl<T: Clone> From<&mut [T]> for Vec<T> {
    fn from(s: &mut [T]) -> Vec<T> {
        s.to_vec()
    }
}

impl<T: Clone, const N: usize> From<&[T; N]> for Vec<T> {
    fn from(s: &[T; N]) -> Vec<T> {
        s.as_slice().to_vec()
    }
}

impl<T: Clone, const N: usize> From<&mut [T; N]> for Vec<T> {
    fn from(s: &mut [T; N]) -> Vec<T> {
        s.as_slice().to_vec()
    }
}

impl<T, const N: usize> From<[T; N]> for Vec<T> {
    fn from(s: [T; N]) -> Vec<T> {
        let mut v = Vec::with_capacity(N);
        let src = mem::ManuallyDrop::new(s);
        unsafe {
            intrinsics::copy_nonoverlapping(&*src as *const [T; N] as *const T, v.ptr, N);
            v.len = N;
        }
        v
    }
}

impl<T> From<Box<[T]>> for Vec<T> {
    fn from(s: Box<[T]>) -> Vec<T> {
        let len = s.len();
        let p = Box::into_raw(s);
        unsafe { Vec::from_raw_parts(intrinsics::slice_ptr(p as *const [T]) as *mut T, len, len) }
    }
}

impl From<&str> for Vec<u8> {
    fn from(s: &str) -> Vec<u8> {
        From::from(s.as_bytes())
    }
}

impl<'a, T: Clone> From<crate::borrow::Cow<'a, [T]>> for Vec<T> {
    fn from(s: crate::borrow::Cow<'a, [T]>) -> Vec<T> {
        s.into_owned()
    }
}

impl<T, const N: usize> TryFrom<Vec<T>> for [T; N] {
    type Error = Vec<T>;
    fn try_from(vec: Vec<T>) -> Result<[T; N], Vec<T>> {
        if vec.len() != N {
            return Err(vec);
        }
        let mut vec = vec;
        unsafe {
            vec.set_len(0);
            let p = vec.as_ptr() as *const [T; N];
            Ok(intrinsics::read(p))
        }
    }
}
