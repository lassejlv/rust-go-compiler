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
    // Byte copy: the C backend must not assume `src` is aligned.
    let mut tmp: T = intrinsics::uninit();
    intrinsics::copy_nonoverlapping(src as *const u8, &mut tmp as *mut T as *mut u8, crate::mem::size_of::<T>());
    tmp
}

/// Overwrites a memory location with the given value without reading or dropping the old value.
#[inline]
pub unsafe fn write<T>(dst: *mut T, src: T) {
    intrinsics::write(dst, src)
}

/// Overwrites a memory location (no alignment requirement).
#[inline]
pub unsafe fn write_unaligned<T>(dst: *mut T, src: T) {
    intrinsics::copy_nonoverlapping(&src as *const T as *const u8, dst as *mut u8, crate::mem::size_of::<T>());
    intrinsics::forget(src);
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

#[inline]
fn align_offset_impl(addr: usize, size: usize, align: usize) -> usize {
    if !align.is_power_of_two() {
        crate::rt::panic_str("align_offset: align is not a power-of-two", file!(), line!(), column!());
    }
    let rem = addr & (align - 1);
    if rem == 0 {
        return 0;
    }
    if size == 0 {
        return usize::MAX;
    }
    // Smallest k with (addr + k * size) % align == 0.
    let mut k = 0usize;
    let mut a = addr;
    while k < align {
        if a & (align - 1) == 0 {
            return k;
        }
        a = a.wrapping_add(size);
        k += 1;
    }
    usize::MAX
}

/// Creates a pointer with the given address and no provenance.
#[inline]
pub const fn without_provenance<T>(addr: usize) -> *const T {
    addr as *const T
}

/// Creates a mutable pointer with the given address and no provenance.
#[inline]
pub const fn without_provenance_mut<T>(addr: usize) -> *mut T {
    addr as *mut T
}

/// Converts an address back to a pointer, picking up some previously "exposed" provenance.
#[inline]
pub fn with_exposed_provenance<T>(addr: usize) -> *const T {
    addr as *const T
}

/// Converts an address back to a mutable pointer, picking up some previously "exposed" provenance.
#[inline]
pub fn with_exposed_provenance_mut<T>(addr: usize) -> *mut T {
    addr as *mut T
}

/// Converts a reference to a raw pointer.
#[inline]
pub const fn from_ref<T: ?Sized>(r: &T) -> *const T {
    r
}

/// Converts a mutable reference to a raw pointer.
#[inline]
pub fn from_mut<T: ?Sized>(r: &mut T) -> *mut T {
    r
}

/// Performs a volatile read of the value from `src` without moving it.
#[inline]
pub unsafe fn read_volatile<T>(src: *const T) -> T {
    intrinsics::read(src)
}

/// Performs a volatile write of a memory location with the given value without reading or dropping the old value.
#[inline]
pub unsafe fn write_volatile<T>(dst: *mut T, src: T) {
    intrinsics::write(dst, src)
}

/// Compares the addresses of the two function pointers for equality.
#[inline]
pub fn fn_addr_eq<T: Copy, U: Copy>(f: T, g: U) -> bool {
    unsafe { intrinsics::transmute::<T, usize>(f) == intrinsics::transmute::<U, usize>(g) }
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

    /// Returns a shared reference to the value behind the pointer (no null check).
    #[inline]
    pub unsafe fn as_ref_unchecked<'a>(self) -> &'a T {
        &*self
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
        // Integer arithmetic: the C backend must not see out-of-object pointer arithmetic.
        (self as usize).wrapping_add(count.wrapping_mul(crate::mem::size_of::<T>())) as *const T
    }

    /// Adds a signed offset to a pointer using wrapping arithmetic.
    #[inline]
    pub fn wrapping_offset(self, count: isize) -> *const T {
        (self as usize).wrapping_add((count as usize).wrapping_mul(crate::mem::size_of::<T>())) as *const T
    }

    /// Adds a signed offset in bytes using wrapping arithmetic.
    #[inline]
    pub fn wrapping_byte_offset(self, count: isize) -> *const T {
        (self as usize).wrapping_add(count as usize) as *const T
    }

    /// Adds an unsigned offset in bytes using wrapping arithmetic.
    #[inline]
    pub fn wrapping_byte_add(self, count: usize) -> *const T {
        (self as usize).wrapping_add(count) as *const T
    }

    /// Subtracts an unsigned offset in bytes using wrapping arithmetic.
    #[inline]
    pub fn wrapping_byte_sub(self, count: usize) -> *const T {
        (self as usize).wrapping_sub(count) as *const T
    }

    /// Adds an unsigned offset in bytes to a pointer.
    #[inline]
    pub unsafe fn byte_add(self, count: usize) -> *const T {
        (self as *const u8).add(count) as *const T
    }

    /// Subtracts an unsigned offset in bytes from a pointer.
    #[inline]
    pub unsafe fn byte_sub(self, count: usize) -> *const T {
        (self as *const u8).sub(count) as *const T
    }

    /// Adds a signed offset in bytes to a pointer.
    #[inline]
    pub unsafe fn byte_offset(self, count: isize) -> *const T {
        (self as *const u8).offset(count) as *const T
    }

    /// Calculates the distance between two pointers in bytes.
    #[inline]
    pub unsafe fn byte_offset_from<U>(self, origin: *const U) -> isize {
        (self as usize as isize).wrapping_sub(origin as usize as isize)
    }

    /// Calculates the distance between two pointers, where it's known that `self >= origin`.
    #[inline]
    pub unsafe fn offset_from_unsigned(self, origin: *const T) -> usize {
        intrinsics::ptr_diff(self as *const T, origin) as usize
    }

    /// Calculates the distance between two pointers, where it's known that `self >= origin`.
    #[inline]
    pub unsafe fn sub_ptr(self, origin: *const T) -> usize {
        intrinsics::ptr_diff(self as *const T, origin) as usize
    }

    /// Creates a new pointer with the given address.
    #[inline]
    pub fn with_addr(self, addr: usize) -> *const T {
        addr as *const T
    }

    /// Creates a new pointer by mapping `self`'s address to a new one.
    #[inline]
    pub fn map_addr(self, f: impl FnOnce(usize) -> usize) -> *const T {
        f(self as usize) as *const T
    }

    /// Exposes the "provenance" part of the pointer and returns the "address" portion.
    #[inline]
    pub fn expose_provenance(self) -> usize {
        self as usize
    }

    /// Returns whether the pointer is properly aligned for `T`.
    #[inline]
    pub fn is_aligned(self) -> bool {
        (self as usize) & (crate::mem::align_of::<T>() - 1) == 0
    }

    /// Returns whether the pointer is aligned to `align`.
    #[inline]
    pub fn is_aligned_to(self, align: usize) -> bool {
        if !align.is_power_of_two() {
            crate::rt::panic_str("is_aligned_to: align is not a power-of-two", file!(), line!(), column!());
        }
        (self as usize) & (align - 1) == 0
    }

    /// Computes the offset that needs to be applied to the pointer in order to make it aligned to `align`.
    #[inline]
    pub fn align_offset(self, align: usize) -> usize {
        align_offset_impl(self as usize, crate::mem::size_of::<T>(), align)
    }

    /// Performs a volatile read of the value from `self` without moving it.
    #[inline]
    pub unsafe fn read_volatile(self) -> T {
        intrinsics::read(self as *const T)
    }

    /// Casts a pointer to an array to a pointer to a slice of the same length.
    #[inline]
    pub fn cast_array<const N: usize>(self) -> *const [T; N] {
        self as *const [T; N]
    }

    /// Subtracts an unsigned offset from a pointer using wrapping arithmetic.
    #[inline]
    pub fn wrapping_sub(self, count: usize) -> *const T {
        (self as usize).wrapping_sub(count.wrapping_mul(crate::mem::size_of::<T>())) as *const T
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
        read_unaligned(self as *const T)
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

    /// Returns a shared reference to the value behind the pointer (no null check).
    #[inline]
    pub unsafe fn as_ref_unchecked<'a>(self) -> &'a T {
        &*self
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

    /// Returns a unique reference to the value behind the pointer (no null check).
    #[inline]
    pub unsafe fn as_mut_unchecked<'a>(self) -> &'a mut T {
        &mut *self
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
        // Integer arithmetic: the C backend must not see out-of-object pointer arithmetic.
        (self as usize).wrapping_add(count.wrapping_mul(crate::mem::size_of::<T>())) as *mut T
    }

    /// Adds a signed offset to a pointer using wrapping arithmetic.
    #[inline]
    pub fn wrapping_offset(self, count: isize) -> *mut T {
        (self as usize).wrapping_add((count as usize).wrapping_mul(crate::mem::size_of::<T>())) as *mut T
    }

    /// Adds a signed offset in bytes using wrapping arithmetic.
    #[inline]
    pub fn wrapping_byte_offset(self, count: isize) -> *mut T {
        (self as usize).wrapping_add(count as usize) as *mut T
    }

    /// Adds an unsigned offset in bytes using wrapping arithmetic.
    #[inline]
    pub fn wrapping_byte_add(self, count: usize) -> *mut T {
        (self as usize).wrapping_add(count) as *mut T
    }

    /// Subtracts an unsigned offset in bytes using wrapping arithmetic.
    #[inline]
    pub fn wrapping_byte_sub(self, count: usize) -> *mut T {
        (self as usize).wrapping_sub(count) as *mut T
    }

    /// Adds an unsigned offset in bytes to a pointer.
    #[inline]
    pub unsafe fn byte_add(self, count: usize) -> *mut T {
        (self as *mut u8).add(count) as *mut T
    }

    /// Subtracts an unsigned offset in bytes from a pointer.
    #[inline]
    pub unsafe fn byte_sub(self, count: usize) -> *mut T {
        (self as *mut u8).sub(count) as *mut T
    }

    /// Adds a signed offset in bytes to a pointer.
    #[inline]
    pub unsafe fn byte_offset(self, count: isize) -> *mut T {
        (self as *mut u8).offset(count) as *mut T
    }

    /// Calculates the distance between two pointers in bytes.
    #[inline]
    pub unsafe fn byte_offset_from<U>(self, origin: *const U) -> isize {
        (self as usize as isize).wrapping_sub(origin as usize as isize)
    }

    /// Calculates the distance between two pointers, where it's known that `self >= origin`.
    #[inline]
    pub unsafe fn offset_from_unsigned(self, origin: *const T) -> usize {
        intrinsics::ptr_diff(self as *const T, origin) as usize
    }

    /// Calculates the distance between two pointers, where it's known that `self >= origin`.
    #[inline]
    pub unsafe fn sub_ptr(self, origin: *const T) -> usize {
        intrinsics::ptr_diff(self as *const T, origin) as usize
    }

    /// Creates a new pointer with the given address.
    #[inline]
    pub fn with_addr(self, addr: usize) -> *mut T {
        addr as *mut T
    }

    /// Creates a new pointer by mapping `self`'s address to a new one.
    #[inline]
    pub fn map_addr(self, f: impl FnOnce(usize) -> usize) -> *mut T {
        f(self as usize) as *mut T
    }

    /// Exposes the "provenance" part of the pointer and returns the "address" portion.
    #[inline]
    pub fn expose_provenance(self) -> usize {
        self as usize
    }

    /// Returns whether the pointer is properly aligned for `T`.
    #[inline]
    pub fn is_aligned(self) -> bool {
        (self as usize) & (crate::mem::align_of::<T>() - 1) == 0
    }

    /// Returns whether the pointer is aligned to `align`.
    #[inline]
    pub fn is_aligned_to(self, align: usize) -> bool {
        if !align.is_power_of_two() {
            crate::rt::panic_str("is_aligned_to: align is not a power-of-two", file!(), line!(), column!());
        }
        (self as usize) & (align - 1) == 0
    }

    /// Computes the offset that needs to be applied to the pointer in order to make it aligned to `align`.
    #[inline]
    pub fn align_offset(self, align: usize) -> usize {
        align_offset_impl(self as usize, crate::mem::size_of::<T>(), align)
    }

    /// Performs a volatile read of the value from `self` without moving it.
    #[inline]
    pub unsafe fn read_volatile(self) -> T {
        intrinsics::read(self as *const T)
    }

    /// Casts a pointer to an array to a pointer to a slice of the same length.
    #[inline]
    pub fn cast_array<const N: usize>(self) -> *mut [T; N] {
        self as *mut [T; N]
    }

    /// Subtracts an unsigned offset from a pointer using wrapping arithmetic.
    #[inline]
    pub fn wrapping_sub(self, count: usize) -> *mut T {
        (self as usize).wrapping_sub(count.wrapping_mul(crate::mem::size_of::<T>())) as *mut T
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

    /// Overwrites a memory location with the given value without reading or dropping the old value
    /// (no alignment requirement).
    #[inline]
    pub unsafe fn write_unaligned(self, val: T) {
        write_unaligned(self, val)
    }

    /// Reads the value from `self` without moving it (no alignment requirement).
    #[inline]
    pub unsafe fn read_unaligned(self) -> T {
        read_unaligned(self as *const T)
    }

    /// Performs a volatile write of a memory location with the given value.
    #[inline]
    pub unsafe fn write_volatile(self, val: T) {
        write_volatile(self, val)
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

    /// Creates a new pointer with the given address.
    #[inline]
    pub fn with_addr(self, addr: crate::num::NonZeroUsize) -> Self {
        NonNull { pointer: addr.get() as *const T }
    }

    /// Creates a pointer with the given address and no provenance.
    #[inline]
    pub fn without_provenance(addr: crate::num::NonZeroUsize) -> Self {
        NonNull { pointer: addr.get() as *const T }
    }

    /// Adds an offset to a pointer (in units of `T`).
    #[inline]
    pub unsafe fn add(self, count: usize) -> Self {
        NonNull { pointer: intrinsics::ptr_add(self.pointer, count) }
    }

    /// Subtracts an offset from a pointer (in units of `T`).
    #[inline]
    pub unsafe fn sub(self, count: usize) -> Self {
        NonNull { pointer: intrinsics::ptr_offset(self.pointer, -(count as isize)) }
    }

    /// Adds a signed offset to a pointer (in units of `T`).
    #[inline]
    pub unsafe fn offset(self, count: isize) -> Self {
        NonNull { pointer: intrinsics::ptr_offset(self.pointer, count) }
    }

    /// Adds an offset in bytes to a pointer.
    #[inline]
    pub unsafe fn byte_add(self, count: usize) -> Self {
        NonNull { pointer: intrinsics::ptr_add(self.pointer as *const u8, count) as *const T }
    }

    /// Subtracts an offset in bytes from a pointer.
    #[inline]
    pub unsafe fn byte_sub(self, count: usize) -> Self {
        NonNull { pointer: intrinsics::ptr_offset(self.pointer as *const u8, -(count as isize)) as *const T }
    }

    /// Calculates the distance between two pointers (in units of `T`).
    #[inline]
    pub unsafe fn offset_from(self, origin: NonNull<T>) -> isize {
        intrinsics::ptr_diff(self.pointer, origin.pointer)
    }

    /// Calculates the distance between two pointers, where it's known that `self >= origin`.
    #[inline]
    pub unsafe fn offset_from_unsigned(self, origin: NonNull<T>) -> usize {
        intrinsics::ptr_diff(self.pointer, origin.pointer) as usize
    }

    /// Calculates the distance between two pointers, where it's known that `self >= origin`.
    #[inline]
    pub unsafe fn sub_ptr(self, origin: NonNull<T>) -> usize {
        intrinsics::ptr_diff(self.pointer, origin.pointer) as usize
    }

    /// Reads the value from `self` without moving it.
    #[inline]
    pub unsafe fn read(self) -> T {
        intrinsics::read(self.pointer)
    }

    /// Reads the value from `self` without requiring alignment.
    #[inline]
    pub unsafe fn read_unaligned(self) -> T {
        read_unaligned(self.pointer)
    }

    /// Overwrites a memory location with the given value without reading or dropping the old value.
    #[inline]
    pub unsafe fn write(self, val: T) {
        intrinsics::write(self.pointer as *mut T, val)
    }

    /// Overwrites a memory location with the given value without requiring alignment.
    #[inline]
    pub unsafe fn write_unaligned(self, val: T) {
        write_unaligned(self.pointer as *mut T, val)
    }

    /// Invokes memset on the specified pointer.
    #[inline]
    pub unsafe fn write_bytes(self, val: u8, count: usize) {
        intrinsics::write_bytes(self.pointer as *mut T, val, count)
    }

    /// Replaces the value at `self` with `src`, returning the old value.
    #[inline]
    pub unsafe fn replace(self, src: T) -> T {
        replace(self.pointer as *mut T, src)
    }

    /// Swaps the values at two mutable locations of the same type.
    #[inline]
    pub unsafe fn swap(self, with: NonNull<T>) {
        swap(self.pointer as *mut T, with.pointer as *mut T)
    }

    /// Copies `count * size_of::<T>()` bytes from `self` to `dest` (may overlap).
    #[inline]
    pub unsafe fn copy_to(self, dest: NonNull<T>, count: usize) {
        intrinsics::copy(self.pointer, dest.pointer as *mut T, count)
    }

    /// Copies `count * size_of::<T>()` bytes from `self` to `dest` (must not overlap).
    #[inline]
    pub unsafe fn copy_to_nonoverlapping(self, dest: NonNull<T>, count: usize) {
        intrinsics::copy_nonoverlapping(self.pointer, dest.pointer as *mut T, count)
    }

    /// Copies `count * size_of::<T>()` bytes from `src` to `self` (may overlap).
    #[inline]
    pub unsafe fn copy_from(self, src: NonNull<T>, count: usize) {
        intrinsics::copy(src.pointer, self.pointer as *mut T, count)
    }

    /// Copies `count * size_of::<T>()` bytes from `src` to `self` (must not overlap).
    #[inline]
    pub unsafe fn copy_from_nonoverlapping(self, src: NonNull<T>, count: usize) {
        intrinsics::copy_nonoverlapping(src.pointer, self.pointer as *mut T, count)
    }

    /// Returns whether the pointer is aligned for `T`.
    #[inline]
    pub fn is_aligned(self) -> bool {
        (self.pointer as usize) % crate::mem::align_of::<T>() == 0
    }

    /// Computes the offset that needs to be applied to the pointer in order to make it aligned to `align`.
    pub fn align_offset(self, align: usize) -> usize {
        let addr = self.pointer as usize;
        let size = crate::mem::size_of::<T>();
        let rem = addr % align;
        if rem == 0 {
            return 0;
        }
        if size == 0 {
            return usize::MAX;
        }
        let bytes = align - rem;
        if bytes % size == 0 { bytes / size } else { usize::MAX }
    }

    /// Creates a non-null raw slice from a thin pointer and a length.
    #[inline]
    pub fn slice_from_raw_parts(data: NonNull<T>, len: usize) -> NonNull<[T]> {
        NonNull { pointer: unsafe { intrinsics::slice_from_raw_parts(data.pointer, len) } }
    }

    /// Returns a shared reference to the value (which may be uninitialized).
    #[inline]
    pub unsafe fn as_uninit_ref<'a>(self) -> &'a crate::mem::MaybeUninit<T> {
        &*(self.pointer as *const crate::mem::MaybeUninit<T>)
    }

    /// Returns a unique reference to the value (which may be uninitialized).
    #[inline]
    pub unsafe fn as_uninit_mut<'a>(self) -> &'a mut crate::mem::MaybeUninit<T> {
        &mut *(self.pointer as *mut crate::mem::MaybeUninit<T>)
    }
}

impl<T> NonNull<[T]> {
    /// Returns the length of a non-null raw slice.
    #[inline]
    pub fn len(self) -> usize {
        unsafe { intrinsics::slice_len(self.pointer) }
    }

    /// Returns `true` if the non-null raw slice has a length of 0.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Returns a non-null pointer to the slice's buffer.
    #[inline]
    pub fn as_non_null_ptr(self) -> NonNull<T> {
        NonNull { pointer: unsafe { intrinsics::slice_ptr(self.pointer) } }
    }

    /// Returns a raw pointer to the slice's buffer.
    #[inline]
    pub fn as_mut_ptr(self) -> *mut T {
        unsafe { intrinsics::slice_ptr(self.pointer) as *mut T }
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
        NonNull { pointer: self.pointer as *const u8 as *const U }
    }

    /// Gets the "address" portion of the pointer.
    #[inline]
    pub fn addr(self) -> crate::num::NonZeroUsize {
        unsafe { crate::num::NonZeroUsize::new_unchecked(self.pointer as *const u8 as usize) }
    }

    /// Converts from `&T` to `NonNull<T>`.
    #[inline]
    pub fn from_ref(r: &T) -> Self {
        NonNull { pointer: r as *const T }
    }

    /// Converts from `&mut T` to `NonNull<T>`.
    #[inline]
    pub fn from_mut(r: &mut T) -> Self {
        NonNull { pointer: r as *mut T as *const T }
    }

    /// Executes the destructor (if any) of the pointed-to value.
    #[inline]
    pub unsafe fn drop_in_place(self) {
        intrinsics::drop_in_place(self.pointer as *mut T)
    }
}

impl<T: ?Sized> From<NonNull<T>> for *mut T {
    #[inline]
    fn from(p: NonNull<T>) -> *mut T {
        p.pointer as *mut T
    }
}

impl<T: ?Sized> From<NonNull<T>> for *const T {
    #[inline]
    fn from(p: NonNull<T>) -> *const T {
        p.pointer
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
