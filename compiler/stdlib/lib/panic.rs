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
    fn call_once(self, _args: ()) -> F::Output {
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
