//! Utilities for dynamic typing or type reflection.

use crate::fmt;
use crate::hash::{Hash, Hasher};

/// A `TypeId` represents a globally unique identifier for a type (here:
/// the type's name).
#[derive(Clone, Copy, Eq, PartialOrd, Ord)]
pub struct TypeId {
    t: &'static str,
}

impl PartialEq for TypeId {
    #[inline]
    fn eq(&self, other: &TypeId) -> bool {
        self.t == other.t
    }
}

impl Hash for TypeId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.t.hash(state)
    }
}

impl TypeId {
    /// Returns the `TypeId` of the generic type parameter.
    #[inline]
    pub fn of<T: ?Sized + 'static>() -> TypeId {
        TypeId { t: type_name::<T>() }
    }
}

impl fmt::Debug for TypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TypeId({:?})", self.t)
    }
}

/// Returns the name of a type as a string slice.
#[inline]
pub fn type_name<T: ?Sized>() -> &'static str {
    unsafe { crate::intrinsics::type_name::<T>() }
}

/// Returns the type name of the pointed-to value as a string slice.
#[inline]
pub fn type_name_of_val<T: ?Sized>(_val: &T) -> &'static str {
    type_name::<T>()
}

/// A trait to emulate dynamic typing.
pub trait Any: 'static {
    /// Gets the `TypeId` of `self`.
    fn type_id(&self) -> TypeId;
}

impl<T: 'static + ?Sized> Any for T {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

impl dyn Any {
    /// Returns `true` if the inner type is the same as `T`.
    #[inline]
    pub fn is<T: Any>(&self) -> bool {
        let t = TypeId::of::<T>();
        let concrete = self.type_id();
        t == concrete
    }

    /// Returns some reference to the inner value if it is of type `T`, or `None` if it isn't.
    #[inline]
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        if self.is::<T>() {
            unsafe { Some(&*(self as *const dyn Any as *const T)) }
        } else {
            None
        }
    }

    /// Returns some mutable reference to the inner value if it is of type `T`, or `None` if it isn't.
    #[inline]
    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        if self.is::<T>() {
            unsafe { Some(&mut *(self as *mut dyn Any as *mut T)) }
        } else {
            None
        }
    }
}

impl fmt::Debug for dyn Any {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Any")?;
        f.write_str(" { .. }")
    }
}

impl Box<dyn Any> {
    /// Attempts to downcast the box to a concrete type.
    pub fn downcast<T: Any>(self) -> Result<Box<T>, Self> {
        if self.is::<T>() {
            let raw: *mut dyn Any = Box::into_raw(self);
            Ok(unsafe { Box::from_raw(raw as *mut T) })
        } else {
            Err(self)
        }
    }
}
