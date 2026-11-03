//! Interfaces for working with errors.

use crate::fmt::{Debug, Display};

/// `Error` is a trait representing the basic expectations for error values.
pub trait Error: Debug + Display {
    /// Returns the lower-level source of this error, if any.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    /// Deprecated: use the `Display` impl or `to_string()`.
    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    /// Deprecated: replaced by `Error::source`.
    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }

    /// Gets the `TypeId` of `self` (used for downcasting).
    #[doc(hidden)]
    fn type_id(&self, _: private::Internal) -> crate::any::TypeId
    where
        Self: 'static,
    {
        crate::any::TypeId::of::<Self>()
    }
}

mod private {
    /// A type that cannot be named outside std (seals `Error::type_id`).
    #[derive(Debug)]
    pub struct Internal;
}

impl dyn Error {
    /// Returns `true` if the inner type is the same as `T`.
    #[inline]
    pub fn is<T: Error + 'static>(&self) -> bool {
        crate::any::TypeId::of::<T>() == self.type_id(private::Internal)
    }

    /// Returns some reference to the inner value if it is of type `T`, or `None` if it isn't.
    #[inline]
    pub fn downcast_ref<T: Error + 'static>(&self) -> Option<&T> {
        if self.is::<T>() {
            unsafe { Some(&*(self as *const dyn Error as *const T)) }
        } else {
            None
        }
    }

    /// Returns some mutable reference to the inner value if it is of type `T`, or `None` if it isn't.
    #[inline]
    pub fn downcast_mut<T: Error + 'static>(&mut self) -> Option<&mut T> {
        if self.is::<T>() {
            unsafe { Some(&mut *(self as *mut dyn Error as *mut T)) }
        } else {
            None
        }
    }
}

impl (dyn Error + Send) {
    /// Returns `true` if the inner type is the same as `T`.
    #[inline]
    pub fn is<T: Error + 'static>(&self) -> bool {
        crate::any::TypeId::of::<T>() == self.type_id(private::Internal)
    }

    /// Returns some reference to the inner value if it is of type `T`, or `None` if it isn't.
    #[inline]
    pub fn downcast_ref<T: Error + 'static>(&self) -> Option<&T> {
        if self.is::<T>() {
            unsafe { Some(&*(self as *const (dyn Error + Send) as *const T)) }
        } else {
            None
        }
    }

    /// Returns some mutable reference to the inner value if it is of type `T`, or `None` if it isn't.
    #[inline]
    pub fn downcast_mut<T: Error + 'static>(&mut self) -> Option<&mut T> {
        if self.is::<T>() {
            unsafe { Some(&mut *(self as *mut (dyn Error + Send) as *mut T)) }
        } else {
            None
        }
    }
}

impl Box<dyn Error> {
    /// Attempts to downcast the box to a concrete type.
    pub fn downcast<T: Error + 'static>(self) -> Result<Box<T>, Box<dyn Error>> {
        if self.is::<T>() {
            let raw: *mut dyn Error = Box::into_raw(self);
            Ok(unsafe { Box::from_raw(raw as *mut T) })
        } else {
            Err(self)
        }
    }
}

impl Box<dyn Error + Send> {
    /// Attempts to downcast the box to a concrete type.
    pub fn downcast<T: Error + 'static>(self) -> Result<Box<T>, Box<dyn Error + Send>> {
        if self.is::<T>() {
            let raw: *mut (dyn Error + Send) = Box::into_raw(self);
            Ok(unsafe { Box::from_raw(raw as *mut T) })
        } else {
            Err(self)
        }
    }
}

impl (dyn Error + Send + Sync) {
    /// Returns `true` if the inner type is the same as `T`.
    #[inline]
    pub fn is<T: Error + 'static>(&self) -> bool {
        crate::any::TypeId::of::<T>() == self.type_id(private::Internal)
    }

    /// Returns some reference to the inner value if it is of type `T`, or `None` if it isn't.
    #[inline]
    pub fn downcast_ref<T: Error + 'static>(&self) -> Option<&T> {
        if self.is::<T>() {
            unsafe { Some(&*(self as *const (dyn Error + Send + Sync) as *const T)) }
        } else {
            None
        }
    }

    /// Returns some mutable reference to the inner value if it is of type `T`, or `None` if it isn't.
    #[inline]
    pub fn downcast_mut<T: Error + 'static>(&mut self) -> Option<&mut T> {
        if self.is::<T>() {
            unsafe { Some(&mut *(self as *mut (dyn Error + Send + Sync) as *mut T)) }
        } else {
            None
        }
    }
}

impl Box<dyn Error + Send + Sync> {
    /// Attempts to downcast the box to a concrete type.
    pub fn downcast<T: Error + 'static>(self) -> Result<Box<T>, Box<dyn Error + Send + Sync>> {
        if self.is::<T>() {
            let raw: *mut (dyn Error + Send + Sync) = Box::into_raw(self);
            Ok(unsafe { Box::from_raw(raw as *mut T) })
        } else {
            Err(self)
        }
    }
}

impl<'a, E: Error + 'a> From<E> for Box<dyn Error + 'a> {
    fn from(err: E) -> Box<dyn Error + 'a> {
        Box::new(err)
    }
}

impl<'a, E: Error + Send + Sync + 'a> From<E> for Box<dyn Error + Send + Sync + 'a> {
    fn from(err: E) -> Box<dyn Error + Send + Sync + 'a> {
        Box::new(err)
    }
}

/// An error carrying only a message (used by the `From<&str>`/`From<String>` conversions).
struct StringError(String);

impl Error for StringError {}

impl Display for StringError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Debug for StringError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl<'a> From<String> for Box<dyn Error + Send + Sync + 'a> {
    #[inline]
    fn from(err: String) -> Box<dyn Error + Send + Sync + 'a> {
        Box::new(StringError(err))
    }
}

impl<'a> From<String> for Box<dyn Error + 'a> {
    fn from(str_err: String) -> Box<dyn Error + 'a> {
        Box::new(StringError(str_err))
    }
}

impl<'a, 'b> From<&'b str> for Box<dyn Error + Send + Sync + 'a> {
    #[inline]
    fn from(err: &'b str) -> Box<dyn Error + Send + Sync + 'a> {
        Box::new(StringError(String::from(err)))
    }
}

impl<'a, 'b> From<&'b str> for Box<dyn Error + 'a> {
    fn from(err: &'b str) -> Box<dyn Error + 'a> {
        Box::new(StringError(String::from(err)))
    }
}

impl<T: Error> Error for Box<T> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Error::source(&**self)
    }
}

impl<'a, T: Error + ?Sized> Error for &'a T {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Error::source(&**self)
    }
}
