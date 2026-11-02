//! Panic support: catching panics and building their payloads.

use crate::any::Any;
use crate::cell::RefCell;
use crate::intrinsics;

thread_local! {
    // A payload set by `panic_any` / `resume_unwind` for the panic in flight.
    static CUSTOM_PAYLOAD: RefCell<Option<Box<dyn Any + Send>>> = RefCell::new(None);
}

pub(crate) fn set_custom_payload(p: Box<dyn Any + Send>) {
    CUSTOM_PAYLOAD.with(|c| *c.borrow_mut() = Some(p));
}

// The payload of the panic that was just caught on this thread.
fn take_payload() -> Box<dyn Any + Send> {
    let custom = CUSTOM_PAYLOAD.with(|c| c.borrow_mut().take());
    if let Some(p) = custom {
        return p;
    }
    unsafe {
        let msg = intrinsics::panic_message();
        if intrinsics::panic_message_is_static() {
            let s: &'static str = String::from(msg).leak();
            Box::new(s)
        } else {
            Box::new(String::from(msg))
        }
    }
}

struct CatchData<F, R> {
    f: Option<F>,
    r: Option<R>,
}

fn catch_call<F, R>(p: *mut u8)
where
    F: FnOnce() -> R,
{
    let d = unsafe { &mut *(p as *mut CatchData<F, R>) };
    let f = d.f.take().unwrap();
    d.r = Some(f());
}

/// Calls `f`, catching a panic (its destructors have run when this returns).
pub(crate) fn catch<F, R>(f: F) -> Result<R, Box<dyn Any + Send>>
where
    F: FnOnce() -> R,
{
    let mut d: CatchData<F, R> = CatchData { f: Some(f), r: None };
    let panicked = unsafe { intrinsics::try_call(catch_call::<F, R>, &mut d as *mut CatchData<F, R> as *mut u8) };
    if panicked {
        Err(take_payload())
    } else {
        Ok(d.r.take().unwrap())
    }
}

// The message the default hook prints for a payload.
pub(crate) fn payload_as_str(payload: &(dyn Any + Send)) -> &str {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        s
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.as_str()
    } else {
        "Box<dyn Any>"
    }
}
