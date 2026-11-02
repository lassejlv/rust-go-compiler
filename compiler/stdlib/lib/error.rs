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
        None
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
