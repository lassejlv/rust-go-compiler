//! Atomic types. Every operation is implemented with sequentially consistent
//! hardware atomics (stronger than or equal to any requested ordering).

use crate::cell::UnsafeCell;
use crate::fmt;
use crate::intrinsics;

/// Atomic memory orderings.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum Ordering {
    /// No ordering constraints, only atomic operations.
    Relaxed,
    /// Release ordering.
    Release,
    /// Acquire ordering.
    Acquire,
    /// Both acquire and release.
    AcqRel,
    /// Sequentially consistent.
    SeqCst,
}

#[track_caller]
#[inline]
fn check_load(order: Ordering) {
    match order {
        Ordering::Release => crate::rt::panic_str("there is no such thing as a release load", file!(), line!(), column!()),
        Ordering::AcqRel => crate::rt::panic_str("there is no such thing as an acquire-release load", file!(), line!(), column!()),
        _ => {}
    }
}

#[track_caller]
#[inline]
fn check_store(order: Ordering) {
    match order {
        Ordering::Acquire => crate::rt::panic_str("there is no such thing as an acquire store", file!(), line!(), column!()),
        Ordering::AcqRel => crate::rt::panic_str("there is no such thing as an acquire-release store", file!(), line!(), column!()),
        _ => {}
    }
}

#[track_caller]
#[inline]
fn check_failure(order: Ordering) {
    match order {
        Ordering::Release => crate::rt::panic_str("there is no such thing as a release failure ordering", file!(), line!(), column!()),
        Ordering::AcqRel => crate::rt::panic_str("there is no such thing as an acquire-release failure ordering", file!(), line!(), column!()),
        _ => {}
    }
}

/// An atomic fence.
#[inline]
#[track_caller]
pub fn fence(order: Ordering) {
    if let Ordering::Relaxed = order {
        crate::rt::panic_str("there is no such thing as a relaxed fence", file!(), line!(), column!());
    }
    unsafe { intrinsics::atomic_fence() }
}

/// A compiler memory fence.
#[inline]
#[track_caller]
pub fn compiler_fence(order: Ordering) {
    if let Ordering::Relaxed = order {
        crate::rt::panic_str("there is no such thing as a relaxed fence", file!(), line!(), column!());
    }
    unsafe { intrinsics::atomic_fence() }
}

/// Signals the processor that it is inside a busy-wait spin-loop.
#[inline]
pub fn spin_loop_hint() {}

macro_rules! __std_atomic_common {
    ($name:ident, $t:ty) => {
        impl $name {
            /// Creates a new atomic value.
            #[inline]
            pub const fn new(v: $t) -> $name {
                $name { v: UnsafeCell::new(v) }
            }
            /// Returns a mutable reference to the underlying value.
            #[inline]
            pub fn get_mut(&mut self) -> &mut $t {
                self.v.get_mut()
            }
            /// Consumes the atomic and returns the contained value.
            #[inline]
            pub fn into_inner(self) -> $t {
                self.v.into_inner()
            }
            /// Returns a mutable pointer to the underlying value.
            #[inline]
            pub const fn as_ptr(&self) -> *mut $t {
                self.v.get()
            }
            /// Loads a value from the atomic.
            #[inline]
            #[track_caller]
            pub fn load(&self, order: Ordering) -> $t {
                check_load(order);
                unsafe { intrinsics::atomic_load(self.v.get() as *const $t) }
            }
            /// Stores a value into the atomic.
            #[inline]
            #[track_caller]
            pub fn store(&self, val: $t, order: Ordering) {
                check_store(order);
                unsafe { intrinsics::atomic_store(self.v.get(), val) }
            }
            /// Stores a value into the atomic, returning the previous value.
            #[inline]
            pub fn swap(&self, val: $t, _order: Ordering) -> $t {
                unsafe { intrinsics::atomic_xchg(self.v.get(), val) }
            }
            /// Stores a value if the current value is the same as `current` (deprecated).
            #[inline]
            pub fn compare_and_swap(&self, current: $t, new: $t, _order: Ordering) -> $t {
                match self.compare_exchange(current, new, Ordering::SeqCst, Ordering::SeqCst) {
                    Ok(v) => v,
                    Err(v) => v,
                }
            }
            /// Stores a value if the current value is the same as `current`.
            #[inline]
            #[track_caller]
            pub fn compare_exchange(&self, current: $t, new: $t, _success: Ordering, failure: Ordering) -> Result<$t, $t> {
                check_failure(failure);
                let (old, ok) = unsafe { intrinsics::atomic_cxchg(self.v.get(), current, new) };
                if ok { Ok(old) } else { Err(old) }
            }
            /// Stores a value if the current value is the same as `current` (may fail spuriously).
            #[inline]
            #[track_caller]
            pub fn compare_exchange_weak(&self, current: $t, new: $t, _success: Ordering, failure: Ordering) -> Result<$t, $t> {
                check_failure(failure);
                let (old, ok) = unsafe { intrinsics::atomic_cxchg_weak(self.v.get(), current, new) };
                if ok { Ok(old) } else { Err(old) }
            }
            /// Fetches the value, and applies a function to it that returns an optional new value.
            pub fn fetch_update<F>(&self, _set_order: Ordering, _fetch_order: Ordering, f: F) -> Result<$t, $t>
            where
                F: FnMut($t) -> Option<$t>,
            {
                let mut f = f;
                let mut prev = self.load(Ordering::SeqCst);
                loop {
                    match f(prev) {
                        Some(next) => match self.compare_exchange_weak(prev, next, Ordering::SeqCst, Ordering::SeqCst) {
                            Ok(x) => return Ok(x),
                            Err(next_prev) => prev = next_prev,
                        },
                        None => return Err(prev),
                    }
                }
            }
        }

        impl Default for $name {
            fn default() -> $name {
                $name::new(Default::default())
            }
        }

        impl From<$t> for $name {
            fn from(v: $t) -> $name {
                $name::new(v)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(&self.load(Ordering::SeqCst), f)
            }
        }

    };
}

macro_rules! __std_atomic_int {
    ($($name:ident: $t:ty),* $(,)?) => {
        $(
            /// An integer type which can be safely shared between threads.
            #[repr(transparent)]
            pub struct $name {
                v: UnsafeCell<$t>,
            }

            __std_atomic_common!($name, $t);

            impl $name {
                /// Adds to the current value, returning the previous value.
                #[inline]
                pub fn fetch_add(&self, val: $t, _order: Ordering) -> $t {
                    unsafe { intrinsics::atomic_xadd(self.v.get(), val) }
                }
                /// Subtracts from the current value, returning the previous value.
                #[inline]
                pub fn fetch_sub(&self, val: $t, _order: Ordering) -> $t {
                    unsafe { intrinsics::atomic_xsub(self.v.get(), val) }
                }
                /// Bitwise "and" with the current value, returning the previous value.
                #[inline]
                pub fn fetch_and(&self, val: $t, _order: Ordering) -> $t {
                    unsafe { intrinsics::atomic_and(self.v.get(), val) }
                }
                /// Bitwise "nand" with the current value, returning the previous value.
                #[inline]
                pub fn fetch_nand(&self, val: $t, _order: Ordering) -> $t {
                    unsafe { intrinsics::atomic_nand(self.v.get(), val) }
                }
                /// Bitwise "or" with the current value, returning the previous value.
                #[inline]
                pub fn fetch_or(&self, val: $t, _order: Ordering) -> $t {
                    unsafe { intrinsics::atomic_or(self.v.get(), val) }
                }
                /// Bitwise "xor" with the current value, returning the previous value.
                #[inline]
                pub fn fetch_xor(&self, val: $t, _order: Ordering) -> $t {
                    unsafe { intrinsics::atomic_xor(self.v.get(), val) }
                }
                /// Maximum with the current value, returning the previous value.
                #[inline]
                pub fn fetch_max(&self, val: $t, _order: Ordering) -> $t {
                    unsafe { intrinsics::atomic_max(self.v.get(), val) }
                }
                /// Minimum with the current value, returning the previous value.
                #[inline]
                pub fn fetch_min(&self, val: $t, _order: Ordering) -> $t {
                    unsafe { intrinsics::atomic_min(self.v.get(), val) }
                }
            }
        )*
    };
}

__std_atomic_int! {
    AtomicI8: i8, AtomicI16: i16, AtomicI32: i32, AtomicI64: i64, AtomicIsize: isize,
    AtomicU8: u8, AtomicU16: u16, AtomicU32: u32, AtomicU64: u64, AtomicUsize: usize,
}

/// A boolean type which can be safely shared between threads.
#[repr(transparent)]
pub struct AtomicBool {
    v: UnsafeCell<bool>,
}

__std_atomic_common!(AtomicBool, bool);

impl AtomicBool {
    #[inline]
    fn as_u8(&self) -> *mut u8 {
        self.v.get() as *mut u8
    }
    /// Logical "and" with a boolean value, returning the previous value.
    #[inline]
    pub fn fetch_and(&self, val: bool, _order: Ordering) -> bool {
        unsafe { intrinsics::atomic_and(self.as_u8(), val as u8) != 0 }
    }
    /// Logical "nand" with a boolean value, returning the previous value.
    #[inline]
    pub fn fetch_nand(&self, val: bool, order: Ordering) -> bool {
        // !(x & val) for booleans: keep the stored value 0 or 1.
        match self.fetch_update(order, Ordering::SeqCst, |x| Some(!(x & val))) {
            Ok(v) => v,
            Err(v) => v,
        }
    }
    /// Logical "or" with a boolean value, returning the previous value.
    #[inline]
    pub fn fetch_or(&self, val: bool, _order: Ordering) -> bool {
        unsafe { intrinsics::atomic_or(self.as_u8(), val as u8) != 0 }
    }
    /// Logical "xor" with a boolean value, returning the previous value.
    #[inline]
    pub fn fetch_xor(&self, val: bool, _order: Ordering) -> bool {
        unsafe { intrinsics::atomic_xor(self.as_u8(), val as u8) != 0 }
    }
    /// Logical "not" of the current value, returning the previous value.
    #[inline]
    pub fn fetch_not(&self, order: Ordering) -> bool {
        self.fetch_xor(true, order)
    }
}

/// A raw pointer type which can be safely shared between threads.
#[repr(transparent)]
pub struct AtomicPtr<T> {
    p: UnsafeCell<*mut T>,
}


impl<T> AtomicPtr<T> {
    /// Creates a new `AtomicPtr`.
    #[inline]
    pub const fn new(p: *mut T) -> AtomicPtr<T> {
        AtomicPtr { p: UnsafeCell::new(p) }
    }
    /// Returns a mutable reference to the underlying pointer.
    #[inline]
    pub fn get_mut(&mut self) -> &mut *mut T {
        self.p.get_mut()
    }
    /// Consumes the atomic and returns the contained value.
    #[inline]
    pub fn into_inner(self) -> *mut T {
        self.p.into_inner()
    }
    /// Returns a mutable pointer to the underlying pointer.
    #[inline]
    pub const fn as_ptr(&self) -> *mut *mut T {
        self.p.get()
    }
    /// Loads a value from the pointer.
    #[inline]
    #[track_caller]
    pub fn load(&self, order: Ordering) -> *mut T {
        check_load(order);
        unsafe { intrinsics::atomic_load(self.p.get() as *const *mut T) }
    }
    /// Stores a value into the pointer.
    #[inline]
    #[track_caller]
    pub fn store(&self, ptr: *mut T, order: Ordering) {
        check_store(order);
        unsafe { intrinsics::atomic_store(self.p.get(), ptr) }
    }
    /// Stores a value into the pointer, returning the previous value.
    #[inline]
    pub fn swap(&self, ptr: *mut T, _order: Ordering) -> *mut T {
        unsafe { intrinsics::atomic_xchg(self.p.get(), ptr) }
    }
    /// Stores a value into the pointer if the current value is the same as `current`.
    #[inline]
    #[track_caller]
    pub fn compare_exchange(&self, current: *mut T, new: *mut T, _success: Ordering, failure: Ordering) -> Result<*mut T, *mut T> {
        check_failure(failure);
        let (old, ok) = unsafe { intrinsics::atomic_cxchg(self.p.get(), current, new) };
        if ok { Ok(old) } else { Err(old) }
    }
    /// Like `compare_exchange`, but may fail spuriously.
    #[inline]
    #[track_caller]
    pub fn compare_exchange_weak(&self, current: *mut T, new: *mut T, success: Ordering, failure: Ordering) -> Result<*mut T, *mut T> {
        self.compare_exchange(current, new, success, failure)
    }
    /// Fetches the value, and applies a function to it that returns an optional new value.
    pub fn fetch_update<F>(&self, _set_order: Ordering, _fetch_order: Ordering, f: F) -> Result<*mut T, *mut T>
    where
        F: FnMut(*mut T) -> Option<*mut T>,
    {
        let mut f = f;
        let mut prev = self.load(Ordering::SeqCst);
        loop {
            match f(prev) {
                Some(next) => match self.compare_exchange(prev, next, Ordering::SeqCst, Ordering::SeqCst) {
                    Ok(x) => return Ok(x),
                    Err(next_prev) => prev = next_prev,
                },
                None => return Err(prev),
            }
        }
    }
}

impl<T> Default for AtomicPtr<T> {
    fn default() -> AtomicPtr<T> {
        AtomicPtr::new(crate::ptr::null_mut())
    }
}

impl<T> From<*mut T> for AtomicPtr<T> {
    fn from(p: *mut T) -> AtomicPtr<T> {
        AtomicPtr::new(p)
    }
}

impl<T> fmt::Debug for AtomicPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.load(Ordering::SeqCst), f)
    }
}

/// An `AtomicBool` initialized to `false` (deprecated).
pub const ATOMIC_BOOL_INIT: AtomicBool = AtomicBool::new(false);
/// An `AtomicUsize` initialized to `0` (deprecated).
pub const ATOMIC_USIZE_INIT: AtomicUsize = AtomicUsize::new(0);
/// An `AtomicIsize` initialized to `0` (deprecated).
pub const ATOMIC_ISIZE_INIT: AtomicIsize = AtomicIsize::new(0);
