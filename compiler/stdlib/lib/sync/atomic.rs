//! Atomic types (single-threaded implementation).

use crate::cell::UnsafeCell;
use crate::fmt;

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

/// An atomic fence (no-op).
#[inline]
pub fn fence(_order: Ordering) {}

/// A compiler memory fence (no-op).
#[inline]
pub fn compiler_fence(_order: Ordering) {}

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
            /// Loads a value from the atomic.
            #[inline]
            pub fn load(&self, _order: Ordering) -> $t {
                unsafe { *self.v.get() }
            }
            /// Stores a value into the atomic.
            #[inline]
            pub fn store(&self, val: $t, _order: Ordering) {
                unsafe {
                    *self.v.get() = val;
                }
            }
            /// Stores a value into the atomic, returning the previous value.
            #[inline]
            pub fn swap(&self, val: $t, _order: Ordering) -> $t {
                unsafe { crate::mem::replace(&mut *self.v.get(), val) }
            }
            /// Stores a value if the current value is the same as `current`.
            #[inline]
            pub fn compare_exchange(&self, current: $t, new: $t, _success: Ordering, _failure: Ordering) -> Result<$t, $t> {
                let old = self.load(Ordering::SeqCst);
                if old == current {
                    self.store(new, Ordering::SeqCst);
                    Ok(old)
                } else {
                    Err(old)
                }
            }
            /// Stores a value if the current value is the same as `current` (may fail spuriously).
            #[inline]
            pub fn compare_exchange_weak(&self, current: $t, new: $t, success: Ordering, failure: Ordering) -> Result<$t, $t> {
                self.compare_exchange(current, new, success, failure)
            }
            /// Fetches the value, and applies a function to it that returns an optional new value.
            pub fn fetch_update<F>(&self, _set_order: Ordering, _fetch_order: Ordering, f: F) -> Result<$t, $t>
            where
                F: FnMut($t) -> Option<$t>,
            {
                let mut f = f;
                let prev = self.load(Ordering::SeqCst);
                match f(prev) {
                    Some(next) => {
                        self.store(next, Ordering::SeqCst);
                        Ok(prev)
                    }
                    None => Err(prev),
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
                    let old = self.load(Ordering::SeqCst);
                    self.store(old.wrapping_add(val), Ordering::SeqCst);
                    old
                }
                /// Subtracts from the current value, returning the previous value.
                #[inline]
                pub fn fetch_sub(&self, val: $t, _order: Ordering) -> $t {
                    let old = self.load(Ordering::SeqCst);
                    self.store(old.wrapping_sub(val), Ordering::SeqCst);
                    old
                }
                /// Bitwise "and" with the current value, returning the previous value.
                #[inline]
                pub fn fetch_and(&self, val: $t, _order: Ordering) -> $t {
                    let old = self.load(Ordering::SeqCst);
                    self.store(old & val, Ordering::SeqCst);
                    old
                }
                /// Bitwise "or" with the current value, returning the previous value.
                #[inline]
                pub fn fetch_or(&self, val: $t, _order: Ordering) -> $t {
                    let old = self.load(Ordering::SeqCst);
                    self.store(old | val, Ordering::SeqCst);
                    old
                }
                /// Bitwise "xor" with the current value, returning the previous value.
                #[inline]
                pub fn fetch_xor(&self, val: $t, _order: Ordering) -> $t {
                    let old = self.load(Ordering::SeqCst);
                    self.store(old ^ val, Ordering::SeqCst);
                    old
                }
                /// Maximum with the current value, returning the previous value.
                #[inline]
                pub fn fetch_max(&self, val: $t, _order: Ordering) -> $t {
                    let old = self.load(Ordering::SeqCst);
                    if val > old {
                        self.store(val, Ordering::SeqCst);
                    }
                    old
                }
                /// Minimum with the current value, returning the previous value.
                #[inline]
                pub fn fetch_min(&self, val: $t, _order: Ordering) -> $t {
                    let old = self.load(Ordering::SeqCst);
                    if val < old {
                        self.store(val, Ordering::SeqCst);
                    }
                    old
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
    /// Logical "and" with a boolean value, returning the previous value.
    #[inline]
    pub fn fetch_and(&self, val: bool, _order: Ordering) -> bool {
        let old = self.load(Ordering::SeqCst);
        self.store(old & val, Ordering::SeqCst);
        old
    }
    /// Logical "or" with a boolean value, returning the previous value.
    #[inline]
    pub fn fetch_or(&self, val: bool, _order: Ordering) -> bool {
        let old = self.load(Ordering::SeqCst);
        self.store(old | val, Ordering::SeqCst);
        old
    }
    /// Logical "xor" with a boolean value, returning the previous value.
    #[inline]
    pub fn fetch_xor(&self, val: bool, _order: Ordering) -> bool {
        let old = self.load(Ordering::SeqCst);
        self.store(old ^ val, Ordering::SeqCst);
        old
    }
    /// Logical "not" of the current value, returning the previous value.
    #[inline]
    pub fn fetch_not(&self, _order: Ordering) -> bool {
        let old = self.load(Ordering::SeqCst);
        self.store(!old, Ordering::SeqCst);
        old
    }
}
