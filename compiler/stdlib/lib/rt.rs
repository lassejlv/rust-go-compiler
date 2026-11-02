//! Runtime support used by compiler-generated code (`panic!`, `assert_eq!`, ...).

use crate::fmt;
use crate::intrinsics;

/// Panics with a plain message (`panic!()` without format arguments, `assert!`).
#[inline(never)]
#[cold]
#[track_caller]
pub fn panic_str(msg: &str, file: &str, line: u32, col: u32) -> ! {
    unsafe { intrinsics::panic_impl_static(msg, file, line, col) }
}

/// Panics with a formatted message (`panic!("...", args)`).
#[inline(never)]
#[cold]
#[track_caller]
pub fn panic_fmt(args: fmt::Arguments<'_>, file: &str, line: u32, col: u32) -> ! {
    if let Some(s) = args.as_str() {
        unsafe { intrinsics::panic_impl_static(s, file, line, col) }
    }
    let s = args.into_string();
    unsafe { intrinsics::panic_impl(s.as_str(), file, line, col) }
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
