//! Runtime support used by compiler-generated code (`panic!`, `assert_eq!`, ...).

use crate::fmt;
use crate::intrinsics;

/// The panic location: when called (via `panic!`) from a #[track_caller]
/// function such as `Option::unwrap`, its caller's location, as in rustc;
/// otherwise the `panic!` site passed explicitly.
#[inline(always)]
#[track_caller]
fn caller_or<'a>(file: &'a str, line: u32, col: u32) -> (&'a str, u32, u32) {
    if unsafe { intrinsics::caller_location_tracked() } {
        let l = unsafe { intrinsics::caller_location() };
        (l.file(), l.line(), l.column())
    } else {
        (file, line, col)
    }
}

/// Panics with a plain message (`panic!()` without format arguments, `assert!`).
#[inline(never)]
#[cold]
#[track_caller]
pub fn panic_str(msg: &str, file: &str, line: u32, col: u32) -> ! {
    // `msg` is always a string literal here (its payload is `&'static str`).
    let msg: &'static str = unsafe { &*(msg as *const str) };
    let (file, line, col) = caller_or(file, line, col);
    crate::panicking::begin_panic_static(msg, file, line, col)
}

/// Panics with a formatted message (`panic!("...", args)`).
#[inline(never)]
#[cold]
#[track_caller]
pub fn panic_fmt(args: fmt::Arguments<'_>, file: &str, line: u32, col: u32) -> ! {
    let (file, line, col) = caller_or(file, line, col);
    if let Some(s) = args.as_str() {
        crate::panicking::begin_panic_static(s, file, line, col)
    }
    crate::panicking::begin_panic_string(args.into_string(), file, line, col)
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
