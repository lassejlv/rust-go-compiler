//! Single-threaded reference-counting pointers.

use crate::borrow::Borrow;
use crate::cell::Cell;
use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::intrinsics;
use crate::iter::{FromIterator, IntoIterator, Iterator};
use crate::mem;
use crate::ops::Deref;

// The heap part of an `Rc`: the counts followed by the (possibly unsized)
// value. `weak` counts the weak pointers plus one for all strong pointers.
struct RcBox<T: ?Sized> {
    strong: Cell<usize>,
    weak: Cell<usize>,
    value: T,
}

/// A single-threaded reference-counting pointer.
#[rustc_coerce_unsized]
pub struct Rc<T: ?Sized> {
    ptr: *mut RcBox<T>,
}

/// A version of `Rc` that holds a non-owning reference to the managed allocation (null for `Weak::new`).
#[rustc_coerce_unsized]
pub struct Weak<T: ?Sized> {
    ptr: *mut RcBox<T>,
}

#[inline]
fn round_up(n: usize, align: usize) -> usize {
    (n + align - 1) & !(align - 1)
}

/// Allocates an `RcBox<[T]>` for `len` elements with counts set to 1; the
/// elements are uninitialized. Returns the box pointer and the element pointer.
unsafe fn allocate_for_slice<T>(len: usize) -> (*mut RcBox<[T]>, *mut T) {
    let word = mem::size_of::<usize>();
    let elem_align = mem::align_of::<T>();
    let align = if elem_align > word { elem_align } else { word };
    let header = round_up(2 * word, elem_align);
    let size = round_up(header + len * mem::size_of::<T>(), align);
    let mem_ptr = intrinsics::alloc(size, align);
    intrinsics::write(mem_ptr as *mut usize, 1);
    intrinsics::write(intrinsics::ptr_add_mut(mem_ptr as *mut usize, 1), 1);
    let elems = intrinsics::ptr_add_mut(mem_ptr, header) as *mut T;
    let fat = intrinsics::slice_from_raw_parts_mut(mem_ptr as *mut T, len) as *mut RcBox<[T]>;
    (fat, elems)
}

impl<T> Rc<T> {
    /// Constructs a new `Rc<T>`.
    pub fn new(value: T) -> Rc<T> {
        let b = Box::new(RcBox { strong: Cell::new(1), weak: Cell::new(1), value });
        Rc { ptr: Box::into_raw(b) }
    }

    /// Constructs a new `Rc<T>` while giving you a `Weak<T>` to the allocation.
    pub fn new_cyclic<F>(data_fn: F) -> Rc<T>
    where
        F: FnOnce(&Weak<T>) -> T,
    {
        unsafe {
            let p = crate::alloc::alloc_array::<RcBox<T>>(1);
            intrinsics::write(&mut (*p).strong as *mut Cell<usize>, Cell::new(0));
            intrinsics::write(&mut (*p).weak as *mut Cell<usize>, Cell::new(1));
            let weak = Weak { ptr: p };
            let data = data_fn(&weak);
            intrinsics::write(&mut (*p).value as *mut T, data);
            (*p).strong.set(1);
            mem::forget(weak);
            Rc { ptr: p }
        }
    }

    /// Constructs a new `Pin<Rc<T>>`.
    pub fn pin(value: T) -> crate::pin::Pin<Rc<T>> {
        unsafe { crate::pin::Pin::new_unchecked(Rc::new(value)) }
    }

    /// Returns the inner value, if the `Rc` has exactly one strong reference.
    pub fn try_unwrap(this: Self) -> Result<T, Self> {
        if Rc::strong_count(&this) == 1 {
            unsafe {
                let p = this.ptr;
                let val = intrinsics::read(&(*p).value as *const T);
                (*p).strong.set(0);
                mem::forget(this);
                // Drop the implicit weak reference held by the strong ones.
                drop(Weak { ptr: p });
                Ok(val)
            }
        } else {
            Err(this)
        }
    }

    /// Returns the inner value, if the `Rc` has exactly one strong reference.
    pub fn into_inner(this: Self) -> Option<T> {
        Rc::try_unwrap(this).ok()
    }

    /// Consumes the `Rc`, returning the wrapped pointer.
    pub fn into_raw(this: Self) -> *const T {
        let p = Rc::as_ptr(&this);
        mem::forget(this);
        p
    }

    /// Constructs an `Rc<T>` from a raw pointer obtained from `Rc::into_raw`.
    pub unsafe fn from_raw(ptr: *const T) -> Self {
        let base = mem::align_of::<RcBox<T>>() as *const RcBox<T>;
        let offset = (&(*base).value as *const T as usize) - (base as usize);
        Rc { ptr: ((ptr as usize) - offset) as *mut RcBox<T> }
    }
}

impl<T: Clone> Rc<T> {
    /// If we have the only reference to `T` then unwrap it. Otherwise, clone `T` and return the clone.
    pub fn unwrap_or_clone(this: Self) -> T {
        match Rc::try_unwrap(this) {
            Ok(t) => t,
            Err(rc) => (*rc).clone(),
        }
    }

    /// Makes a mutable reference into the given `Rc`, cloning the inner value if needed.
    pub fn make_mut(this: &mut Self) -> &mut T {
        if Rc::strong_count(this) != 1 {
            *this = Rc::new((**this).clone());
        } else if Rc::weak_count(this) != 0 {
            unsafe {
                let old = this.ptr;
                let fresh = Rc::new(intrinsics::read(&(*old).value as *const T));
                (*old).strong.set(0);
                (*old).weak.set((*old).weak.get() - 1);
                intrinsics::write(this as *mut Rc<T>, fresh);
            }
        }
        unsafe { &mut (*this.ptr).value }
    }
}

impl<T: ?Sized> Rc<T> {
    #[inline]
    fn inner(&self) -> &RcBox<T> {
        unsafe { &*self.ptr }
    }

    /// Provides a raw pointer to the data.
    #[inline]
    pub fn as_ptr(this: &Self) -> *const T {
        unsafe { &(*this.ptr).value as *const T }
    }

    /// Creates a new `Weak` pointer to this allocation.
    pub fn downgrade(this: &Self) -> Weak<T> {
        let w = this.inner().weak.get();
        this.inner().weak.set(w + 1);
        Weak { ptr: this.ptr }
    }

    /// Gets the number of `Weak` pointers to this allocation.
    #[inline]
    pub fn weak_count(this: &Self) -> usize {
        this.inner().weak.get() - 1
    }

    /// Gets the number of strong (`Rc`) pointers to this allocation.
    #[inline]
    pub fn strong_count(this: &Self) -> usize {
        this.inner().strong.get()
    }

    /// Increments the strong reference count on the `Rc<T>` associated with the provided pointer by one.
    pub unsafe fn increment_strong_count(ptr: *const T)
    where
        T: Sized,
    {
        let rc = mem::ManuallyDrop::new(Rc::<T>::from_raw(ptr));
        let s = rc.inner().strong.get();
        rc.inner().strong.set(s + 1);
    }

    /// Returns a mutable reference into the given `Rc`, if there are no other `Rc` or `Weak` pointers to the same allocation.
    #[inline]
    pub fn get_mut(this: &mut Self) -> Option<&mut T> {
        if Rc::strong_count(this) == 1 && Rc::weak_count(this) == 0 {
            unsafe { Some(&mut (*this.ptr).value) }
        } else {
            None
        }
    }

    /// Returns `true` if the two `Rc`s point to the same allocation.
    #[inline]
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        (this.ptr as *const u8 as usize) == (other.ptr as *const u8 as usize)
    }
}

impl<T: ?Sized> Clone for Rc<T> {
    #[inline]
    fn clone(&self) -> Rc<T> {
        let s = self.inner().strong.get();
        self.inner().strong.set(s + 1);
        Rc { ptr: self.ptr }
    }
}

impl<T: ?Sized> Drop for Rc<T> {
    fn drop(&mut self) {
        unsafe {
            let p = self.ptr;
            let s = (*p).strong.get() - 1;
            (*p).strong.set(s);
            if s == 0 {
                intrinsics::drop_in_place(&mut (*p).value as *mut T);
                let w = (*p).weak.get() - 1;
                (*p).weak.set(w);
                if w == 0 {
                    let size = intrinsics::size_of_val(p as *const RcBox<T>);
                    let align = intrinsics::align_of_val(p as *const RcBox<T>);
                    intrinsics::dealloc(p as *mut u8, size, align);
                }
            }
        }
    }
}

impl<T: ?Sized> Deref for Rc<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        &self.inner().value
    }
}

impl<T: ?Sized> AsRef<T> for Rc<T> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized> Borrow<T> for Rc<T> {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T: Default> Default for Rc<T> {
    #[inline]
    fn default() -> Rc<T> {
        Rc::new(Default::default())
    }
}

impl<T: ?Sized + PartialEq> PartialEq for Rc<T> {
    #[inline]
    fn eq(&self, other: &Rc<T>) -> bool {
        **self == **other
    }
    #[inline]
    fn ne(&self, other: &Rc<T>) -> bool {
        **self != **other
    }
}

impl<T: ?Sized + Eq> Eq for Rc<T> {}

impl<T: ?Sized + PartialOrd> PartialOrd for Rc<T> {
    #[inline]
    fn partial_cmp(&self, other: &Rc<T>) -> Option<Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl<T: ?Sized + Ord> Ord for Rc<T> {
    #[inline]
    fn cmp(&self, other: &Rc<T>) -> Ordering {
        (**self).cmp(&**other)
    }
}

impl<T: ?Sized + Hash> Hash for Rc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for Rc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Rc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized> fmt::Pointer for Rc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p: *const T = Rc::as_ptr(self);
        fmt::Pointer::fmt(&p, f)
    }
}

impl<T> From<T> for Rc<T> {
    fn from(t: T) -> Self {
        Rc::new(t)
    }
}

impl From<crate::string::String> for Rc<str> {
    fn from(v: crate::string::String) -> Rc<str> {
        Rc::from(v.as_str())
    }
}

impl From<&str> for Rc<str> {
    fn from(v: &str) -> Rc<str> {
        unsafe {
            let len = v.len();
            let (p, elems) = allocate_for_slice::<u8>(len);
            intrinsics::copy_nonoverlapping(v.as_ptr(), elems, len);
            Rc { ptr: p as *mut RcBox<str> }
        }
    }
}

impl<T: Clone> From<&[T]> for Rc<[T]> {
    fn from(v: &[T]) -> Rc<[T]> {
        unsafe {
            let len = v.len();
            let (p, elems) = allocate_for_slice::<T>(len);
            let mut i = 0;
            while i < len {
                intrinsics::write(intrinsics::ptr_add_mut(elems, i), v[i].clone());
                i += 1;
            }
            Rc { ptr: p }
        }
    }
}

impl<T> From<Vec<T>> for Rc<[T]> {
    fn from(v: Vec<T>) -> Rc<[T]> {
        unsafe {
            let len = v.len();
            let (p, elems) = allocate_for_slice::<T>(len);
            let mut v = v;
            intrinsics::copy_nonoverlapping(v.as_ptr(), elems, len);
            v.set_len(0);
            Rc { ptr: p }
        }
    }
}

impl<T> From<Box<T>> for Rc<T> {
    fn from(b: Box<T>) -> Rc<T> {
        Rc::new(Box::into_inner(b))
    }
}

impl<T> FromIterator<T> for Rc<[T]> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let v: Vec<T> = iter.into_iter().collect();
        Rc::from(v)
    }
}

// ---------------------------------------------------------------------------
// Weak

impl<T> Weak<T> {
    /// Constructs a new `Weak<T>`, without allocating any memory.
    pub const fn new() -> Weak<T> {
        Weak { ptr: 0usize as *mut RcBox<T> }
    }
}

impl<T: ?Sized> Weak<T> {
    #[inline]
    fn is_dangling(&self) -> bool {
        (self.ptr as *const u8 as usize) == 0
    }

    /// Attempts to upgrade the `Weak` pointer to an `Rc`.
    pub fn upgrade(&self) -> Option<Rc<T>> {
        if self.is_dangling() {
            return None;
        }
        unsafe {
            let s = (*self.ptr).strong.get();
            if s == 0 {
                None
            } else {
                (*self.ptr).strong.set(s + 1);
                Some(Rc { ptr: self.ptr })
            }
        }
    }

    /// Gets the number of strong (`Rc`) pointers pointing to this allocation.
    pub fn strong_count(&self) -> usize {
        if self.is_dangling() {
            0
        } else {
            unsafe { (*self.ptr).strong.get() }
        }
    }

    /// Gets the number of `Weak` pointers pointing to this allocation.
    pub fn weak_count(&self) -> usize {
        if self.is_dangling() {
            return 0;
        }
        unsafe {
            let strong = (*self.ptr).strong.get();
            if strong > 0 {
                (*self.ptr).weak.get() - 1
            } else {
                0
            }
        }
    }

    /// Returns a raw pointer to the object `T` pointed to by this `Weak<T>`.
    pub fn as_ptr(&self) -> *const T {
        unsafe { &(*self.ptr).value as *const T }
    }

    /// Returns `true` if the two `Weak`s point to the same allocation.
    #[inline]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        (self.ptr as *const u8 as usize) == (other.ptr as *const u8 as usize)
    }
}

impl<T: ?Sized> Clone for Weak<T> {
    #[inline]
    fn clone(&self) -> Weak<T> {
        if !self.is_dangling() {
            unsafe {
                let w = (*self.ptr).weak.get();
                (*self.ptr).weak.set(w + 1);
            }
        }
        Weak { ptr: self.ptr }
    }
}

impl<T: ?Sized> Drop for Weak<T> {
    fn drop(&mut self) {
        if self.is_dangling() {
            return;
        }
        unsafe {
            let p = self.ptr;
            let w = (*p).weak.get() - 1;
            (*p).weak.set(w);
            if w == 0 {
                let size = intrinsics::size_of_val(p as *const RcBox<T>);
                let align = intrinsics::align_of_val(p as *const RcBox<T>);
                intrinsics::dealloc(p as *mut u8, size, align);
            }
        }
    }
}

impl<T> Default for Weak<T> {
    fn default() -> Weak<T> {
        Weak::new()
    }
}

impl<T: ?Sized> fmt::Debug for Weak<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("(Weak)")
    }
}


// ---------------------------------------------------------------------------
// Uninitialized values and downcasting

impl<T> Rc<T> {
    /// Constructs a new `Rc` with uninitialized contents.
    pub fn new_uninit() -> Rc<mem::MaybeUninit<T>> {
        Rc::new(mem::MaybeUninit::uninit())
    }

    /// Constructs a new `Rc` with uninitialized contents, with the memory being filled with `0` bytes.
    pub fn new_zeroed() -> Rc<mem::MaybeUninit<T>> {
        Rc::new(mem::MaybeUninit::zeroed())
    }

    /// Constructs a new reference-counted slice with uninitialized contents.
    pub fn new_uninit_slice(len: usize) -> Rc<[mem::MaybeUninit<T>]> {
        unsafe {
            let (p, _) = allocate_for_slice::<mem::MaybeUninit<T>>(len);
            Rc { ptr: p }
        }
    }

    /// Constructs a new reference-counted slice with uninitialized contents, with the memory being filled with `0` bytes.
    pub fn new_zeroed_slice(len: usize) -> Rc<[mem::MaybeUninit<T>]> {
        unsafe {
            let (p, elems) = allocate_for_slice::<mem::MaybeUninit<T>>(len);
            intrinsics::write_bytes(elems as *mut u8, 0, len * mem::size_of::<T>());
            Rc { ptr: p }
        }
    }
}

impl<T> Rc<mem::MaybeUninit<T>> {
    /// Converts to `Rc<T>`.
    pub unsafe fn assume_init(self) -> Rc<T> {
        let p = self.ptr as *mut RcBox<T>;
        mem::forget(self);
        Rc { ptr: p }
    }
}

impl<T> Rc<[mem::MaybeUninit<T>]> {
    /// Converts to `Rc<[T]>`.
    pub unsafe fn assume_init(self) -> Rc<[T]> {
        let p = self.ptr as *mut RcBox<[T]>;
        mem::forget(self);
        Rc { ptr: p }
    }
}

impl Rc<dyn crate::any::Any> {
    /// Attempts to downcast the `Rc<dyn crate::any::Any>` to a concrete type.
    pub fn downcast<T: crate::any::Any>(self) -> Result<Rc<T>, Self> {
        if (*self).is::<T>() {
            let p = self.ptr as *mut RcBox<T>;
            mem::forget(self);
            Ok(Rc { ptr: p })
        } else {
            Err(self)
        }
    }
}

impl<T> Rc<T> {
    /// Decrements the strong reference count on the `Rc<T>` associated with the provided pointer by one.
    pub unsafe fn decrement_strong_count(ptr: *const T) {
        drop(Rc::<T>::from_raw(ptr));
    }
}
