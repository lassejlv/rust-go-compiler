//! Memory allocation APIs.

use crate::fmt;
use crate::intrinsics;

/// Layout of a block of memory.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Layout {
    size: usize,
    align: usize,
}

/// The parameters given to `Layout::from_size_align` do not satisfy its constraints.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LayoutError;

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
}

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
