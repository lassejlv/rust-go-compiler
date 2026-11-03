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
    let loc = Location::caller();
    if crate::panicking::run_custom_hook(&*payload, loc.file(), loc.line(), loc.column()) {
        crate::panicking::set_custom_payload(payload);
        unsafe { intrinsics::panic_resume(text.as_str(), false) }
    }
    crate::panicking::set_custom_payload(payload);
    unsafe { intrinsics::panic_impl(text.as_str(), loc.file(), loc.line(), loc.column()) }
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
    #[track_caller]
    #[inline]
    pub const fn caller() -> &'static Location<'static> {
        unsafe { crate::intrinsics::caller_location() }
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

/// A struct providing information about a panic, passed to the panic hook.
pub struct PanicHookInfo<'a> {
    payload: &'a (dyn Any + Send),
    location: &'a Location<'a>,
}

/// Deprecated alias of [`PanicHookInfo`].
pub type PanicInfo<'a> = PanicHookInfo<'a>;

impl<'a> PanicHookInfo<'a> {
    pub(crate) fn new(payload: &'a (dyn Any + Send), location: &'a Location<'a>) -> Self {
        PanicHookInfo { payload, location }
    }

    /// Returns the payload associated with the panic.
    pub fn payload(&self) -> &(dyn Any + Send) {
        self.payload
    }

    /// Returns the payload associated with the panic, if it is a string.
    pub fn payload_as_str(&self) -> Option<&str> {
        if let Some(s) = self.payload.downcast_ref::<&str>() {
            Some(s)
        } else if let Some(s) = self.payload.downcast_ref::<String>() {
            Some(s.as_str())
        } else {
            None
        }
    }

    /// Returns information about the location from which the panic originated.
    pub fn location(&self) -> Option<&Location<'_>> {
        Some(self.location)
    }

    /// Returns whether the panic handler is allowed to unwind the stack.
    pub fn can_unwind(&self) -> bool {
        true
    }
}

impl fmt::Display for PanicHookInfo<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("panicked at ")?;
        fmt::Display::fmt(self.location, f)?;
        if let Some(s) = self.payload_as_str() {
            f.write_str(":\n")?;
            f.write_str(s)?;
        }
        Ok(())
    }
}

impl fmt::Debug for PanicHookInfo<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PanicHookInfo")
            .field("payload", &crate::panicking::payload_as_str(self.payload))
            .field("location", &self.location)
            .field("can_unwind", &true)
            .finish()
    }
}

/// Registers a custom panic hook, replacing the previously registered hook.
///
/// NOTE: rustgo runs the hook for panics raised through `panic!`, `assert*!`,
/// `unwrap`/`expect` and other std panics; panics raised directly by
/// compiler-generated checks (arithmetic overflow, builtin indexing) are
/// reported by the runtime without the hook.
pub fn set_hook(hook: Box<dyn Fn(&PanicHookInfo<'_>) + Sync + Send + 'static>) {
    crate::panicking::set_hook(hook)
}

/// Unregisters the current panic hook and returns it, registering the default hook in its place.
pub fn take_hook() -> Box<dyn Fn(&PanicHookInfo<'_>) + Sync + Send + 'static> {
    crate::panicking::take_hook()
}
