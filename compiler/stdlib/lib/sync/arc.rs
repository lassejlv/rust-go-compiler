//! Thread-safe reference-counting pointers (atomic reference counts).

use crate::borrow::Borrow;
use crate::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::intrinsics;
use crate::iter::{FromIterator, IntoIterator, Iterator};
use crate::mem;
use crate::ops::Deref;

// The heap part of an `Arc`: the counts followed by the (possibly unsized)
// value. `weak` counts the weak pointers plus one for all strong pointers.
struct ArcInner<T: ?Sized> {
    strong: AtomicUsize,
    weak: AtomicUsize,
    value: T,
}

/// A thread-safe reference-counting pointer.
#[rustc_coerce_unsized]
pub struct Arc<T: ?Sized> {
    ptr: *mut ArcInner<T>,
}

/// A version of `Arc` that holds a non-owning reference to the managed allocation (null for `Weak::new`).
#[rustc_coerce_unsized]
pub struct Weak<T: ?Sized> {
    ptr: *mut ArcInner<T>,
}

#[inline]
fn round_up(n: usize, align: usize) -> usize {
    (n + align - 1) & !(align - 1)
}

/// Allocates an `ArcInner<[T]>` for `len` elements with counts set to 1; the
/// elements are uninitialized. Returns the box pointer and the element pointer.
unsafe fn allocate_for_slice<T>(len: usize) -> (*mut ArcInner<[T]>, *mut T) {
    let word = mem::size_of::<usize>();
    let elem_align = mem::align_of::<T>();
    let align = if elem_align > word { elem_align } else { word };
    let header = round_up(2 * word, elem_align);
    let size = round_up(header + len * mem::size_of::<T>(), align);
    let mem_ptr = intrinsics::alloc(size, align);
    intrinsics::write(mem_ptr as *mut usize, 1);
    intrinsics::write(intrinsics::ptr_add_mut(mem_ptr as *mut usize, 1), 1);
    let elems = intrinsics::ptr_add_mut(mem_ptr, header) as *mut T;
    let fat = intrinsics::slice_from_raw_parts_mut(mem_ptr as *mut T, len) as *mut ArcInner<[T]>;
    (fat, elems)
}

impl<T> Arc<T> {
    /// Constructs a new `Arc<T>`.
    pub fn new(value: T) -> Arc<T> {
        let b = Box::new(ArcInner { strong: AtomicUsize::new(1), weak: AtomicUsize::new(1), value });
        Arc { ptr: Box::into_raw(b) }
    }

    /// Constructs a new `Arc<T>` while giving you a `Weak<T>` to the allocation.
    pub fn new_cyclic<F>(data_fn: F) -> Arc<T>
    where
        F: FnOnce(&Weak<T>) -> T,
    {
        unsafe {
            let p = crate::alloc::alloc_array::<ArcInner<T>>(1);
            intrinsics::write(&mut (*p).strong as *mut AtomicUsize, AtomicUsize::new(0));
            intrinsics::write(&mut (*p).weak as *mut AtomicUsize, AtomicUsize::new(1));
            let weak = Weak { ptr: p };
            let data = data_fn(&weak);
            intrinsics::write(&mut (*p).value as *mut T, data);
            (*p).strong.store(1, SeqCst);
            mem::forget(weak);
            Arc { ptr: p }
        }
    }

    /// Constructs a new `Pin<Arc<T>>`.
    pub fn pin(value: T) -> crate::pin::Pin<Arc<T>> {
        unsafe { crate::pin::Pin::new_unchecked(Arc::new(value)) }
    }

    /// Returns the inner value, if the `Arc` has exactly one strong reference.
    pub fn try_unwrap(this: Self) -> Result<T, Self> {
        if this.inner().strong.compare_exchange(1, 0, SeqCst, SeqCst).is_ok() {
            unsafe {
                let p = this.ptr;
                let val = intrinsics::read(&(*p).value as *const T);
                mem::forget(this);
                // Drop the implicit weak reference held by the strong ones.
                drop(Weak { ptr: p });
                Ok(val)
            }
        } else {
            Err(this)
        }
    }

    /// Returns the inner value, if the `Arc` has exactly one strong reference.
    pub fn into_inner(this: Self) -> Option<T> {
        Arc::try_unwrap(this).ok()
    }

    /// Consumes the `Arc`, returning the wrapped pointer.
    pub fn into_raw(this: Self) -> *const T {
        let p = Arc::as_ptr(&this);
        mem::forget(this);
        p
    }

    /// Constructs an `Arc<T>` from a raw pointer obtained from `Arc::into_raw`.
    pub unsafe fn from_raw(ptr: *const T) -> Self {
        let base = mem::align_of::<ArcInner<T>>() as *const ArcInner<T>;
        let offset = (&(*base).value as *const T as usize) - (base as usize);
        Arc { ptr: ((ptr as usize) - offset) as *mut ArcInner<T> }
    }
}

impl<T: Clone> Arc<T> {
    /// If we have the only reference to `T` then unwrap it. Otherwise, clone `T` and return the clone.
    pub fn unwrap_or_clone(this: Self) -> T {
        match Arc::try_unwrap(this) {
            Ok(t) => t,
            Err(rc) => (*rc).clone(),
        }
    }

    /// Makes a mutable reference into the given `Arc`, cloning the inner value if needed.
    pub fn make_mut(this: &mut Self) -> &mut T {
        if Arc::strong_count(this) != 1 {
            *this = Arc::new((**this).clone());
        } else if Arc::weak_count(this) != 0 {
            unsafe {
                let old = this.ptr;
                let fresh = Arc::new(intrinsics::read(&(*old).value as *const T));
                (*old).strong.store(0, SeqCst);
                (*old).weak.fetch_sub(1, SeqCst);
                intrinsics::write(this as *mut Arc<T>, fresh);
            }
        }
        unsafe { &mut (*this.ptr).value }
    }
}

impl<T: ?Sized> Arc<T> {
    #[inline]
    fn inner(&self) -> &ArcInner<T> {
        unsafe { &*self.ptr }
    }

    /// Provides a raw pointer to the data.
    #[inline]
    pub fn as_ptr(this: &Self) -> *const T {
        unsafe { &(*this.ptr).value as *const T }
    }

    /// Creates a new `Weak` pointer to this allocation.
    pub fn downgrade(this: &Self) -> Weak<T> {
        this.inner().weak.fetch_add(1, SeqCst);
        Weak { ptr: this.ptr }
    }

    /// Gets the number of `Weak` pointers to this allocation.
    #[inline]
    pub fn weak_count(this: &Self) -> usize {
        this.inner().weak.load(SeqCst) - 1
    }

    /// Gets the number of strong (`Arc`) pointers to this allocation.
    #[inline]
    pub fn strong_count(this: &Self) -> usize {
        this.inner().strong.load(SeqCst)
    }

    /// Increments the strong reference count on the `Arc<T>` associated with the provided pointer by one.
    pub unsafe fn increment_strong_count(ptr: *const T)
    where
        T: Sized,
    {
        let rc = mem::ManuallyDrop::new(Arc::<T>::from_raw(ptr));
        rc.inner().strong.fetch_add(1, SeqCst);
    }

    /// Decrements the strong reference count on the `Arc<T>` associated with the provided pointer by one.
    pub unsafe fn decrement_strong_count(ptr: *const T)
    where
        T: Sized,
    {
        drop(Arc::<T>::from_raw(ptr));
    }

    /// Returns a mutable reference into the given `Arc`, if there are no other `Arc` or `Weak` pointers to the same allocation.
    #[inline]
    pub fn get_mut(this: &mut Self) -> Option<&mut T> {
        if Arc::strong_count(this) == 1 && Arc::weak_count(this) == 0 {
            unsafe { Some(&mut (*this.ptr).value) }
        } else {
            None
        }
    }

    /// Returns `true` if the two `Arc`s point to the same allocation.
    #[inline]
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        (this.ptr as *const u8 as usize) == (other.ptr as *const u8 as usize)
    }
}

impl<T: ?Sized> Clone for Arc<T> {
    #[inline]
    fn clone(&self) -> Arc<T> {
        self.inner().strong.fetch_add(1, SeqCst);
        Arc { ptr: self.ptr }
    }
}

impl<T: ?Sized> Drop for Arc<T> {
    fn drop(&mut self) {
        unsafe {
            let p = self.ptr;
            if (*p).strong.fetch_sub(1, SeqCst) == 1 {
                intrinsics::drop_in_place(&mut (*p).value as *mut T);
                if (*p).weak.fetch_sub(1, SeqCst) == 1 {
                    let size = intrinsics::size_of_val(p as *const ArcInner<T>);
                    let align = intrinsics::align_of_val(p as *const ArcInner<T>);
                    intrinsics::dealloc(p as *mut u8, size, align);
                }
            }
        }
    }
}

impl<T: ?Sized> Deref for Arc<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        &self.inner().value
    }
}

impl<T: ?Sized> AsRef<T> for Arc<T> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized> Borrow<T> for Arc<T> {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T: Default> Default for Arc<T> {
    #[inline]
    fn default() -> Arc<T> {
        Arc::new(Default::default())
    }
}

impl<T: ?Sized + PartialEq> PartialEq for Arc<T> {
    #[inline]
    fn eq(&self, other: &Arc<T>) -> bool {
        **self == **other
    }
    #[inline]
    fn ne(&self, other: &Arc<T>) -> bool {
        **self != **other
    }
}

impl<T: ?Sized + Eq> Eq for Arc<T> {}

impl<T: ?Sized + PartialOrd> PartialOrd for Arc<T> {
    #[inline]
    fn partial_cmp(&self, other: &Arc<T>) -> Option<Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl<T: ?Sized + Ord> Ord for Arc<T> {
    #[inline]
    fn cmp(&self, other: &Arc<T>) -> Ordering {
        (**self).cmp(&**other)
    }
}

impl<T: ?Sized + Hash> Hash for Arc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for Arc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Arc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized> fmt::Pointer for Arc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p: *const T = Arc::as_ptr(self);
        fmt::Pointer::fmt(&p, f)
    }
}

impl<T> From<T> for Arc<T> {
    fn from(t: T) -> Self {
        Arc::new(t)
    }
}

impl From<&str> for Arc<str> {
    fn from(v: &str) -> Arc<str> {
        unsafe {
            let len = v.len();
            let (p, elems) = allocate_for_slice::<u8>(len);
            intrinsics::copy_nonoverlapping(v.as_ptr(), elems, len);
            Arc { ptr: p as *mut ArcInner<str> }
        }
    }
}

impl<T: Clone> From<&[T]> for Arc<[T]> {
    fn from(v: &[T]) -> Arc<[T]> {
        unsafe {
            let len = v.len();
            let (p, elems) = allocate_for_slice::<T>(len);
            let mut i = 0;
            while i < len {
                intrinsics::write(intrinsics::ptr_add_mut(elems, i), v[i].clone());
                i += 1;
            }
            Arc { ptr: p }
        }
    }
}

impl<T> From<Vec<T>> for Arc<[T]> {
    fn from(v: Vec<T>) -> Arc<[T]> {
        unsafe {
            let len = v.len();
            let (p, elems) = allocate_for_slice::<T>(len);
            let mut v = v;
            intrinsics::copy_nonoverlapping(v.as_ptr(), elems, len);
            v.set_len(0);
            Arc { ptr: p }
        }
    }
}

impl<T> From<Box<T>> for Arc<T> {
    fn from(b: Box<T>) -> Arc<T> {
        Arc::new(Box::into_inner(b))
    }
}

impl<T> FromIterator<T> for Arc<[T]> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let v: Vec<T> = iter.into_iter().collect();
        Arc::from(v)
    }
}

// ---------------------------------------------------------------------------
// Weak

impl<T> Weak<T> {
    /// Constructs a new `Weak<T>`, without allocating any memory.
    pub const fn new() -> Weak<T> {
        Weak { ptr: 0usize as *mut ArcInner<T> }
    }
}

impl<T: ?Sized> Weak<T> {
    #[inline]
    fn is_dangling(&self) -> bool {
        (self.ptr as *const u8 as usize) == 0
    }

    /// Attempts to upgrade the `Weak` pointer to an `Arc`.
    pub fn upgrade(&self) -> Option<Arc<T>> {
        if self.is_dangling() {
            return None;
        }
        unsafe {
            let mut s = (*self.ptr).strong.load(SeqCst);
            loop {
                if s == 0 {
                    return None;
                }
                match (*self.ptr).strong.compare_exchange_weak(s, s + 1, SeqCst, SeqCst) {
                    Ok(_) => return Some(Arc { ptr: self.ptr }),
                    Err(old) => s = old,
                }
            }
        }
    }

    /// Gets the number of strong (`Arc`) pointers pointing to this allocation.
    pub fn strong_count(&self) -> usize {
        if self.is_dangling() {
            0
        } else {
            unsafe { (*self.ptr).strong.load(SeqCst) }
        }
    }

    /// Gets the number of `Weak` pointers pointing to this allocation.
    pub fn weak_count(&self) -> usize {
        if self.is_dangling() {
            return 0;
        }
        unsafe {
            let strong = (*self.ptr).strong.load(SeqCst);
            if strong > 0 {
                (*self.ptr).weak.load(SeqCst) - 1
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
                (*self.ptr).weak.fetch_add(1, SeqCst);
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
            if (*p).weak.fetch_sub(1, SeqCst) == 1 {
                let size = intrinsics::size_of_val(p as *const ArcInner<T>);
                let align = intrinsics::align_of_val(p as *const ArcInner<T>);
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


impl From<String> for Arc<str> {
    fn from(v: String) -> Arc<str> {
        Arc::from(v.as_str())
    }
}

// ---------------------------------------------------------------------------
// Uninitialized values and downcasting

impl<T> Arc<T> {
    /// Constructs a new `Arc` with uninitialized contents.
    pub fn new_uninit() -> Arc<mem::MaybeUninit<T>> {
        Arc::new(mem::MaybeUninit::uninit())
    }

    /// Constructs a new `Arc` with uninitialized contents, with the memory being filled with `0` bytes.
    pub fn new_zeroed() -> Arc<mem::MaybeUninit<T>> {
        Arc::new(mem::MaybeUninit::zeroed())
    }

    /// Constructs a new reference-counted slice with uninitialized contents.
    pub fn new_uninit_slice(len: usize) -> Arc<[mem::MaybeUninit<T>]> {
        unsafe {
            let (p, _) = allocate_for_slice::<mem::MaybeUninit<T>>(len);
            Arc { ptr: p }
        }
    }

    /// Constructs a new reference-counted slice with uninitialized contents, with the memory being filled with `0` bytes.
    pub fn new_zeroed_slice(len: usize) -> Arc<[mem::MaybeUninit<T>]> {
        unsafe {
            let (p, elems) = allocate_for_slice::<mem::MaybeUninit<T>>(len);
            intrinsics::write_bytes(elems as *mut u8, 0, len * mem::size_of::<T>());
            Arc { ptr: p }
        }
    }
}

impl<T> Arc<mem::MaybeUninit<T>> {
    /// Converts to `Arc<T>`.
    pub unsafe fn assume_init(self) -> Arc<T> {
        let p = self.ptr as *mut ArcInner<T>;
        mem::forget(self);
        Arc { ptr: p }
    }
}

impl<T> Arc<[mem::MaybeUninit<T>]> {
    /// Converts to `Arc<[T]>`.
    pub unsafe fn assume_init(self) -> Arc<[T]> {
        let p = self.ptr as *mut ArcInner<[T]>;
        mem::forget(self);
        Arc { ptr: p }
    }
}

impl Arc<dyn crate::any::Any + Send + Sync> {
    /// Attempts to downcast the `Arc<dyn crate::any::Any + Send + Sync>` to a concrete type.
    pub fn downcast<T: crate::any::Any + Send + Sync>(self) -> Result<Arc<T>, Self> {
        if (*self).is::<T>() {
            let p = self.ptr as *mut ArcInner<T>;
            mem::forget(self);
            Ok(Arc { ptr: p })
        } else {
            Err(self)
        }
    }
}
