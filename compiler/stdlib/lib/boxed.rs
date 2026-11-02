//! The `Box<T>` type for heap allocation.

use crate::borrow::{Borrow, BorrowMut};
use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::intrinsics;
use crate::iter::{DoubleEndedIterator, ExactSizeIterator, FromIterator, FusedIterator, IntoIterator, Iterator};
use crate::ops::{Deref, DerefMut};

/// A pointer type that uniquely owns a heap allocation of type `T`.
///
/// The compiler treats `*b` as a builtin dereference of `ptr` and converts
/// `ptr` for unsizing coercions (`Box<T>` -> `Box<dyn Trait>` / `Box<[T]>`).
#[lang = "owned_box"]
pub struct Box<T: ?Sized> {
    pub(crate) ptr: *mut T,
}

impl<T> Box<T> {
    /// Allocates memory on the heap and then places `x` into it.
    #[inline]
    pub fn new(x: T) -> Box<T> {
        unsafe {
            let p = crate::alloc::alloc_array::<T>(1);
            intrinsics::write(p, x);
            Box { ptr: p }
        }
    }

    /// Consumes the box, returning the wrapped value.
    #[inline]
    pub fn into_inner(boxed: Self) -> T {
        unsafe {
            let p = boxed.ptr;
            intrinsics::forget(boxed);
            let v = intrinsics::read(p as *const T);
            crate::alloc::dealloc_array(p, 1);
            v
        }
    }

    /// Constructs a new `Pin<Box<T>>` (pinning is a no-op here).
    #[inline]
    pub fn pin(x: T) -> Box<T> {
        Box::new(x)
    }
}

impl<T: ?Sized> Box<T> {
    /// Constructs a box from a raw pointer.
    #[inline]
    pub unsafe fn from_raw(raw: *mut T) -> Box<T> {
        Box { ptr: raw }
    }

    /// Consumes the `Box`, returning a wrapped raw pointer.
    #[inline]
    pub fn into_raw(b: Box<T>) -> *mut T {
        let p = b.ptr;
        crate::mem::forget(b);
        p
    }

    /// Consumes and leaks the `Box`, returning a mutable reference.
    #[inline]
    pub fn leak<'a>(b: Box<T>) -> &'a mut T {
        unsafe { &mut *Box::into_raw(b) }
    }

    /// Returns a raw pointer to the box's contents.
    #[inline]
    pub fn as_ptr(b: &Box<T>) -> *const T {
        b.ptr as *const T
    }

    /// Returns a raw mutable pointer to the box's contents.
    #[inline]
    pub fn as_mut_ptr(b: &mut Box<T>) -> *mut T {
        b.ptr
    }
}

impl<T: ?Sized> Drop for Box<T> {
    fn drop(&mut self) {
        unsafe {
            let p = self.ptr;
            let size = intrinsics::size_of_val(p as *const T);
            let align = intrinsics::align_of_val(p as *const T);
            intrinsics::drop_in_place(p);
            if size != 0 {
                intrinsics::dealloc(p as *mut u8, size, align);
            }
        }
    }
}

impl<T: ?Sized> Deref for Box<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        unsafe { &*self.ptr }
    }
}

impl<T: ?Sized> DerefMut for Box<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }
}

impl<T: Default> Default for Box<T> {
    #[inline]
    fn default() -> Self {
        Box::new(T::default())
    }
}

impl<T> Default for Box<[T]> {
    #[inline]
    fn default() -> Self {
        Vec::<T>::new().into_boxed_slice()
    }
}

impl Default for Box<str> {
    #[inline]
    fn default() -> Self {
        String::new().into_boxed_str()
    }
}

impl<T: Clone> Clone for Box<T> {
    #[inline]
    fn clone(&self) -> Self {
        Box::new((**self).clone())
    }
}

impl Clone for Box<str> {
    fn clone(&self) -> Self {
        String::from(&**self).into_boxed_str()
    }
}

impl<T: Clone> Clone for Box<[T]> {
    fn clone(&self) -> Self {
        (**self).to_vec().into_boxed_slice()
    }
}

impl<T: ?Sized + PartialEq> PartialEq for Box<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
    #[inline]
    fn ne(&self, other: &Self) -> bool {
        PartialEq::ne(&**self, &**other)
    }
}

impl<T: ?Sized + Eq> Eq for Box<T> {}

impl<T: ?Sized + PartialOrd> PartialOrd for Box<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
    #[inline]
    fn lt(&self, other: &Self) -> bool {
        PartialOrd::lt(&**self, &**other)
    }
    #[inline]
    fn le(&self, other: &Self) -> bool {
        PartialOrd::le(&**self, &**other)
    }
    #[inline]
    fn ge(&self, other: &Self) -> bool {
        PartialOrd::ge(&**self, &**other)
    }
    #[inline]
    fn gt(&self, other: &Self) -> bool {
        PartialOrd::gt(&**self, &**other)
    }
}

impl<T: ?Sized + Ord> Ord for Box<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<T: ?Sized + Hash> Hash for Box<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: ?Sized + Hasher> Hasher for Box<T> {
    fn finish(&self) -> u64 {
        (**self).finish()
    }
    fn write(&mut self, bytes: &[u8]) {
        (**self).write(bytes)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for Box<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Box<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized> fmt::Pointer for Box<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr: *const T = self.ptr as *const T;
        fmt::Pointer::fmt(&ptr, f)
    }
}

impl<T> From<T> for Box<T> {
    /// Converts a `T` into a `Box<T>`.
    fn from(t: T) -> Self {
        Box::new(t)
    }
}

impl<T: Clone> From<&[T]> for Box<[T]> {
    fn from(slice: &[T]) -> Box<[T]> {
        slice.to_vec().into_boxed_slice()
    }
}

impl<T, const N: usize> From<[T; N]> for Box<[T]> {
    fn from(array: [T; N]) -> Box<[T]> {
        Vec::from(array).into_boxed_slice()
    }
}

impl<T> From<Vec<T>> for Box<[T]> {
    fn from(v: Vec<T>) -> Box<[T]> {
        v.into_boxed_slice()
    }
}

impl From<&str> for Box<str> {
    #[inline]
    fn from(s: &str) -> Box<str> {
        String::from(s).into_boxed_str()
    }
}

impl From<String> for Box<str> {
    #[inline]
    fn from(s: String) -> Box<str> {
        s.into_boxed_str()
    }
}

impl<T: ?Sized> AsRef<T> for Box<T> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized> AsMut<T> for Box<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T: ?Sized> Borrow<T> for Box<T> {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized> BorrowMut<T> for Box<T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<I: Iterator + ?Sized> Iterator for Box<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        (**self).next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (**self).size_hint()
    }
    fn nth(&mut self, n: usize) -> Option<I::Item> {
        (**self).nth(n)
    }
}

impl<I: DoubleEndedIterator + ?Sized> DoubleEndedIterator for Box<I> {
    fn next_back(&mut self) -> Option<I::Item> {
        (**self).next_back()
    }
}

impl<I: ExactSizeIterator + ?Sized> ExactSizeIterator for Box<I> {
    fn len(&self) -> usize {
        (**self).len()
    }
}

impl<I: FusedIterator + ?Sized> FusedIterator for Box<I> {}

impl<T> FromIterator<T> for Box<[T]> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let v: Vec<T> = FromIterator::from_iter(iter);
        v.into_boxed_slice()
    }
}

impl<T> IntoIterator for Box<[T]> {
    type Item = T;
    type IntoIter = crate::vec::IntoIter<T>;
    fn into_iter(self) -> crate::vec::IntoIter<T> {
        Vec::from(self).into_iter()
    }
}

impl<'a, T> IntoIterator for &'a Box<[T]> {
    type Item = &'a T;
    type IntoIter = crate::slice::Iter<'a, T>;
    fn into_iter(self) -> crate::slice::Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Box<[T]> {
    type Item = &'a mut T;
    type IntoIter = crate::slice::IterMut<'a, T>;
    fn into_iter(self) -> crate::slice::IterMut<'a, T> {
        self.iter_mut()
    }
}
