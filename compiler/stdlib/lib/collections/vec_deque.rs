//! A double-ended queue implemented with a growable ring buffer.

use crate::alloc::{alloc_array, capacity_overflow, dealloc_array};
use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::intrinsics;
use crate::iter::{DoubleEndedIterator, ExactSizeIterator, Extend, FromIterator, FusedIterator, IntoIterator, Iterator};
use crate::marker::PhantomData;
use crate::mem;
use crate::ops::{Index, IndexMut, RangeBounds};
use crate::slice;

/// A double-ended queue implemented with a growable ring buffer.
pub struct VecDeque<T> {
    buf: *mut T,
    cap: usize,
    head: usize,
    len: usize,
    _marker: PhantomData<T>,
}

#[inline(never)]
#[track_caller]
fn index_oob(index: usize, len: usize) -> ! {
    crate::rt::panic_fmt(format_args!("index out of bounds: the len is {len} but the index is {index}"), file!(), line!(), column!())
}

impl<T> VecDeque<T> {
    /// Creates an empty deque.
    #[inline]
    pub const fn new() -> VecDeque<T> {
        VecDeque {
            buf: mem::align_of::<T>() as *mut T,
            cap: if mem::size_of::<T>() == 0 { usize::MAX } else { 0 },
            head: 0,
            len: 0,
            _marker: PhantomData,
        }
    }

    /// Creates an empty deque with space for at least `capacity` elements.
    pub fn with_capacity(capacity: usize) -> VecDeque<T> {
        if mem::size_of::<T>() == 0 || capacity == 0 {
            return VecDeque::new();
        }
        VecDeque { buf: unsafe { alloc_array::<T>(capacity) }, cap: capacity, head: 0, len: 0, _marker: PhantomData }
    }

    #[inline]
    fn to_physical_idx(&self, idx: usize) -> usize {
        let i = self.head + idx;
        if i >= self.cap {
            i - self.cap
        } else {
            i
        }
    }

    #[inline]
    fn ptr_at(&self, physical: usize) -> *mut T {
        unsafe { intrinsics::ptr_add_mut(self.buf, physical) }
    }

    fn grow_to(&mut self, new_cap: usize) {
        if mem::size_of::<T>() == 0 {
            if new_cap > self.cap {
                capacity_overflow();
            }
            return;
        }
        unsafe {
            let new_buf = alloc_array::<T>(new_cap);
            // Copy the elements in logical order to the front of the new buffer.
            let (a, b) = self.as_slices();
            let alen = a.len();
            let blen = b.len();
            intrinsics::copy_nonoverlapping(a.as_ptr(), new_buf, alen);
            intrinsics::copy_nonoverlapping(b.as_ptr(), intrinsics::ptr_add_mut(new_buf, alen), blen);
            dealloc_array(self.buf, self.cap);
            self.buf = new_buf;
            self.cap = new_cap;
            self.head = 0;
        }
    }

    fn grow(&mut self) {
        let new_cap = if self.cap == 0 { 4 } else { self.cap.checked_mul(2).unwrap_or_else(|| capacity_overflow()) };
        self.grow_to(new_cap);
    }

    /// Provides a reference to the element at the given index.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len {
            unsafe { Some(&*self.ptr_at(self.to_physical_idx(index))) }
        } else {
            None
        }
    }

    /// Provides a mutable reference to the element at the given index.
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < self.len {
            unsafe { Some(&mut *self.ptr_at(self.to_physical_idx(index))) }
        } else {
            None
        }
    }

    /// Swaps elements at indices `i` and `j`.
    #[track_caller]
    pub fn swap(&mut self, i: usize, j: usize) {
        if i >= self.len {
            crate::rt::panic_str("assertion failed: i < self.len()", file!(), line!(), column!());
        }
        if j >= self.len {
            crate::rt::panic_str("assertion failed: j < self.len()", file!(), line!(), column!());
        }
        if i != j {
            unsafe { crate::ptr::swap(self.ptr_at(self.to_physical_idx(i)), self.ptr_at(self.to_physical_idx(j))) }
        }
    }

    /// Returns the number of elements the deque can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.cap
    }

    /// Reserves capacity for at least `additional` more elements.
    pub fn reserve(&mut self, additional: usize) {
        let needed = self.len.checked_add(additional).unwrap_or_else(|| capacity_overflow());
        if needed > self.cap {
            let mut new_cap = if self.cap == 0 { 4 } else { self.cap * 2 };
            if new_cap < needed {
                new_cap = needed;
            }
            self.grow_to(new_cap);
        }
    }

    /// Reserves the minimum capacity for at least `additional` more elements.
    pub fn reserve_exact(&mut self, additional: usize) {
        let needed = self.len.checked_add(additional).unwrap_or_else(|| capacity_overflow());
        if needed > self.cap {
            self.grow_to(needed);
        }
    }

    /// Shrinks the capacity of the deque as much as possible.
    pub fn shrink_to_fit(&mut self) {
        if mem::size_of::<T>() != 0 && self.cap > self.len {
            if self.len == 0 {
                unsafe { dealloc_array(self.buf, self.cap) };
                self.buf = mem::align_of::<T>() as *mut T;
                self.cap = 0;
                self.head = 0;
            } else {
                let len = self.len;
                self.grow_to(len);
            }
        }
    }

    /// Shortens the deque, keeping the first `len` elements and dropping the rest.
    pub fn truncate(&mut self, len: usize) {
        while self.len > len {
            let _ = self.pop_back();
        }
    }

    /// Returns a front-to-back iterator.
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        let (a, b) = self.as_slices();
        Iter { a: a.iter(), b: b.iter() }
    }

    /// Returns a front-to-back iterator that returns mutable references.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        let (a, b) = self.as_mut_slices();
        IterMut { a: a.iter_mut(), b: b.iter_mut() }
    }

    /// Returns a pair of slices which contain, in order, the contents of the deque.
    #[inline]
    pub fn as_slices(&self) -> (&[T], &[T]) {
        unsafe {
            let first_len = if self.head + self.len <= self.cap || mem::size_of::<T>() == 0 {
                self.len
            } else {
                self.cap - self.head
            };
            let a = slice::from_raw_parts(self.ptr_at(self.head) as *const T, first_len);
            let b = slice::from_raw_parts(self.buf as *const T, self.len - first_len);
            (a, b)
        }
    }

    /// Returns a pair of mutable slices which contain, in order, the contents of the deque.
    #[inline]
    pub fn as_mut_slices(&mut self) -> (&mut [T], &mut [T]) {
        unsafe {
            let first_len = if self.head + self.len <= self.cap || mem::size_of::<T>() == 0 {
                self.len
            } else {
                self.cap - self.head
            };
            let a = slice::from_raw_parts_mut(self.ptr_at(self.head), first_len);
            let b = slice::from_raw_parts_mut(self.buf, self.len - first_len);
            (a, b)
        }
    }

    /// Returns the number of elements in the deque.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the deque is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Creates an iterator that covers the specified range in the deque.
    #[track_caller]
    pub fn range<R: RangeBounds<usize>>(&self, range: R) -> Iter<'_, T> {
        let r = slice::range(range, self.len);
        let mut it = self.iter();
        let mut i = 0;
        while i < r.start {
            it.next();
            i += 1;
        }
        let mut j = self.len;
        while j > r.end {
            it.next_back();
            j -= 1;
        }
        it
    }

    /// Creates an iterator that covers the specified mutable range in the deque.
    #[track_caller]
    pub fn range_mut<R: RangeBounds<usize>>(&mut self, range: R) -> IterMut<'_, T> {
        let r = slice::range(range, self.len);
        let len = self.len;
        let mut it = self.iter_mut();
        let mut i = 0;
        while i < r.start {
            it.next();
            i += 1;
        }
        let mut j = len;
        while j > r.end {
            it.next_back();
            j -= 1;
        }
        it
    }

    /// Shrinks the capacity of the deque with a lower bound.
    pub fn shrink_to(&mut self, min_capacity: usize) {
        if self.cap > min_capacity && min_capacity >= self.len {
            if mem::size_of::<T>() != 0 && min_capacity > 0 {
                self.grow_to(min_capacity);
            } else if min_capacity == 0 {
                self.shrink_to_fit();
            }
        }
    }

    /// Tries to reserve capacity for at least `additional` more elements.
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), crate::collections::TryReserveError> {
        self.reserve(additional);
        Ok(())
    }

    /// Tries to reserve the minimum capacity for at least `additional` more elements.
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), crate::collections::TryReserveError> {
        self.reserve_exact(additional);
        Ok(())
    }

    /// Removes and returns the first element if the predicate returns `true`.
    pub fn pop_front_if(&mut self, predicate: impl FnOnce(&mut T) -> bool) -> Option<T> {
        let first = self.front_mut()?;
        if predicate(first) { self.pop_front() } else { None }
    }

    /// Removes and returns the last element if the predicate returns `true`.
    pub fn pop_back_if(&mut self, predicate: impl FnOnce(&mut T) -> bool) -> Option<T> {
        let last = self.back_mut()?;
        if predicate(last) { self.pop_back() } else { None }
    }

    /// Removes the specified range from the deque in bulk, returning all removed elements as an iterator.
    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Drain<'_, T> {
        let r = slice::range(range, self.len);
        let mut removed = Vec::with_capacity(r.end - r.start);
        let mut kept_tail = Vec::with_capacity(self.len - r.end);
        // Pop everything after the range, then the range itself.
        while self.len > r.end {
            kept_tail.push(self.pop_back().unwrap());
        }
        while self.len > r.start {
            removed.push(self.pop_back().unwrap());
        }
        removed.reverse();
        while let Some(x) = kept_tail.pop() {
            self.push_back(x);
        }
        Drain { inner: removed.into_iter(), _marker: PhantomData }
    }

    /// Clears the deque, removing all values.
    #[inline]
    pub fn clear(&mut self) {
        self.truncate(0);
        self.head = 0;
    }

    /// Returns `true` if the deque contains an element equal to the given value.
    pub fn contains(&self, x: &T) -> bool
    where
        T: PartialEq,
    {
        let (a, b) = self.as_slices();
        a.contains(x) || b.contains(x)
    }

    /// Provides a reference to the front element.
    #[inline]
    pub fn front(&self) -> Option<&T> {
        self.get(0)
    }

    /// Provides a mutable reference to the front element.
    #[inline]
    pub fn front_mut(&mut self) -> Option<&mut T> {
        self.get_mut(0)
    }

    /// Provides a reference to the back element.
    #[inline]
    pub fn back(&self) -> Option<&T> {
        if self.len == 0 {
            None
        } else {
            self.get(self.len - 1)
        }
    }

    /// Provides a mutable reference to the back element.
    #[inline]
    pub fn back_mut(&mut self) -> Option<&mut T> {
        if self.len == 0 {
            None
        } else {
            let i = self.len - 1;
            self.get_mut(i)
        }
    }

    /// Removes the first element and returns it, or `None` if the deque is empty.
    #[inline]
    pub fn pop_front(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            let old_head = self.head;
            self.head = self.to_physical_idx(1);
            self.len -= 1;
            unsafe { Some(intrinsics::read(self.ptr_at(old_head) as *const T)) }
        }
    }

    /// Removes the last element and returns it, or `None` if the deque is empty.
    #[inline]
    pub fn pop_back(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(intrinsics::read(self.ptr_at(self.to_physical_idx(self.len)) as *const T)) }
        }
    }

    /// Prepends an element to the deque.
    #[inline]
    pub fn push_front(&mut self, value: T) {
        if self.len == self.cap {
            self.grow();
        }
        self.head = if self.head == 0 { self.cap - 1 } else { self.head - 1 };
        self.len += 1;
        unsafe { intrinsics::write(self.ptr_at(self.head), value) }
    }

    /// Appends an element to the back of the deque.
    #[inline]
    pub fn push_back(&mut self, value: T) {
        if self.len == self.cap {
            self.grow();
        }
        unsafe { intrinsics::write(self.ptr_at(self.to_physical_idx(self.len)), value) }
        self.len += 1;
    }

    /// Appends an element to the back of the deque, returning a reference to it.
    pub fn push_back_mut(&mut self, value: T) -> &mut T {
        self.push_back(value);
        self.back_mut().unwrap()
    }

    /// Prepends an element to the deque, returning a reference to it.
    pub fn push_front_mut(&mut self, value: T) -> &mut T {
        self.push_front(value);
        self.front_mut().unwrap()
    }

    /// Inserts an element at `index` within the deque, returning a reference to it.
    pub fn insert_mut(&mut self, index: usize, value: T) -> &mut T {
        self.insert(index, value);
        self.get_mut(index).unwrap()
    }

    /// Removes an element from anywhere in the deque and returns it, replacing it with the first element.
    pub fn swap_remove_front(&mut self, index: usize) -> Option<T> {
        let length = self.len;
        if index < length && index != 0 {
            self.swap(index, 0);
        } else if index >= length {
            return None;
        }
        self.pop_front()
    }

    /// Removes an element from anywhere in the deque and returns it, replacing it with the last element.
    pub fn swap_remove_back(&mut self, index: usize) -> Option<T> {
        let length = self.len;
        if length > 0 && index < length - 1 {
            self.swap(index, length - 1);
        } else if index >= length {
            return None;
        }
        self.pop_back()
    }

    /// Inserts an element at `index` within the deque, shifting all elements with indices greater than or equal to `index` towards the back.
    #[track_caller]
    pub fn insert(&mut self, index: usize, value: T) {
        if index > self.len {
            crate::rt::panic_str("index out of bounds", file!(), line!(), column!());
        }
        self.push_back(value);
        let mut i = self.len - 1;
        while i > index {
            self.swap(i, i - 1);
            i -= 1;
        }
    }

    /// Removes and returns the element at `index` from the deque.
    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.len {
            return None;
        }
        let mut i = index;
        while i + 1 < self.len {
            self.swap(i, i + 1);
            i += 1;
        }
        self.pop_back()
    }

    /// Splits the deque into two at the given index.
    #[track_caller]
    pub fn split_off(&mut self, at: usize) -> VecDeque<T> {
        if at > self.len {
            crate::rt::panic_str("`at` out of bounds", file!(), line!(), column!());
        }
        let mut other = VecDeque::with_capacity(self.len - at);
        let mut tail = Vec::with_capacity(self.len - at);
        while self.len > at {
            tail.push(self.pop_back().unwrap());
        }
        while let Some(x) = tail.pop() {
            other.push_back(x);
        }
        other
    }

    /// Moves all the elements of `other` into `self`, leaving `other` empty.
    pub fn append(&mut self, other: &mut VecDeque<T>) {
        self.reserve(other.len);
        while let Some(x) = other.pop_front() {
            self.push_back(x);
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

    /// Retains only the elements specified by the predicate (with mutable access).
    pub fn retain_mut<F>(&mut self, f: F)
    where
        F: FnMut(&mut T) -> bool,
    {
        let mut f = f;
        let n = self.len;
        let mut i = 0;
        while i < n {
            let mut x = self.pop_front().unwrap();
            if f(&mut x) {
                self.push_back(x);
            }
            i += 1;
        }
    }

    /// Modifies the deque in-place so that `len()` is equal to `new_len`, filling with the result of calling `generator`.
    pub fn resize_with<F: FnMut() -> T>(&mut self, new_len: usize, generator: F) {
        let mut generator = generator;
        if new_len > self.len {
            while self.len < new_len {
                self.push_back(generator());
            }
        } else {
            self.truncate(new_len);
        }
    }

    /// Rearranges the internal storage of this deque so it is one contiguous slice, which is then returned.
    pub fn make_contiguous(&mut self) -> &mut [T] {
        if self.head + self.len > self.cap && mem::size_of::<T>() == 0 {
            // unreachable for ZSTs
        }
        if self.head + self.len > self.cap {
            let cap = self.cap;
            self.grow_to(cap);
        }
        unsafe { slice::from_raw_parts_mut(self.ptr_at(self.head), self.len) }
    }

    /// Rotates the double-ended queue `n` places to the left.
    #[track_caller]
    pub fn rotate_left(&mut self, n: usize) {
        if n > self.len {
            crate::rt::panic_str("assertion failed: n <= self.len()", file!(), line!(), column!());
        }
        let mut i = 0;
        while i < n {
            let x = self.pop_front().unwrap();
            self.push_back(x);
            i += 1;
        }
    }

    /// Rotates the double-ended queue `n` places to the right.
    #[track_caller]
    pub fn rotate_right(&mut self, n: usize) {
        if n > self.len {
            crate::rt::panic_str("assertion failed: n <= self.len()", file!(), line!(), column!());
        }
        let mut i = 0;
        while i < n {
            let x = self.pop_back().unwrap();
            self.push_front(x);
            i += 1;
        }
    }

    /// Binary searches this deque for a given element.
    pub fn binary_search(&self, x: &T) -> Result<usize, usize>
    where
        T: Ord,
    {
        self.binary_search_by(|e| e.cmp(x))
    }

    /// Binary searches this deque with a comparator function.
    pub fn binary_search_by<'a, F>(&'a self, f: F) -> Result<usize, usize>
    where
        F: FnMut(&'a T) -> Ordering,
    {
        let mut f = f;
        let (front, back) = self.as_slices();
        let cmp_back = match back.first() {
            Some(elem) => f(elem),
            None => Ordering::Greater,
        };
        if let Ordering::Equal = cmp_back {
            Ok(front.len())
        } else if let Ordering::Less = cmp_back {
            match back.binary_search_by(f) {
                Ok(idx) => Ok(idx + front.len()),
                Err(idx) => Err(idx + front.len()),
            }
        } else {
            front.binary_search_by(f)
        }
    }

    /// Binary searches this deque with a key extraction function.
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
        let (front, back) = self.as_slices();
        if let Some(true) = back.first().map(|v| pred(v)) {
            back.partition_point(pred) + front.len()
        } else {
            front.partition_point(pred)
        }
    }
}

impl<T: Clone> VecDeque<T> {
    /// Modifies the deque in-place so that `len()` is equal to `new_len`, filling with clones of `value`.
    pub fn resize(&mut self, new_len: usize, value: T) {
        if new_len > self.len {
            while self.len < new_len {
                self.push_back(value.clone());
            }
        } else {
            self.truncate(new_len);
        }
    }
}

impl<T> Drop for VecDeque<T> {
    fn drop(&mut self) {
        unsafe {
            let (a, b) = self.as_mut_slices();
            let pa = a as *mut [T];
            let pb = b as *mut [T];
            intrinsics::drop_in_place(pa);
            intrinsics::drop_in_place(pb);
            if mem::size_of::<T>() != 0 {
                dealloc_array(self.buf, self.cap);
            }
        }
    }
}

impl<T: Clone> Clone for VecDeque<T> {
    fn clone(&self) -> Self {
        let mut deq = VecDeque::with_capacity(self.len);
        let mut it = self.iter();
        while let Some(x) = it.next() {
            deq.push_back(x.clone());
        }
        deq
    }
}

impl<T> Default for VecDeque<T> {
    #[inline]
    fn default() -> VecDeque<T> {
        VecDeque::new()
    }
}

impl<T: PartialEq> PartialEq for VecDeque<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.len != other.len {
            return false;
        }
        let mut a = self.iter();
        let mut b = other.iter();
        while let (Some(x), Some(y)) = (a.next(), b.next()) {
            if *x != *y {
                return false;
            }
        }
        true
    }
}

impl<T: Eq> Eq for VecDeque<T> {}

impl<T: PartialEq> PartialEq<Vec<T>> for VecDeque<T> {
    fn eq(&self, other: &Vec<T>) -> bool {
        if self.len != other.len() {
            return false;
        }
        let mut i = 0;
        let mut a = self.iter();
        while let Some(x) = a.next() {
            if *x != other[i] {
                return false;
            }
            i += 1;
        }
        true
    }
}

impl<T: PartialEq> PartialEq<&[T]> for VecDeque<T> {
    fn eq(&self, other: &&[T]) -> bool {
        if self.len != other.len() {
            return false;
        }
        let mut i = 0;
        let mut a = self.iter();
        while let Some(x) = a.next() {
            if *x != other[i] {
                return false;
            }
            i += 1;
        }
        true
    }
}

impl<T: PartialEq, const N: usize> PartialEq<[T; N]> for VecDeque<T> {
    fn eq(&self, other: &[T; N]) -> bool {
        if self.len != N {
            return false;
        }
        let mut i = 0;
        let mut a = self.iter();
        while let Some(x) = a.next() {
            if *x != other[i] {
                return false;
            }
            i += 1;
        }
        true
    }
}

impl<T: PartialOrd> PartialOrd for VecDeque<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}

impl<T: Ord> Ord for VecDeque<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.iter().cmp(other.iter())
    }
}

impl<T: Hash> Hash for VecDeque<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_length_prefix(self.len);
        let mut it = self.iter();
        while let Some(x) = it.next() {
            x.hash(state);
        }
    }
}

impl<T> Index<usize> for VecDeque<T> {
    type Output = T;
    #[inline]
    #[track_caller]
    fn index(&self, index: usize) -> &T {
        match self.get(index) {
            Some(v) => v,
            None => crate::rt::panic_str("out of bounds access", file!(), line!(), column!()),
        }
    }
}

impl<T> IndexMut<usize> for VecDeque<T> {
    #[inline]
    #[track_caller]
    fn index_mut(&mut self, index: usize) -> &mut T {
        match self.get_mut(index) {
            Some(v) => v,
            None => crate::rt::panic_str("out of bounds access", file!(), line!(), column!()),
        }
    }
}

impl<T> FromIterator<T> for VecDeque<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> VecDeque<T> {
        let mut deq = VecDeque::new();
        deq.extend(iter);
        deq
    }
}

impl<T> IntoIterator for VecDeque<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> {
        IntoIter { inner: self }
    }
}

impl<'a, T> IntoIterator for &'a VecDeque<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut VecDeque<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;
    fn into_iter(self) -> IterMut<'a, T> {
        self.iter_mut()
    }
}

impl<T> Extend<T> for VecDeque<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let mut it = iter.into_iter();
        let (lower, _) = it.size_hint();
        self.reserve(lower);
        while let Some(x) = it.next() {
            self.push_back(x);
        }
    }
    #[inline]
    fn extend_one(&mut self, elem: T) {
        self.push_back(elem);
    }
}

impl<'a, T: 'a + Copy> Extend<&'a T> for VecDeque<T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        let mut it = iter.into_iter();
        while let Some(x) = it.next() {
            self.push_back(*x);
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for VecDeque<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T> From<Vec<T>> for VecDeque<T> {
    fn from(other: Vec<T>) -> Self {
        let len = other.len();
        let cap = other.capacity();
        let mut other = mem::ManuallyDrop::new(other);
        let buf = other.as_mut_ptr();
        if mem::size_of::<T>() == 0 {
            return VecDeque { buf: mem::align_of::<T>() as *mut T, cap: usize::MAX, head: 0, len, _marker: PhantomData };
        }
        VecDeque { buf, cap, head: 0, len, _marker: PhantomData }
    }
}

impl<T> From<VecDeque<T>> for Vec<T> {
    fn from(other: VecDeque<T>) -> Self {
        let mut other = other;
        let mut v = Vec::with_capacity(other.len);
        while let Some(x) = other.pop_front() {
            v.push(x);
        }
        v
    }
}

impl<T, const N: usize> From<[T; N]> for VecDeque<T> {
    fn from(arr: [T; N]) -> Self {
        VecDeque::from(Vec::from(arr))
    }
}

// ---------------------------------------------------------------------------
// Iterators

/// An iterator over the elements of a `VecDeque`.
pub struct Iter<'a, T: 'a> {
    a: slice::Iter<'a, T>,
    b: slice::Iter<'a, T>,
}

impl<'a, T> Clone for Iter<'a, T> {
    fn clone(&self) -> Self {
        Iter { a: self.a.clone(), b: self.b.clone() }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    #[inline]
    fn next(&mut self) -> Option<&'a T> {
        match self.a.next() {
            Some(x) => Some(x),
            None => {
                mem::swap(&mut self.a, &mut self.b);
                self.a.next()
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.a.len() + self.b.len();
        (len, Some(len))
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
        match self.b.next_back() {
            Some(x) => Some(x),
            None => {
                mem::swap(&mut self.a, &mut self.b);
                self.b.next_back()
            }
        }
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.a.len() + self.b.len()
    }
}

impl<'a, T> FusedIterator for Iter<'a, T> {}

impl<'a, T: fmt::Debug> fmt::Debug for Iter<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Iter").field(&self.a.as_slice()).field(&self.b.as_slice()).finish()
    }
}

/// A mutable iterator over the elements of a `VecDeque`.
pub struct IterMut<'a, T: 'a> {
    a: slice::IterMut<'a, T>,
    b: slice::IterMut<'a, T>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    #[inline]
    fn next(&mut self) -> Option<&'a mut T> {
        match self.a.next() {
            Some(x) => Some(x),
            None => {
                mem::swap(&mut self.a, &mut self.b);
                self.a.next()
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.a.len() + self.b.len();
        (len, Some(len))
    }
}

impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a mut T> {
        match self.b.next_back() {
            Some(x) => Some(x),
            None => {
                mem::swap(&mut self.a, &mut self.b);
                self.b.next_back()
            }
        }
    }
}

impl<'a, T> ExactSizeIterator for IterMut<'a, T> {
    fn len(&self) -> usize {
        self.a.len() + self.b.len()
    }
}

impl<'a, T> FusedIterator for IterMut<'a, T> {}

impl<'a, T> IterMut<'a, T> {
    /// The remaining elements, front part first.
    pub(crate) fn remaining(&self) -> (&[T], &[T]) {
        (self.a.as_slice(), self.b.as_slice())
    }
}

impl<'a, T: fmt::Debug> fmt::Debug for IterMut<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IterMut").field(&self.a.as_slice()).field(&self.b.as_slice()).finish()
    }
}

/// An owning iterator over the elements of a `VecDeque`.
pub struct IntoIter<T> {
    inner: VecDeque<T>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        self.inner.pop_front()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.inner.len();
        (len, Some(len))
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        self.inner.pop_back()
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<T> FusedIterator for IntoIter<T> {}

impl<T: Clone> Clone for IntoIter<T> {
    fn clone(&self) -> Self {
        IntoIter { inner: self.inner.clone() }
    }
}

impl<T> IntoIter<T> {
    /// The remaining elements.
    pub(crate) fn as_deque(&self) -> &VecDeque<T> {
        &self.inner
    }
}

impl<T: fmt::Debug> fmt::Debug for IntoIter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoIter").field(&self.inner).finish()
    }
}

/// A draining iterator over the elements of a `VecDeque`.
pub struct Drain<'a, T: 'a> {
    inner: crate::vec::IntoIter<T>,
    _marker: PhantomData<&'a mut VecDeque<T>>,
}

// Note: std prints its internal counters here; this prints what is left.
impl<'a, T: fmt::Debug> fmt::Debug for Drain<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain").field(&self.inner.as_slice()).finish()
    }
}

impl<'a, T> Iterator for Drain<'a, T> {
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        self.inner.next()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, T> DoubleEndedIterator for Drain<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        self.inner.next_back()
    }
}

impl<'a, T> ExactSizeIterator for Drain<'a, T> {}

impl<'a, T> Default for Iter<'a, T> {
    fn default() -> Self {
        Iter { a: Default::default(), b: Default::default() }
    }
}

impl<'a, T> Default for IterMut<'a, T> {
    fn default() -> Self {
        IterMut { a: Default::default(), b: Default::default() }
    }
}
