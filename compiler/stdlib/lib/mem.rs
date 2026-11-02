//! Basic functions for dealing with memory.

use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::intrinsics;
use crate::marker::PhantomData;
use crate::ops::{Deref, DerefMut};

pub use crate::intrinsics::transmute;

/// Returns the size of a type in bytes.
#[inline]
pub const fn size_of<T>() -> usize {
    unsafe { intrinsics::size_of::<T>() }
}

/// Returns the size of the pointed-to value in bytes.
#[inline]
pub fn size_of_val<T: ?Sized>(val: &T) -> usize {
    unsafe { intrinsics::size_of_val::<T>(val as *const T) }
}

/// Returns the ABI-required minimum alignment of a type in bytes.
#[inline]
pub const fn align_of<T>() -> usize {
    unsafe { intrinsics::align_of::<T>() }
}

/// Returns the ABI-required minimum alignment of the type of the value that `val` points to in bytes.
#[inline]
pub fn align_of_val<T: ?Sized>(val: &T) -> usize {
    unsafe { intrinsics::align_of_val::<T>(val as *const T) }
}

/// Returns the minimum alignment of a type (deprecated alias).
#[inline]
pub fn min_align_of<T>() -> usize {
    align_of::<T>()
}

/// Returns `true` if dropping values of type `T` matters.
#[inline]
pub const fn needs_drop<T: ?Sized>() -> bool {
    unsafe { intrinsics::needs_drop::<T>() }
}

/// Swaps the values at two mutable locations, without deinitializing either one.
#[inline]
pub fn swap<T>(x: &mut T, y: &mut T) {
    unsafe {
        let px = x as *mut T;
        let py = y as *mut T;
        let tmp = intrinsics::read(px as *const T);
        intrinsics::copy_nonoverlapping(py as *const T, px, 1);
        intrinsics::write(py, tmp);
    }
}

/// Replaces `dest` with the default value of `T`, returning the previous `dest` value.
#[inline]
pub fn take<T: Default>(dest: &mut T) -> T {
    replace(dest, T::default())
}

/// Moves `src` into the referenced `dest`, returning the previous `dest` value.
#[inline]
pub fn replace<T>(dest: &mut T, src: T) -> T {
    unsafe {
        let p = dest as *mut T;
        let result = intrinsics::read(p as *const T);
        intrinsics::write(p, src);
        result
    }
}

/// Disposes of a value.
#[inline]
pub fn drop<T>(_x: T) {}

/// Takes ownership and "forgets" about the value without running its destructor.
#[inline]
pub fn forget<T>(t: T) {
    unsafe { intrinsics::forget(t) }
}

/// Returns the value of type `T` represented by the all-zero byte-pattern.
#[inline]
pub unsafe fn zeroed<T>() -> T {
    intrinsics::zeroed::<T>()
}

/// Bypasses normal memory-initialization checks by pretending to produce a value of type `T`.
#[inline]
pub unsafe fn uninitialized<T>() -> T {
    intrinsics::uninit::<T>()
}

/// Reads `src` as having type `&Dst` and copies the value out.
#[inline]
pub unsafe fn transmute_copy<Src, Dst>(src: &Src) -> Dst {
    intrinsics::read(src as *const Src as *const Dst)
}

/// Opaque type representing the discriminant of an enum.
pub struct Discriminant<T> {
    value: i64,
    _marker: PhantomData<T>,
}

impl<T> Copy for Discriminant<T> {}

impl<T> Clone for Discriminant<T> {
    fn clone(&self) -> Self {
        Discriminant { value: self.value, _marker: PhantomData }
    }
}

impl<T> PartialEq for Discriminant<T> {
    fn eq(&self, rhs: &Self) -> bool {
        self.value == rhs.value
    }
}

impl<T> Eq for Discriminant<T> {}

impl<T> Hash for Discriminant<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T> fmt::Debug for Discriminant<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_tuple("Discriminant").field(&self.value).finish()
    }
}

/// Returns a value uniquely identifying the enum variant in `v`.
#[inline]
pub fn discriminant<T>(v: &T) -> Discriminant<T> {
    Discriminant { value: unsafe { intrinsics::discriminant_value(v) }, _marker: PhantomData }
}

// ---------------------------------------------------------------------------
// ManuallyDrop

/// A wrapper to inhibit the compiler from automatically calling `T`'s destructor.
#[lang = "manually_drop"]
pub struct ManuallyDrop<T: ?Sized> {
    value: T,
}

impl<T> ManuallyDrop<T> {
    /// Wrap a value to be manually dropped.
    #[inline]
    pub const fn new(value: T) -> ManuallyDrop<T> {
        ManuallyDrop { value }
    }

    /// Extracts the value from the `ManuallyDrop` container.
    #[inline]
    pub fn into_inner(slot: ManuallyDrop<T>) -> T {
        unsafe { intrinsics::read(&slot.value as *const T) }
    }

    /// Takes the value from the `ManuallyDrop<T>` container out.
    #[inline]
    pub unsafe fn take(slot: &mut ManuallyDrop<T>) -> T {
        intrinsics::read(&slot.value as *const T)
    }
}

impl<T: ?Sized> ManuallyDrop<T> {
    /// Manually drops the contained value.
    #[inline]
    pub unsafe fn drop(slot: &mut ManuallyDrop<T>) {
        intrinsics::drop_in_place(&mut slot.value as *mut T)
    }
}

impl<T: ?Sized> Deref for ManuallyDrop<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: ?Sized> DerefMut for ManuallyDrop<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T: Clone> Clone for ManuallyDrop<T> {
    fn clone(&self) -> Self {
        ManuallyDrop::new(self.value.clone())
    }
}

impl<T: Copy> Copy for ManuallyDrop<T> {}

impl<T: Default> Default for ManuallyDrop<T> {
    fn default() -> Self {
        ManuallyDrop::new(T::default())
    }
}

impl<T: ?Sized + PartialEq> PartialEq for ManuallyDrop<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: ?Sized + Eq> Eq for ManuallyDrop<T> {}

impl<T: ?Sized + PartialOrd> PartialOrd for ManuallyDrop<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<T: ?Sized + Ord> Ord for ManuallyDrop<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T: ?Sized + Hash> Hash for ManuallyDrop<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for ManuallyDrop<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ManuallyDrop").field("value", &self.value).finish()
    }
}

// ---------------------------------------------------------------------------
// MaybeUninit

/// A wrapper type to construct uninitialized instances of `T`.
pub struct MaybeUninit<T> {
    value: ManuallyDrop<T>,
}

impl<T: Copy> Copy for MaybeUninit<T> {}

impl<T: Copy> Clone for MaybeUninit<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> MaybeUninit<T> {
    /// Creates a new `MaybeUninit<T>` initialized with the given value.
    #[inline]
    pub const fn new(val: T) -> MaybeUninit<T> {
        MaybeUninit { value: ManuallyDrop::new(val) }
    }

    /// Creates a new `MaybeUninit<T>` in an uninitialized state.
    #[inline]
    pub fn uninit() -> MaybeUninit<T> {
        MaybeUninit { value: ManuallyDrop::new(unsafe { intrinsics::uninit::<T>() }) }
    }

    /// Creates a new `MaybeUninit<T>` with the memory filled with `0` bytes.
    #[inline]
    pub fn zeroed() -> MaybeUninit<T> {
        MaybeUninit { value: ManuallyDrop::new(unsafe { intrinsics::zeroed::<T>() }) }
    }

    /// Sets the value of the `MaybeUninit<T>`.
    #[inline]
    pub fn write(&mut self, val: T) -> &mut T {
        unsafe {
            intrinsics::write(self.as_mut_ptr(), val);
            &mut *self.as_mut_ptr()
        }
    }

    /// Gets a pointer to the contained value.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        &self.value as *const ManuallyDrop<T> as *const T
    }

    /// Gets a mutable pointer to the contained value.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        &mut self.value as *mut ManuallyDrop<T> as *mut T
    }

    /// Extracts the value from the `MaybeUninit<T>` container.
    #[inline]
    pub unsafe fn assume_init(self) -> T {
        ManuallyDrop::into_inner(self.value)
    }

    /// Reads the value from the `MaybeUninit<T>` container.
    #[inline]
    pub unsafe fn assume_init_read(&self) -> T {
        intrinsics::read(self.as_ptr())
    }

    /// Drops the contained value in place.
    #[inline]
    pub unsafe fn assume_init_drop(&mut self) {
        intrinsics::drop_in_place(self.as_mut_ptr())
    }

    /// Gets a shared reference to the contained value.
    #[inline]
    pub unsafe fn assume_init_ref(&self) -> &T {
        &*self.as_ptr()
    }

    /// Gets a mutable reference to the contained value.
    #[inline]
    pub unsafe fn assume_init_mut(&mut self) -> &mut T {
        &mut *self.as_mut_ptr()
    }
}

impl<T> fmt::Debug for MaybeUninit<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MaybeUninit<")?;
        f.write_str(intrinsics_type_name::<T>())?;
        f.write_str(">")
    }
}

#[inline]
fn intrinsics_type_name<T: ?Sized>() -> &'static str {
    unsafe { intrinsics::type_name::<T>() }
}
