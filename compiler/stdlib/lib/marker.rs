//! Primitive traits and types representing basic properties of types.

use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};

/// Types with a constant size known at compile time.
#[lang = "sized"]
pub trait Sized {}

/// Types whose values can be duplicated simply by copying bits.
#[lang = "copy"]
pub trait Copy: Clone {}

/// Types that can be transferred across thread boundaries.
pub unsafe auto trait Send {}

/// Types for which it is safe to share references between threads.
#[lang = "sync"]
pub unsafe auto trait Sync {}

/// Types that do not require pinning.
pub auto trait Unpin {}

/// The principal of a trait object with only auto traits (`dyn Send`,
/// `dyn Send + Sync`): implemented by every type, it has no methods.
#[lang = "any_trait"]
#[doc(hidden)]
pub trait __AutoOnlyObject {}

/// Tuple types (used by the `Fn*` traits).
#[lang = "tuple_trait"]
pub trait Tuple {}

/// Function pointer types, of any safety and ABI (implemented by the
/// compiler for every fn pointer type; core's unstable `FnPtr`).
#[lang = "fn_ptr_trait"]
pub trait FnPtr: Copy + Clone {
    /// The address of the function.
    fn addr(self) -> *const ();
}

impl<F: FnPtr> PartialEq for F {
    fn eq(&self, other: &Self) -> bool {
        self.addr() == other.addr()
    }
}

impl<F: FnPtr> Eq for F {}

impl<F: FnPtr> PartialOrd for F {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.addr().partial_cmp(&other.addr())
    }
}

impl<F: FnPtr> Ord for F {
    fn cmp(&self, other: &Self) -> Ordering {
        self.addr().cmp(&other.addr())
    }
}

impl<F: FnPtr> Hash for F {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.addr() as usize);
    }
}

impl<F: FnPtr> fmt::Pointer for F {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::pointer_fmt_inner(self.addr() as usize, f)
    }
}

impl<F: FnPtr> fmt::Debug for F {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::pointer_fmt_inner(self.addr() as usize, f)
    }
}

/// Zero-sized type used to mark things that "act like" they own a `T`.
#[lang = "phantom_data"]
pub struct PhantomData<T: ?Sized>;

impl<T: ?Sized> Clone for PhantomData<T> {
    fn clone(&self) -> Self {
        PhantomData
    }
}

impl<T: ?Sized> Copy for PhantomData<T> {}

impl<T: ?Sized> Default for PhantomData<T> {
    fn default() -> Self {
        PhantomData
    }
}

impl<T: ?Sized> PartialEq for PhantomData<T> {
    fn eq(&self, _other: &PhantomData<T>) -> bool {
        true
    }
}

impl<T: ?Sized> Eq for PhantomData<T> {}

impl<T: ?Sized> PartialOrd for PhantomData<T> {
    fn partial_cmp(&self, _other: &PhantomData<T>) -> Option<Ordering> {
        Some(Ordering::Equal)
    }
}

impl<T: ?Sized> Ord for PhantomData<T> {
    fn cmp(&self, _other: &PhantomData<T>) -> Ordering {
        Ordering::Equal
    }
}

impl<T: ?Sized> Hash for PhantomData<T> {
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}

impl<T: ?Sized> fmt::Debug for PhantomData<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PhantomData<")?;
        f.write_str(crate::any::type_name::<T>())?;
        f.write_str(">")
    }
}

/// Marker for types that can be pinned and moved (always implemented).
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PhantomPinned;
