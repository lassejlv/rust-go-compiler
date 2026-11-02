//! Manually manage memory through raw pointers.

use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::intrinsics;

/// Creates a null raw pointer.
#[inline]
pub const fn null<T>() -> *const T {
    0usize as *const T
}

/// Creates a null mutable raw pointer.
#[inline]
pub const fn null_mut<T>() -> *mut T {
    0usize as *mut T
}

/// Creates a new pointer that is dangling, but non-null and well-aligned.
#[inline]
pub fn dangling<T>() -> *const T {
    crate::mem::align_of::<T>() as *const T
}

/// Creates a new mutable pointer that is dangling, but non-null and well-aligned.
#[inline]
pub fn dangling_mut<T>() -> *mut T {
    crate::mem::align_of::<T>() as *mut T
}

/// Reads the value from `src` without moving it.
#[inline]
pub unsafe fn read<T>(src: *const T) -> T {
    intrinsics::read(src)
}

/// Reads the value from `src` without moving it (no alignment requirement).
#[inline]
pub unsafe fn read_unaligned<T>(src: *const T) -> T {
    intrinsics::read(src)
}

/// Overwrites a memory location with the given value without reading or dropping the old value.
#[inline]
pub unsafe fn write<T>(dst: *mut T, src: T) {
    intrinsics::write(dst, src)
}

/// Overwrites a memory location (no alignment requirement).
#[inline]
pub unsafe fn write_unaligned<T>(dst: *mut T, src: T) {
    intrinsics::write(dst, src)
}

/// Copies `count * size_of::<T>()` bytes from `src` to `dst`. The regions may overlap.
#[inline]
pub unsafe fn copy<T>(src: *const T, dst: *mut T, count: usize) {
    intrinsics::copy(src, dst, count)
}

/// Copies `count * size_of::<T>()` bytes from `src` to `dst`. The regions must not overlap.
#[inline]
pub unsafe fn copy_nonoverlapping<T>(src: *const T, dst: *mut T, count: usize) {
    intrinsics::copy_nonoverlapping(src, dst, count)
}

/// Sets `count * size_of::<T>()` bytes of memory starting at `dst` to `val`.
#[inline]
pub unsafe fn write_bytes<T>(dst: *mut T, val: u8, count: usize) {
    intrinsics::write_bytes(dst, val, count)
}

/// Executes the destructor (if any) of the pointed-to value.
#[inline]
pub unsafe fn drop_in_place<T: ?Sized>(to_drop: *mut T) {
    intrinsics::drop_in_place(to_drop)
}

/// Swaps the values at two mutable locations of the same type.
#[inline]
pub unsafe fn swap<T>(x: *mut T, y: *mut T) {
    let tmp = intrinsics::read(x as *const T);
    intrinsics::copy(y as *const T, x, 1);
    intrinsics::write(y, tmp);
}

/// Swaps `count * size_of::<T>()` bytes between the two non-overlapping regions.
#[inline]
pub unsafe fn swap_nonoverlapping<T>(x: *mut T, y: *mut T, count: usize) {
    let mut i = 0;
    while i < count {
        swap(intrinsics::ptr_add_mut(x, i), intrinsics::ptr_add_mut(y, i));
        i += 1;
    }
}

/// Moves `src` into the pointed `dst`, returning the previous `dst` value.
#[inline]
pub unsafe fn replace<T>(dst: *mut T, src: T) -> T {
    let old = intrinsics::read(dst as *const T);
    intrinsics::write(dst, src);
    old
}

/// Compares raw pointers for equality (address and metadata).
#[inline]
pub fn eq<T: ?Sized>(a: *const T, b: *const T) -> bool {
    a == b
}

/// Compares the addresses of the two pointers for equality, ignoring metadata.
#[inline]
pub fn addr_eq<T: ?Sized, U: ?Sized>(p: *const T, q: *const U) -> bool {
    (p as *const u8 as usize) == (q as *const u8 as usize)
}

/// Forms a raw slice from a pointer and a length.
#[inline]
pub fn slice_from_raw_parts<T>(data: *const T, len: usize) -> *const [T] {
    unsafe { intrinsics::slice_from_raw_parts(data, len) }
}

/// Forms a raw mutable slice from a pointer and a length.
#[inline]
pub fn slice_from_raw_parts_mut<T>(data: *mut T, len: usize) -> *mut [T] {
    unsafe { intrinsics::slice_from_raw_parts_mut(data, len) }
}

/// Hash a raw pointer (its address).
pub fn hash<T: ?Sized, S: Hasher>(hashee: *const T, into: &mut S) {
    (hashee as *const u8 as usize).hash(into);
}

// ---------------------------------------------------------------------------
// Inherent methods on raw pointers

impl<T: ?Sized> *const T {
    /// Returns `true` if the pointer is null.
    #[inline]
    pub fn is_null(self) -> bool {
        (self as *const u8 as usize) == 0
    }

    /// Casts to a pointer of another type.
    #[inline]
    pub const fn cast<U>(self) -> *const U {
        self as *const U
    }

    /// Changes constness without changing the type.
    #[inline]
    pub const fn cast_mut(self) -> *mut T {
        self as *mut T
    }

    /// Gets the "address" portion of the pointer.
    #[inline]
    pub fn addr(self) -> usize {
        self as *const u8 as usize
    }

    /// Returns `None` if the pointer is null, or else returns a shared reference to the value.
    #[inline]
    pub unsafe fn as_ref<'a>(self) -> Option<&'a T> {
        if self.is_null() {
            None
        } else {
            Some(&*self)
        }
    }
}

impl<T> *const T {
    /// Adds an unsigned offset to a pointer.
    #[inline]
    pub unsafe fn add(self, count: usize) -> *const T {
        intrinsics::ptr_add(self, count)
    }

    /// Subtracts an unsigned offset from a pointer.
    #[inline]
    pub unsafe fn sub(self, count: usize) -> *const T {
        intrinsics::ptr_offset(self, 0isize - (count as isize))
    }

    /// Adds a signed offset to a pointer.
    #[inline]
    pub unsafe fn offset(self, count: isize) -> *const T {
        intrinsics::ptr_offset(self, count)
    }

    /// Adds an unsigned offset to a pointer using wrapping arithmetic.
    #[inline]
    pub fn wrapping_add(self, count: usize) -> *const T {
        unsafe { intrinsics::ptr_add(self, count) }
    }

    /// Subtracts an unsigned offset from a pointer using wrapping arithmetic.
    #[inline]
    pub fn wrapping_sub(self, count: usize) -> *const T {
        unsafe { intrinsics::ptr_offset(self, 0isize - (count as isize)) }
    }

    /// Calculates the distance between two pointers, in units of `T`.
    #[inline]
    pub unsafe fn offset_from(self, origin: *const T) -> isize {
        intrinsics::ptr_diff(self, origin)
    }

    /// Reads the value from `self` without moving it.
    #[inline]
    pub unsafe fn read(self) -> T {
        intrinsics::read(self)
    }

    /// Reads the value from `self` without moving it (no alignment requirement).
    #[inline]
    pub unsafe fn read_unaligned(self) -> T {
        intrinsics::read(self)
    }

    /// Copies `count * size_of::<T>()` bytes from `self` to `dest`. The regions may overlap.
    #[inline]
    pub unsafe fn copy_to(self, dest: *mut T, count: usize) {
        intrinsics::copy(self, dest, count)
    }

    /// Copies `count * size_of::<T>()` bytes from `self` to `dest`. The regions must not overlap.
    #[inline]
    pub unsafe fn copy_to_nonoverlapping(self, dest: *mut T, count: usize) {
        intrinsics::copy_nonoverlapping(self, dest, count)
    }
}

impl<T: ?Sized> *mut T {
    /// Returns `true` if the pointer is null.
    #[inline]
    pub fn is_null(self) -> bool {
        (self as *const u8 as usize) == 0
    }

    /// Casts to a pointer of another type.
    #[inline]
    pub const fn cast<U>(self) -> *mut U {
        self as *mut U
    }

    /// Changes constness without changing the type.
    #[inline]
    pub const fn cast_const(self) -> *const T {
        self as *const T
    }

    /// Gets the "address" portion of the pointer.
    #[inline]
    pub fn addr(self) -> usize {
        self as *const u8 as usize
    }

    /// Returns `None` if the pointer is null, or else returns a shared reference to the value.
    #[inline]
    pub unsafe fn as_ref<'a>(self) -> Option<&'a T> {
        if self.is_null() {
            None
        } else {
            Some(&*self)
        }
    }

    /// Returns `None` if the pointer is null, or else returns a unique reference to the value.
    #[inline]
    pub unsafe fn as_mut<'a>(self) -> Option<&'a mut T> {
        if self.is_null() {
            None
        } else {
            Some(&mut *self)
        }
    }

    /// Executes the destructor (if any) of the pointed-to value.
    #[inline]
    pub unsafe fn drop_in_place(self) {
        intrinsics::drop_in_place(self)
    }
}

impl<T> *mut T {
    /// Adds an unsigned offset to a pointer.
    #[inline]
    pub unsafe fn add(self, count: usize) -> *mut T {
        intrinsics::ptr_add_mut(self, count)
    }

    /// Subtracts an unsigned offset from a pointer.
    #[inline]
    pub unsafe fn sub(self, count: usize) -> *mut T {
        intrinsics::ptr_offset_mut(self, 0isize - (count as isize))
    }

    /// Adds a signed offset to a pointer.
    #[inline]
    pub unsafe fn offset(self, count: isize) -> *mut T {
        intrinsics::ptr_offset_mut(self, count)
    }

    /// Adds an unsigned offset to a pointer using wrapping arithmetic.
    #[inline]
    pub fn wrapping_add(self, count: usize) -> *mut T {
        unsafe { intrinsics::ptr_add_mut(self, count) }
    }

    /// Subtracts an unsigned offset from a pointer using wrapping arithmetic.
    #[inline]
    pub fn wrapping_sub(self, count: usize) -> *mut T {
        unsafe { intrinsics::ptr_offset_mut(self, 0isize - (count as isize)) }
    }

    /// Calculates the distance between two pointers, in units of `T`.
    #[inline]
    pub unsafe fn offset_from(self, origin: *const T) -> isize {
        intrinsics::ptr_diff(self as *const T, origin)
    }

    /// Reads the value from `self` without moving it.
    #[inline]
    pub unsafe fn read(self) -> T {
        intrinsics::read(self as *const T)
    }

    /// Overwrites a memory location with the given value without reading or dropping the old value.
    #[inline]
    pub unsafe fn write(self, val: T) {
        intrinsics::write(self, val)
    }

    /// Replaces the value at `self` with `src`, returning the old value.
    #[inline]
    pub unsafe fn replace(self, src: T) -> T {
        replace(self, src)
    }

    /// Swaps the values at two mutable locations of the same type.
    #[inline]
    pub unsafe fn swap(self, with: *mut T) {
        swap(self, with)
    }

    /// Copies `count * size_of::<T>()` bytes from `self` to `dest`. The regions may overlap.
    #[inline]
    pub unsafe fn copy_to(self, dest: *mut T, count: usize) {
        intrinsics::copy(self as *const T, dest, count)
    }

    /// Copies `count * size_of::<T>()` bytes from `self` to `dest`. The regions must not overlap.
    #[inline]
    pub unsafe fn copy_to_nonoverlapping(self, dest: *mut T, count: usize) {
        intrinsics::copy_nonoverlapping(self as *const T, dest, count)
    }

    /// Copies `count * size_of::<T>()` bytes from `src` to `self`. The regions may overlap.
    #[inline]
    pub unsafe fn copy_from(self, src: *const T, count: usize) {
        intrinsics::copy(src, self, count)
    }

    /// Copies `count * size_of::<T>()` bytes from `src` to `self`. The regions must not overlap.
    #[inline]
    pub unsafe fn copy_from_nonoverlapping(self, src: *const T, count: usize) {
        intrinsics::copy_nonoverlapping(src, self, count)
    }

    /// Sets `count * size_of::<T>()` bytes of memory starting at `self` to `val`.
    #[inline]
    pub unsafe fn write_bytes(self, val: u8, count: usize) {
        intrinsics::write_bytes(self, val, count)
    }
}

impl<T> *const [T] {
    /// Returns the length of a raw slice.
    #[inline]
    pub fn len(self) -> usize {
        unsafe { intrinsics::slice_len(self) }
    }

    /// Returns `true` if the raw slice has a length of 0.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Returns a raw pointer to the slice's buffer.
    #[inline]
    pub fn as_ptr(self) -> *const T {
        unsafe { intrinsics::slice_ptr(self) }
    }
}

impl<T> *mut [T] {
    /// Returns the length of a raw slice.
    #[inline]
    pub fn len(self) -> usize {
        unsafe { intrinsics::slice_len(self as *const [T]) }
    }

    /// Returns `true` if the raw slice has a length of 0.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Returns a raw pointer to the slice's buffer.
    #[inline]
    pub fn as_mut_ptr(self) -> *mut T {
        unsafe { intrinsics::slice_ptr(self as *const [T]) as *mut T }
    }
}

// ---------------------------------------------------------------------------
// Formatting and hashing of raw pointers

impl<T: ?Sized> fmt::Debug for *const T {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(self, f)
    }
}

impl<T: ?Sized> fmt::Debug for *mut T {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(self, f)
    }
}

impl<T: ?Sized> fmt::Pointer for *const T {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::pointer_fmt_inner(*self as *const u8 as usize, f)
    }
}

impl<T: ?Sized> fmt::Pointer for *mut T {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::pointer_fmt_inner(*self as *const u8 as usize, f)
    }
}

impl<T: ?Sized> Hash for *const T {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(*self as *const u8 as usize);
    }
}

impl<T: ?Sized> Hash for *mut T {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(*self as *const u8 as usize);
    }
}

// ---------------------------------------------------------------------------
// NonNull

/// `*mut T` but non-zero and covariant.
pub struct NonNull<T: ?Sized> {
    pointer: *const T,
}

impl<T> NonNull<T> {
    /// Creates a new `NonNull` that is dangling, but well-aligned.
    #[inline]
    pub fn dangling() -> Self {
        NonNull { pointer: dangling::<T>() }
    }
}

impl<T: ?Sized> NonNull<T> {
    /// Creates a new `NonNull`. `ptr` must be non-null.
    #[inline]
    pub const unsafe fn new_unchecked(ptr: *mut T) -> Self {
        NonNull { pointer: ptr as *const T }
    }

    /// Creates a new `NonNull` if `ptr` is non-null.
    #[inline]
    pub fn new(ptr: *mut T) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            Some(NonNull { pointer: ptr as *const T })
        }
    }

    /// Acquires the underlying `*mut` pointer.
    #[inline]
    pub const fn as_ptr(self) -> *mut T {
        self.pointer as *mut T
    }

    /// Returns a shared reference to the value.
    #[inline]
    pub unsafe fn as_ref<'a>(&self) -> &'a T {
        &*self.pointer
    }

    /// Returns a unique reference to the value.
    #[inline]
    pub unsafe fn as_mut<'a>(&mut self) -> &'a mut T {
        &mut *(self.pointer as *mut T)
    }

    /// Casts to a pointer of another type.
    #[inline]
    pub fn cast<U>(self) -> NonNull<U> {
        NonNull { pointer: self.pointer as *const U }
    }
}

impl<T: ?Sized> Clone for NonNull<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for NonNull<T> {}

impl<T: ?Sized> PartialEq for NonNull<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        (self.pointer as *const u8 as usize) == (other.pointer as *const u8 as usize)
    }
}

impl<T: ?Sized> Eq for NonNull<T> {}

impl<T: ?Sized> PartialOrd for NonNull<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: ?Sized> Ord for NonNull<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.pointer as *const u8 as usize).cmp(&(other.pointer as *const u8 as usize))
    }
}

impl<T: ?Sized> Hash for NonNull<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.pointer as *const u8 as usize);
    }
}

impl<T: ?Sized> fmt::Debug for NonNull<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.pointer, f)
    }
}

impl<T: ?Sized> fmt::Pointer for NonNull<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.pointer, f)
    }
}

impl<T: ?Sized> From<&mut T> for NonNull<T> {
    #[inline]
    fn from(r: &mut T) -> Self {
        NonNull { pointer: r as *mut T as *const T }
    }
}

impl<T: ?Sized> From<&T> for NonNull<T> {
    #[inline]
    fn from(r: &T) -> Self {
        NonNull { pointer: r as *const T }
    }
}
