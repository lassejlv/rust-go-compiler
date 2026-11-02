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

// ---------------------------------------------------------------------------
// Panic hooks

use crate::cell::UnsafeCell;
use crate::panic::{Location, PanicHookInfo};
use crate::sync::atomic::{AtomicBool, Ordering};

type HookFn = Box<dyn Fn(&PanicHookInfo<'_>) + Sync + Send + 'static>;

struct HookSlot {
    lock: AtomicBool,
    custom: AtomicBool,
    hook: UnsafeCell<Option<HookFn>>,
}

unsafe impl Sync for HookSlot {}

static HOOK: HookSlot = HookSlot { lock: AtomicBool::new(false), custom: AtomicBool::new(false), hook: UnsafeCell::new(None) };
static FIRST_PANIC: AtomicBool = AtomicBool::new(true);

fn lock_hook() {
    while HOOK.lock.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        crate::hint::spin_loop();
    }
}

fn unlock_hook() {
    HOOK.lock.store(false, Ordering::Release);
}

pub(crate) fn set_hook(hook: HookFn) {
    lock_hook();
    let old = unsafe { (*HOOK.hook.get()).replace(hook) };
    HOOK.custom.store(true, Ordering::SeqCst);
    unlock_hook();
    drop(old);
}

pub(crate) fn take_hook() -> HookFn {
    lock_hook();
    let old = unsafe { (*HOOK.hook.get()).take() };
    HOOK.custom.store(false, Ordering::SeqCst);
    unlock_hook();
    match old {
        Some(h) => h,
        None => Box::new(default_hook),
    }
}

/// The default panic hook: prints the message like the runtime does.
pub(crate) fn default_hook(info: &PanicHookInfo<'_>) {
    let t = crate::thread::current();
    let name = t.name().unwrap_or("<unnamed>");
    let msg = info.payload_as_str().unwrap_or("Box<dyn Any>");
    let loc = info.location().unwrap();
    let _ = crate::io::Write::flush(&mut crate::io::stdout());
    eprint!("\nthread '{}' ({}) panicked at {}:\n{}\n", name, crate::process::id(), loc, msg);
    if FIRST_PANIC.swap(false, Ordering::SeqCst) {
        eprint!("note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n");
    }
}

/// Runs a custom panic hook if one is installed; returns `false` when the
/// default (runtime) reporting should be used.
pub(crate) fn run_custom_hook(payload: &(dyn Any + Send), file: &str, line: u32, col: u32) -> bool {
    if !HOOK.custom.load(Ordering::SeqCst) {
        return false;
    }
    let loc = Location::internal_constructor(file, line, col);
    let info = PanicHookInfo::new(payload, &loc);
    lock_hook();
    let hook = unsafe { (*HOOK.hook.get()).take() };
    unlock_hook();
    match hook {
        Some(h) => {
            h(&info);
            lock_hook();
            unsafe {
                if (*HOOK.hook.get()).is_none() && HOOK.custom.load(Ordering::SeqCst) {
                    *HOOK.hook.get() = Some(h);
                }
            }
            unlock_hook();
        }
        None => default_hook(&info),
    }
    true
}

/// Panic entry for `&'static str` messages (used by `rt`).
pub(crate) fn begin_panic_static(msg: &'static str, file: &str, line: u32, col: u32) -> ! {
    if run_custom_hook(&msg, file, line, col) {
        unsafe { intrinsics::panic_resume(msg, true) }
    }
    unsafe { intrinsics::panic_impl_static(msg, file, line, col) }
}

/// Panic entry for formatted messages (used by `rt`).
pub(crate) fn begin_panic_string(msg: String, file: &str, line: u32, col: u32) -> ! {
    if run_custom_hook(&msg, file, line, col) {
        unsafe { intrinsics::panic_resume(msg.as_str(), false) }
    }
    unsafe { intrinsics::panic_impl(msg.as_str(), file, line, col) }
}
