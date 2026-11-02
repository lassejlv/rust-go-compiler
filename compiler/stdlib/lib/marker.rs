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

/// Tuple types (used by the `Fn*` traits).
#[lang = "tuple_trait"]
pub trait Tuple {}

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
