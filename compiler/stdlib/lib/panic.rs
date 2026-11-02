//! Panic support in the standard library.

use crate::any::Any;
use crate::fmt;
use crate::intrinsics;
use crate::ops::{Deref, DerefMut};

/// A marker trait which represents "panic safe" types in Rust.
pub auto trait UnwindSafe {}

/// A marker trait representing types where a shared reference is considered unwind safe.
pub auto trait RefUnwindSafe {}

/// A simple wrapper around a type to assert that it is unwind safe.
pub struct AssertUnwindSafe<T>(pub T);

impl<T> Deref for AssertUnwindSafe<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for AssertUnwindSafe<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<F: FnOnce<()>> FnOnce<()> for AssertUnwindSafe<F> {
    type Output = F::Output;
    extern "rust-call" fn call_once(self, _args: ()) -> F::Output {
        (self.0).call_once(())
    }
}

impl<T: fmt::Debug> fmt::Debug for AssertUnwindSafe<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AssertUnwindSafe").field(&self.0).finish()
    }
}

impl<T: Default> Default for AssertUnwindSafe<T> {
    fn default() -> Self {
        AssertUnwindSafe(T::default())
    }
}

/// Invokes a closure, capturing the cause of an unwinding panic if one occurs.
pub fn catch_unwind<F: FnOnce() -> R + UnwindSafe, R>(f: F) -> crate::thread::Result<R> {
    crate::panicking::catch(f)
}

/// Triggers a panic without invoking the panic hook.
pub fn resume_unwind(payload: Box<dyn Any + Send>) -> ! {
    let (msg, is_static) = {
        let p: &(dyn Any + Send) = &*payload;
        if let Some(s) = p.downcast_ref::<&'static str>() {
            (String::from(*s), true)
        } else if let Some(s) = p.downcast_ref::<String>() {
            (s.clone(), false)
        } else {
            (String::from("Box<dyn Any>"), false)
        }
    };
    crate::panicking::set_custom_payload(payload);
    unsafe { intrinsics::panic_resume(msg.as_str(), is_static) }
}

/// Panics the current thread with the given message as the panic payload.
#[track_caller]
pub fn panic_any<M: 'static + Any + Send>(msg: M) -> ! {
    let payload: Box<dyn Any + Send> = Box::new(msg);
    let text = String::from(crate::panicking::payload_as_str(&*payload));
    crate::panicking::set_custom_payload(payload);
    unsafe { intrinsics::panic_impl(text.as_str(), file!(), line!(), column!()) }
}

/// A struct containing information about the location of a panic.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Location<'a> {
    file: &'a str,
    line: u32,
    col: u32,
}

impl<'a> Location<'a> {
    /// Returns the source location of the caller of this function.
    ///
    /// NOTE: rustgo does not implement `#[track_caller]` propagation yet, so
    /// this reports an unknown location.
    #[track_caller]
    #[inline]
    pub const fn caller() -> &'static Location<'static> {
        &UNKNOWN_LOCATION
    }

    /// Returns the name of the source file.
    #[inline]
    pub const fn file(&self) -> &'a str {
        self.file
    }

    /// Returns the line number.
    #[inline]
    pub const fn line(&self) -> u32 {
        self.line
    }

    /// Returns the column.
    #[inline]
    pub const fn column(&self) -> u32 {
        self.col
    }

    #[doc(hidden)]
    pub const fn internal_constructor(file: &'a str, line: u32, col: u32) -> Self {
        Location { file, line, col }
    }
}

static UNKNOWN_LOCATION: Location<'static> = Location { file: "<unknown>", line: 0, col: 0 };

impl fmt::Display for Location<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.col)
    }
}
