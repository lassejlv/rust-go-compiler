//! Shareable mutable containers.

use crate::cmp::Ordering;
use crate::fmt;
use crate::marker::PhantomData;
use crate::mem;
use crate::ops::{Deref, DerefMut};

/// The core primitive for interior mutability in Rust.
#[lang = "unsafe_cell"]
#[repr(transparent)]
pub struct UnsafeCell<T: ?Sized> {
    value: T,
}

impl<T> UnsafeCell<T> {
    /// Constructs a new instance of `UnsafeCell` which will wrap the specified value.
    #[inline]
    pub const fn new(value: T) -> UnsafeCell<T> {
        UnsafeCell { value }
    }

    /// Unwraps the value, consuming the cell.
    #[inline]
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T: ?Sized> UnsafeCell<T> {
    /// Gets a mutable pointer to the wrapped value.
    #[inline]
    pub const fn get(&self) -> *mut T {
        &self.value as *const T as *mut T
    }

    /// Returns a mutable reference to the underlying data.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }

    /// Gets a mutable pointer to the wrapped value (from a raw pointer to the cell).
    #[inline]
    pub const fn raw_get(this: *const Self) -> *mut T {
        this as *const T as *mut T
    }
}

impl<T: Default> Default for UnsafeCell<T> {
    fn default() -> UnsafeCell<T> {
        UnsafeCell::new(Default::default())
    }
}

impl<T> From<T> for UnsafeCell<T> {
    fn from(t: T) -> UnsafeCell<T> {
        UnsafeCell::new(t)
    }
}

impl<T: ?Sized> fmt::Debug for UnsafeCell<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnsafeCell").finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Cell

/// A mutable memory location.
#[repr(transparent)]
pub struct Cell<T: ?Sized> {
    value: UnsafeCell<T>,
}

impl<T> Cell<T> {
    /// Creates a new `Cell` containing the given value.
    #[inline]
    pub const fn new(value: T) -> Cell<T> {
        Cell { value: UnsafeCell::new(value) }
    }

    /// Sets the contained value.
    #[inline]
    pub fn set(&self, val: T) {
        let _old = self.replace(val);
    }

    /// Swaps the values of two `Cell`s.
    #[inline]
    pub fn swap(&self, other: &Self) {
        if (self as *const Cell<T> as usize) == (other as *const Cell<T> as usize) {
            return;
        }
        unsafe { crate::ptr::swap(self.value.get(), other.value.get()) }
    }

    /// Replaces the contained value with `val`, and returns the old contained value.
    #[inline]
    pub fn replace(&self, val: T) -> T {
        unsafe { mem::replace(&mut *self.value.get(), val) }
    }

    /// Unwraps the value, consuming the cell.
    #[inline]
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

impl<T: Copy> Cell<T> {
    /// Returns a copy of the contained value.
    #[inline]
    pub fn get(&self) -> T {
        unsafe { *self.value.get() }
    }

    /// Updates the contained value using a function and returns the new value.
    #[inline]
    pub fn update<F>(&self, f: F) -> T
    where
        F: FnOnce(T) -> T,
    {
        let old = self.get();
        let new = f(old);
        self.set(new);
        new
    }
}

impl<T: ?Sized> Cell<T> {
    /// Returns a `&Cell<T>` from a `&mut T`.
    #[inline]
    pub fn from_mut(t: &mut T) -> &Cell<T> {
        unsafe { &*(t as *mut T as *const Cell<T>) }
    }

    /// Returns a raw pointer to the underlying data in this cell.
    #[inline]
    pub const fn as_ptr(&self) -> *mut T {
        self.value.get()
    }

    /// Returns a mutable reference to the underlying data.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }
}

impl<T: Default> Cell<T> {
    /// Takes the value of the cell, leaving `Default::default()` in its place.
    #[inline]
    pub fn take(&self) -> T {
        self.replace(Default::default())
    }
}

impl<T: Copy> Clone for Cell<T> {
    #[inline]
    fn clone(&self) -> Cell<T> {
        Cell::new(self.get())
    }
}

impl<T: Default> Default for Cell<T> {
    #[inline]
    fn default() -> Cell<T> {
        Cell::new(Default::default())
    }
}

impl<T: PartialEq + Copy> PartialEq for Cell<T> {
    #[inline]
    fn eq(&self, other: &Cell<T>) -> bool {
        self.get() == other.get()
    }
}

impl<T: Eq + Copy> Eq for Cell<T> {}

impl<T: PartialOrd + Copy> PartialOrd for Cell<T> {
    #[inline]
    fn partial_cmp(&self, other: &Cell<T>) -> Option<Ordering> {
        self.get().partial_cmp(&other.get())
    }
}

impl<T: Ord + Copy> Ord for Cell<T> {
    #[inline]
    fn cmp(&self, other: &Cell<T>) -> Ordering {
        self.get().cmp(&other.get())
    }
}

impl<T> From<T> for Cell<T> {
    fn from(t: T) -> Cell<T> {
        Cell::new(t)
    }
}

impl<T: Copy + fmt::Debug> fmt::Debug for Cell<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cell").field("value", &self.get()).finish()
    }
}

// ---------------------------------------------------------------------------
// RefCell

type BorrowFlag = isize;
const UNUSED: BorrowFlag = 0;

/// A mutable memory location with dynamically checked borrow rules.
pub struct RefCell<T: ?Sized> {
    borrow: Cell<BorrowFlag>,
    value: UnsafeCell<T>,
}

/// An error returned by `RefCell::try_borrow`.
#[non_exhaustive]
pub struct BorrowError {}

impl fmt::Debug for BorrowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BorrowError")
    }
}

impl fmt::Display for BorrowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt("RefCell already mutably borrowed", f)
    }
}

impl crate::error::Error for BorrowError {}

/// An error returned by `RefCell::try_borrow_mut`.
#[non_exhaustive]
pub struct BorrowMutError {}

impl fmt::Debug for BorrowMutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BorrowMutError")
    }
}

impl fmt::Display for BorrowMutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt("RefCell already borrowed", f)
    }
}

impl crate::error::Error for BorrowMutError {}

#[inline(never)]
#[track_caller]
fn panic_already_borrowed() -> ! {
    // (the payload is a String, as in std, which formats the error)
    panic!("{}", BorrowMutError {})
}

#[inline(never)]
#[track_caller]
fn panic_already_mutably_borrowed() -> ! {
    panic!("{}", BorrowError {})
}

impl<T> RefCell<T> {
    /// Creates a new `RefCell` containing `value`.
    #[inline]
    pub const fn new(value: T) -> RefCell<T> {
        RefCell { value: UnsafeCell::new(value), borrow: Cell::new(UNUSED) }
    }

    /// Consumes the `RefCell`, returning the wrapped value.
    #[inline]
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }

    /// Replaces the wrapped value with a new one, returning the old value.
    #[inline]
    #[track_caller]
    pub fn replace(&self, t: T) -> T {
        mem::replace(&mut *self.borrow_mut(), t)
    }

    /// Replaces the wrapped value with a new one computed from `f`, returning the old value.
    #[inline]
    #[track_caller]
    pub fn replace_with<F: FnOnce(&mut T) -> T>(&self, f: F) -> T {
        let mut_borrow = &mut *self.borrow_mut();
        let replacement = f(mut_borrow);
        mem::replace(mut_borrow, replacement)
    }

    /// Swaps the wrapped value of `self` with the wrapped value of `other`.
    #[inline]
    pub fn swap(&self, other: &Self) {
        mem::swap(&mut *self.borrow_mut(), &mut *other.borrow_mut())
    }
}

impl<T: ?Sized> RefCell<T> {
    /// Immutably borrows the wrapped value.
    #[inline]
    #[track_caller]
    pub fn borrow(&self) -> Ref<'_, T> {
        match self.try_borrow() {
            Ok(b) => b,
            Err(_) => panic_already_mutably_borrowed(),
        }
    }

    /// Immutably borrows the wrapped value, returning an error if the value is currently mutably borrowed.
    #[inline]
    pub fn try_borrow(&self) -> Result<Ref<'_, T>, BorrowError> {
        let b = self.borrow.get();
        if b < 0 {
            Err(BorrowError {})
        } else {
            self.borrow.set(b + 1);
            Ok(Ref { value: self.value.get() as *const T, borrow: &self.borrow, _marker: PhantomData })
        }
    }

    /// Mutably borrows the wrapped value.
    #[inline]
    #[track_caller]
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        match self.try_borrow_mut() {
            Ok(b) => b,
            Err(_) => panic_already_borrowed(),
        }
    }

    /// Mutably borrows the wrapped value, returning an error if the value is currently borrowed.
    #[inline]
    pub fn try_borrow_mut(&self) -> Result<RefMut<'_, T>, BorrowMutError> {
        let b = self.borrow.get();
        if b != UNUSED {
            Err(BorrowMutError {})
        } else {
            self.borrow.set(-1);
            Ok(RefMut { value: self.value.get(), borrow: &self.borrow, _marker: PhantomData })
        }
    }

    /// Returns a raw pointer to the underlying data in this cell.
    #[inline]
    pub fn as_ptr(&self) -> *mut T {
        self.value.get()
    }

    /// Returns a mutable reference to the underlying data.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    /// Immutably borrows the wrapped value, returning an error if the value is currently mutably borrowed (unchecked lifetime).
    #[inline]
    pub unsafe fn try_borrow_unguarded(&self) -> Result<&T, BorrowError> {
        if self.borrow.get() < 0 {
            Err(BorrowError {})
        } else {
            Ok(&*self.value.get())
        }
    }
}

impl<T: Default> RefCell<T> {
    /// Takes the wrapped value, leaving `Default::default()` in its place.
    #[track_caller]
    pub fn take(&self) -> T {
        self.replace(Default::default())
    }
}

impl<T: Clone> Clone for RefCell<T> {
    #[inline]
    #[track_caller]
    fn clone(&self) -> RefCell<T> {
        RefCell::new(self.borrow().clone())
    }
}

impl<T: Default> Default for RefCell<T> {
    #[inline]
    fn default() -> RefCell<T> {
        RefCell::new(Default::default())
    }
}

impl<T: ?Sized + PartialEq> PartialEq for RefCell<T> {
    #[inline]
    fn eq(&self, other: &RefCell<T>) -> bool {
        *self.borrow() == *other.borrow()
    }
}

impl<T: ?Sized + Eq> Eq for RefCell<T> {}

impl<T: ?Sized + PartialOrd> PartialOrd for RefCell<T> {
    #[inline]
    fn partial_cmp(&self, other: &RefCell<T>) -> Option<Ordering> {
        self.borrow().partial_cmp(&*other.borrow())
    }
}

impl<T: ?Sized + Ord> Ord for RefCell<T> {
    #[inline]
    fn cmp(&self, other: &RefCell<T>) -> Ordering {
        self.borrow().cmp(&*other.borrow())
    }
}

impl<T> From<T> for RefCell<T> {
    fn from(t: T) -> RefCell<T> {
        RefCell::new(t)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for RefCell<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("RefCell");
        match self.try_borrow() {
            Ok(borrow) => {
                d.field("value", &borrow);
            }
            Err(_) => {
                d.field("value", &BorrowedPlaceholder);
            }
        }
        d.finish()
    }
}

struct BorrowedPlaceholder;

impl fmt::Debug for BorrowedPlaceholder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<borrowed>")
    }
}

/// Wraps a borrowed reference to a value in a `RefCell` box.
pub struct Ref<'b, T: ?Sized + 'b> {
    value: *const T,
    borrow: &'b Cell<BorrowFlag>,
    _marker: PhantomData<&'b T>,
}

impl<'b, T: ?Sized> Drop for Ref<'b, T> {
    #[inline]
    fn drop(&mut self) {
        let b = self.borrow.get();
        self.borrow.set(b - 1);
    }
}

impl<'b, T: ?Sized> Deref for Ref<'b, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        unsafe { &*self.value }
    }
}

impl<'b, T: ?Sized> Ref<'b, T> {
    /// Copies a `Ref`.
    #[inline]
    pub fn clone(orig: &Ref<'b, T>) -> Ref<'b, T> {
        let b = orig.borrow.get();
        orig.borrow.set(b + 1);
        Ref { value: orig.value, borrow: orig.borrow, _marker: PhantomData }
    }

    /// Makes a new `Ref` for a component of the borrowed data.
    #[inline]
    pub fn map<U: ?Sized, F>(orig: Ref<'b, T>, f: F) -> Ref<'b, U>
    where
        F: FnOnce(&T) -> &U,
    {
        let value = f(unsafe { &*orig.value }) as *const U;
        let borrow = orig.borrow;
        mem::forget(orig);
        Ref { value, borrow, _marker: PhantomData }
    }

    /// Splits a `Ref` into multiple `Ref`s for different components of the borrowed data.
    #[inline]
    pub fn map_split<U: ?Sized, V: ?Sized, F>(orig: Ref<'b, T>, f: F) -> (Ref<'b, U>, Ref<'b, V>)
    where
        F: FnOnce(&T) -> (&U, &V),
    {
        let (a, b) = f(unsafe { &*orig.value });
        let (a, b) = (a as *const U, b as *const V);
        let borrow = orig.borrow;
        mem::forget(orig);
        borrow.set(borrow.get() + 1);
        (Ref { value: a, borrow, _marker: PhantomData }, Ref { value: b, borrow, _marker: PhantomData })
    }

    /// Makes a new `Ref` for an optional component of the borrowed data.
    #[inline]
    pub fn filter_map<U: ?Sized, F>(orig: Ref<'b, T>, f: F) -> Result<Ref<'b, U>, Self>
    where
        F: FnOnce(&T) -> Option<&U>,
    {
        match f(unsafe { &*orig.value }) {
            Some(v) => {
                let value = v as *const U;
                let borrow = orig.borrow;
                mem::forget(orig);
                Ok(Ref { value, borrow, _marker: PhantomData })
            }
            None => Err(orig),
        }
    }
}

impl<'b, T: ?Sized + fmt::Debug> fmt::Debug for Ref<'b, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<'b, T: ?Sized + fmt::Display> fmt::Display for Ref<'b, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

/// A wrapper type for a mutably borrowed value from a `RefCell<T>`.
pub struct RefMut<'b, T: ?Sized + 'b> {
    value: *mut T,
    borrow: &'b Cell<BorrowFlag>,
    _marker: PhantomData<&'b mut T>,
}

impl<'b, T: ?Sized> Drop for RefMut<'b, T> {
    #[inline]
    fn drop(&mut self) {
        let b = self.borrow.get();
        self.borrow.set(b + 1);
    }
}

impl<'b, T: ?Sized> Deref for RefMut<'b, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        unsafe { &*self.value }
    }
}

impl<'b, T: ?Sized> DerefMut for RefMut<'b, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value }
    }
}

impl<'b, T: ?Sized> RefMut<'b, T> {
    /// Makes a new `RefMut` for a component of the borrowed data.
    #[inline]
    pub fn map<U: ?Sized, F>(orig: RefMut<'b, T>, f: F) -> RefMut<'b, U>
    where
        F: FnOnce(&mut T) -> &mut U,
    {
        let value = f(unsafe { &mut *orig.value }) as *mut U;
        let borrow = orig.borrow;
        mem::forget(orig);
        RefMut { value, borrow, _marker: PhantomData }
    }

    /// Splits a `RefMut` into multiple `RefMut`s for different components of the borrowed data.
    #[inline]
    pub fn map_split<U: ?Sized, V: ?Sized, F>(orig: RefMut<'b, T>, f: F) -> (RefMut<'b, U>, RefMut<'b, V>)
    where
        F: FnOnce(&mut T) -> (&mut U, &mut V),
    {
        let (a, b) = f(unsafe { &mut *orig.value });
        let (a, b) = (a as *mut U, b as *mut V);
        let borrow = orig.borrow;
        mem::forget(orig);
        borrow.set(borrow.get() - 1);
        (RefMut { value: a, borrow, _marker: PhantomData }, RefMut { value: b, borrow, _marker: PhantomData })
    }

    /// Makes a new `RefMut` for an optional component of the borrowed data.
    #[inline]
    pub fn filter_map<U: ?Sized, F>(orig: RefMut<'b, T>, f: F) -> Result<RefMut<'b, U>, Self>
    where
        F: FnOnce(&mut T) -> Option<&mut U>,
    {
        let p = orig.value;
        match f(unsafe { &mut *p }) {
            Some(v) => {
                let value = v as *mut U;
                let borrow = orig.borrow;
                mem::forget(orig);
                Ok(RefMut { value, borrow, _marker: PhantomData })
            }
            None => Err(orig),
        }
    }
}

impl<'b, T: ?Sized + fmt::Debug> fmt::Debug for RefMut<'b, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<'b, T: ?Sized + fmt::Display> fmt::Display for RefMut<'b, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

// ---------------------------------------------------------------------------
// OnceCell

/// A cell which can nominally be written to only once.
pub struct OnceCell<T> {
    inner: UnsafeCell<Option<T>>,
}

impl<T> OnceCell<T> {
    /// Creates a new uninitialized cell.
    #[inline]
    pub const fn new() -> OnceCell<T> {
        OnceCell { inner: UnsafeCell::new(None) }
    }

    /// Gets the reference to the underlying value.
    #[inline]
    pub fn get(&self) -> Option<&T> {
        unsafe { &*self.inner.get() }.as_ref()
    }

    /// Gets the mutable reference to the underlying value.
    #[inline]
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.inner.get_mut().as_mut()
    }

    /// Initializes the contents of the cell to `value`.
    #[inline]
    pub fn set(&self, value: T) -> Result<(), T> {
        if self.get().is_some() {
            return Err(value);
        }
        unsafe {
            *self.inner.get() = Some(value);
        }
        Ok(())
    }

    /// Gets the contents of the cell, initializing it to `f()` if the cell was uninitialized.
    pub fn get_or_init<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        if self.get().is_none() {
            let val = f();
            unsafe {
                *self.inner.get() = Some(val);
            }
        }
        self.get().unwrap()
    }

    /// Consumes the cell, returning the wrapped value.
    #[inline]
    pub fn into_inner(self) -> Option<T> {
        self.inner.into_inner()
    }

    /// Takes the value out of this `OnceCell`, moving it back to an uninitialized state.
    #[inline]
    pub fn take(&mut self) -> Option<T> {
        mem::take(self.inner.get_mut())
    }
}

impl<T> Default for OnceCell<T> {
    fn default() -> Self {
        OnceCell::new()
    }
}

impl<T: fmt::Debug> fmt::Debug for OnceCell<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_tuple("OnceCell");
        match self.get() {
            Some(v) => d.field(v),
            None => d.field(&format_args!("<uninit>")),
        };
        d.finish()
    }
}

impl<T: Clone> Clone for OnceCell<T> {
    fn clone(&self) -> OnceCell<T> {
        let res = OnceCell::new();
        if let Some(value) = self.get() {
            let _ = res.set(value.clone());
        }
        res
    }
}

/// A value which is initialized on the first access.
pub struct LazyCell<T, F = fn() -> T> {
    cell: OnceCell<T>,
    init: Cell<Option<F>>,
}

impl<T, F: FnOnce() -> T> LazyCell<T, F> {
    /// Creates a new lazy value with the given initializing function.
    pub const fn new(f: F) -> LazyCell<T, F> {
        LazyCell { cell: OnceCell::new(), init: Cell::new(Some(f)) }
    }

    /// Forces the evaluation of this lazy value and returns a reference to the result.
    pub fn force(this: &LazyCell<T, F>) -> &T {
        if this.cell.get().is_none() {
            let f = match this.init.take() {
                Some(f) => f,
                None => crate::rt::panic_str("LazyCell instance has previously been poisoned", file!(), line!(), column!()),
            };
            let _ = this.cell.set(f());
        }
        this.cell.get().unwrap()
    }
}

impl<T, F: FnOnce() -> T> Deref for LazyCell<T, F> {
    type Target = T;
    fn deref(&self) -> &T {
        LazyCell::force(self)
    }
}

impl<T> Cell<[T]> {
    /// Returns a `&[Cell<T>]` from a `&Cell<[T]>`.
    #[inline]
    pub fn as_slice_of_cells(&self) -> &[Cell<T>] {
        let p = self.as_ptr();
        unsafe { crate::slice::from_raw_parts(p as *const T as *const Cell<T>, (*p).len()) }
    }
}

impl<T, const N: usize> Cell<[T; N]> {
    /// Returns a `&[Cell<T>; N]` from a `&Cell<[T; N]>`.
    #[inline]
    pub fn as_array_of_cells(&self) -> &[Cell<T>; N] {
        unsafe { &*(self as *const Cell<[T; N]> as *const [Cell<T>; N]) }
    }
}
