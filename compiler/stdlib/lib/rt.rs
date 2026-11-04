//! Runtime support used by compiler-generated code (`panic!`, `assert_eq!`, ...).

use crate::fmt;
use crate::intrinsics;

/// The panic location: when called (via `panic!`) from a #[track_caller]
/// function such as `Option::unwrap`, its caller's location, as in rustc;
/// otherwise the `panic!` site passed explicitly (the outermost macro call
/// site for a `panic!` in a macro_rules! body). A macro, not a function:
/// the check is about the caller of the panicking entry point itself.
macro_rules! caller_or {
    ($file:expr, $line:expr, $col:expr) => {
        if unsafe { intrinsics::caller_location_tracked() } {
            let l = unsafe { intrinsics::caller_location() };
            (l.file(), l.line(), l.column())
        } else {
            ($file, $line, $col)
        }
    };
}

/// Panics with a plain message (`panic!()` without format arguments, `assert!`).
#[inline(never)]
#[cold]
#[track_caller]
pub fn panic_str(msg: &str, file: &str, line: u32, col: u32) -> ! {
    // `msg` is always a string literal here (its payload is `&'static str`).
    let msg: &'static str = unsafe { &*(msg as *const str) };
    let (file, line, col) = caller_or!(file, line, col);
    crate::panicking::begin_panic_static(msg, file, line, col)
}

/// Panics with a formatted message (`panic!("...", args)`).
#[inline(never)]
#[cold]
#[track_caller]
pub fn panic_fmt(args: fmt::Arguments<'_>, file: &str, line: u32, col: u32) -> ! {
    let (file, line, col) = caller_or!(file, line, col);
    if let Some(s) = args.as_str() {
        crate::panicking::begin_panic_static(s, file, line, col)
    }
    crate::panicking::begin_panic_string(args.into_string(), file, line, col)
}

/// Panics with an arbitrary payload (the Rust 2015/2018 `panic!(expr)`).
#[inline(never)]
#[cold]
#[track_caller]
pub fn begin_panic<M: crate::any::Any + Send>(msg: M, file: &str, line: u32, col: u32) -> ! {
    let (file, line, col) = caller_or!(file, line, col);
    let payload: Box<dyn crate::any::Any + Send> = Box::new(msg);
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        crate::panicking::begin_panic_static(*s, file, line, col)
    }
    let payload = match payload.downcast::<String>() {
        Ok(s) => crate::panicking::begin_panic_string(*s, file, line, col),
        Err(p) => p,
    };
    let text = String::from(crate::panicking::payload_as_str(&*payload));
    if crate::panicking::run_custom_hook(&*payload, file, line, col) {
        crate::panicking::set_custom_payload(payload);
        unsafe { intrinsics::panic_resume(text.as_str(), false) }
    }
    crate::panicking::set_custom_payload(payload);
    unsafe { intrinsics::panic_impl(text.as_str(), file, line, col) }
}

/// Panics on a failed `assert_eq!` / `assert_ne!`.
#[inline(never)]
#[cold]
#[track_caller]
pub fn assert_failed<T, U>(
    op: &str,
    left: &T,
    right: &U,
    args: Option<fmt::Arguments<'_>>,
    file: &str,
    line: u32,
    col: u32,
) -> !
where
    T: fmt::Debug + ?Sized,
    U: fmt::Debug + ?Sized,
{
    match args {
        Some(args) => panic_fmt(
            format_args!("assertion `left {op} right` failed: {args}\n  left: {left:?}\n right: {right:?}"),
            file,
            line,
            col,
        ),
        None => panic_fmt(
            format_args!("assertion `left {op} right` failed\n  left: {left:?}\n right: {right:?}"),
            file,
            line,
            col,
        ),
    }
}

/// Panics for an out-of-bounds index (for builtin array/slice indexing).
#[inline(never)]
#[cold]
#[track_caller]
pub fn panic_bounds_check(index: usize, len: usize, file: &str, line: u32, col: u32) -> ! {
    panic_fmt(format_args!("index out of bounds: the len is {len} but the index is {index}"), file, line, col)
}

/// Converts the value returned by `main` into a process exit status.
#[inline]
pub fn main_result<T: crate::process::Termination>(result: T) -> i32 {
    result.report().to_i32()
}

/// Entry point for panics raised by the C runtime (bounds checks, integer
/// overflow, division by zero): goes through the panic hook like rustc's
/// `panic_bounds_check` / `panic_const_*`. `msg` is copied unless static.
#[inline(never)]
#[cold]
pub fn runtime_panic(msg: &'static str, file: &'static str, line: u32, col: u32, is_static: bool) -> ! {
    if is_static {
        crate::panicking::begin_panic_static(msg, file, line, col)
    }
    crate::panicking::begin_panic_string(crate::string::String::from(msg), file, line, col)
}
