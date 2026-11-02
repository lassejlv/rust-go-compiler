//! Memory allocation APIs.

use crate::fmt;
use crate::intrinsics;

/// Layout of a block of memory.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Layout {
    size: usize,
    align: usize,
}

impl fmt::Debug for Layout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Align(usize);
        impl fmt::Debug for Align {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{} (1 << {})", self.0, self.0.trailing_zeros())
            }
        }
        f.debug_struct("Layout").field("size", &self.size).field("align", &Align(self.align)).finish()
    }
}

/// The error type returned by `Layout` constructors (alias kept for compatibility).
pub type LayoutErr = LayoutError;

/// The `AllocError` error indicates an allocation failure.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct AllocError;

impl fmt::Display for AllocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("memory allocation failed")
    }
}

impl crate::error::Error for AllocError {}

/// The parameters given to `Layout::from_size_align` do not satisfy its constraints.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LayoutError;

impl crate::error::Error for LayoutError {}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid parameters to Layout::from_size_align")
    }
}

impl Layout {
    /// Constructs a `Layout` from a given `size` and `align`.
    pub fn from_size_align(size: usize, align: usize) -> Result<Layout, LayoutError> {
        if align == 0 || (align & (align - 1)) != 0 {
            return Err(LayoutError);
        }
        if size > (isize::MAX as usize) - (align - 1) {
            return Err(LayoutError);
        }
        Ok(Layout { size, align })
    }

    /// Creates a layout, bypassing all checks.
    #[inline]
    pub const unsafe fn from_size_align_unchecked(size: usize, align: usize) -> Layout {
        Layout { size, align }
    }

    /// The minimum size in bytes for a memory block of this layout.
    #[inline]
    pub const fn size(&self) -> usize {
        self.size
    }

    /// The minimum byte alignment for a memory block of this layout.
    #[inline]
    pub const fn align(&self) -> usize {
        self.align
    }

    /// Constructs a `Layout` suitable for holding a value of type `T`.
    #[inline]
    pub fn new<T>() -> Layout {
        Layout { size: crate::mem::size_of::<T>(), align: crate::mem::align_of::<T>() }
    }

    /// Produces layout describing a record that could be used to allocate backing structure for `T`.
    #[inline]
    pub fn for_value<T: ?Sized>(t: &T) -> Layout {
        Layout { size: crate::mem::size_of_val(t), align: crate::mem::align_of_val(t) }
    }

    /// Creates a layout describing the record for a `[T; n]`.
    pub fn array<T>(n: usize) -> Result<Layout, LayoutError> {
        let size = crate::mem::size_of::<T>();
        match size.checked_mul(n) {
            Some(total) => Layout::from_size_align(total, crate::mem::align_of::<T>()),
            None => Err(LayoutError),
        }
    }

    /// Creates a layout describing the record that can hold a value of the same layout as `self`, but that also is aligned to alignment `align`.
    pub fn align_to(&self, align: usize) -> Result<Layout, LayoutError> {
        let a = if align > self.align { align } else { self.align };
        Layout::from_size_align(self.size, a)
    }

    /// Creates a layout by rounding the size of this layout up to a multiple of the layout's alignment.
    pub fn pad_to_align(&self) -> Layout {
        let new_size = (self.size + self.align - 1) & !(self.align - 1);
        Layout { size: new_size, align: self.align }
    }

    /// Returns the amount of padding we must insert after `self` to ensure
    /// that the following address will satisfy `align`.
    pub const fn padding_needed_for(&self, align: usize) -> usize {
        let len = self.size;
        let len_rounded_up = (len + align - 1) & !(align - 1);
        len_rounded_up - len
    }

    /// Creates a layout describing the record for `self` followed by `next`,
    /// including any necessary padding; returns the offset of `next`.
    pub fn extend(&self, next: Layout) -> Result<(Layout, usize), LayoutError> {
        let new_align = if self.align > next.align { self.align } else { next.align };
        let pad = self.padding_needed_for(next.align);
        let offset = match self.size.checked_add(pad) {
            Some(o) => o,
            None => return Err(LayoutError),
        };
        let new_size = match offset.checked_add(next.size) {
            Some(s) => s,
            None => return Err(LayoutError),
        };
        let layout = Layout::from_size_align(new_size, new_align)?;
        Ok((layout, offset))
    }

    /// Creates a layout describing the record for `n` instances of `self`,
    /// with padding between each; returns the distance between elements.
    pub fn repeat(&self, n: usize) -> Result<(Layout, usize), LayoutError> {
        let padded = self.pad_to_align();
        match padded.size.checked_mul(n) {
            Some(size) => Ok((Layout::from_size_align(size, self.align)?, padded.size)),
            None => Err(LayoutError),
        }
    }

    /// Creates a layout describing the record for `self` followed by `next` with no padding.
    pub fn extend_packed(&self, next: Layout) -> Result<Layout, LayoutError> {
        match self.size.checked_add(next.size) {
            Some(new_size) => Layout::from_size_align(new_size, self.align),
            None => Err(LayoutError),
        }
    }

    /// Creates a layout describing the record for `n` instances of `self`, with no padding between each instance.
    pub fn repeat_packed(&self, n: usize) -> Result<Layout, LayoutError> {
        match self.size.checked_mul(n) {
            Some(size) => Layout::from_size_align(size, self.align),
            None => Err(LayoutError),
        }
    }

    /// Creates a `NonNull` that is dangling, but well-aligned for this Layout.
    pub fn dangling(&self) -> crate::ptr::NonNull<u8> {
        unsafe { crate::ptr::NonNull::new_unchecked(self.align as *mut u8) }
    }

    /// Produces layout describing a record that could be used to allocate backing structure for `T` (raw pointer version).
    pub unsafe fn for_value_raw<T: ?Sized>(t: *const T) -> Layout {
        Layout { size: intrinsics::size_of_val(t), align: intrinsics::align_of_val(t) }
    }
}

/// A memory allocator that can be registered as the standard library's default
/// through the `#[global_allocator]` attribute.
pub unsafe trait GlobalAlloc {
    /// Allocates memory as described by the given `layout`.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8;

    /// Deallocates the block of memory at the given `ptr` pointer with the given `layout`.
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout);

    /// Behaves like `alloc`, but also ensures that the contents are set to zero before being returned.
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let ptr = self.alloc(layout);
        if !ptr.is_null() {
            intrinsics::write_bytes(ptr, 0, size);
        }
        ptr
    }

    /// Shrinks or grows a block of memory to the given `new_size` in bytes.
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
        let new_ptr = self.alloc(new_layout);
        if !new_ptr.is_null() {
            let n = if layout.size() < new_size { layout.size() } else { new_size };
            intrinsics::copy_nonoverlapping(ptr as *const u8, new_ptr, n);
            self.dealloc(ptr, layout);
        }
        new_ptr
    }
}

/// The default memory allocator provided by the operating system.
#[derive(Debug, Default, Copy, Clone)]
pub struct System;

unsafe impl GlobalAlloc for System {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        alloc(layout)
    }
    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        alloc_zeroed(layout)
    }
    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        dealloc(ptr, layout)
    }
    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        realloc(ptr, layout, new_size)
    }
}

/// The global memory allocator (forwards to `System`).
#[derive(Copy, Clone, Default, Debug)]
pub struct Global;

/// Sets the allocation error hook (accepted and ignored).
pub fn set_alloc_error_hook(_hook: fn(Layout)) {}

/// Allocates memory as described by the given `layout`. Zero-sized
/// layouts return a dangling, well-aligned pointer.
#[inline]
pub unsafe fn alloc(layout: Layout) -> *mut u8 {
    if layout.size == 0 {
        return layout.align as *mut u8;
    }
    intrinsics::alloc(layout.size, layout.align)
}

/// Allocates zero-initialized memory as described by the given `layout`.
#[inline]
pub unsafe fn alloc_zeroed(layout: Layout) -> *mut u8 {
    let p = alloc(layout);
    if layout.size != 0 {
        intrinsics::write_bytes(p, 0, layout.size);
    }
    p
}

/// Deallocates memory with the given `layout`.
#[inline]
pub unsafe fn dealloc(ptr: *mut u8, layout: Layout) {
    if layout.size != 0 {
        intrinsics::dealloc(ptr, layout.size, layout.align)
    }
}

/// Reallocates memory to the given `new_size`.
#[inline]
pub unsafe fn realloc(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
    if layout.size == 0 {
        return alloc(Layout { size: new_size, align: layout.align });
    }
    if new_size == 0 {
        intrinsics::dealloc(ptr, layout.size, layout.align);
        return layout.align as *mut u8;
    }
    intrinsics::realloc(ptr, layout.size, layout.align, new_size)
}

/// Signals a memory allocation error.
pub fn handle_alloc_error(layout: Layout) -> ! {
    crate::rt::panic_fmt(format_args!("memory allocation of {} bytes failed", layout.size), file!(), line!(), column!())
}

// Internal helpers used by the collections: allocate/free arrays of `T`
// (never calling the allocator for zero-sized requests).

/// Allocates room for `n` values of `T`. Returns a dangling pointer when the
/// total size is zero.
#[inline]
pub(crate) unsafe fn alloc_array<T>(n: usize) -> *mut T {
    let size = crate::mem::size_of::<T>();
    let align = crate::mem::align_of::<T>();
    if size == 0 || n == 0 {
        return align as *mut T;
    }
    let total = match size.checked_mul(n) {
        Some(t) => t,
        None => capacity_overflow(),
    };
    intrinsics::alloc(total, align) as *mut T
}

/// Frees an array allocated by `alloc_array` (or `realloc_array`).
#[inline]
pub(crate) unsafe fn dealloc_array<T>(p: *mut T, n: usize) {
    let size = crate::mem::size_of::<T>();
    if size == 0 || n == 0 {
        return;
    }
    intrinsics::dealloc(p as *mut u8, size * n, crate::mem::align_of::<T>())
}

/// Grows or shrinks an array allocation from `old_n` to `new_n` elements.
#[inline]
pub(crate) unsafe fn realloc_array<T>(p: *mut T, old_n: usize, new_n: usize) -> *mut T {
    let size = crate::mem::size_of::<T>();
    let align = crate::mem::align_of::<T>();
    if size == 0 {
        return align as *mut T;
    }
    if old_n == 0 {
        return alloc_array::<T>(new_n);
    }
    if new_n == 0 {
        dealloc_array(p, old_n);
        return align as *mut T;
    }
    let total = match size.checked_mul(new_n) {
        Some(t) => t,
        None => capacity_overflow(),
    };
    intrinsics::realloc(p as *mut u8, size * old_n, align, total) as *mut T
}

#[inline(never)]
pub(crate) fn capacity_overflow() -> ! {
    crate::rt::panic_str("capacity overflow", file!(), line!(), column!())
}
