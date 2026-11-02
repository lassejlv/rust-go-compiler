//! Hints to the compiler that affect how code should be emitted or optimized.

/// An identity function that hints to the compiler to be maximally pessimistic about what `black_box` could do.
#[inline(never)]
pub fn black_box<T>(dummy: T) -> T {
    dummy
}

/// Hints to the compiler that the current code path is cold.
#[inline(always)]
pub fn cold_path() {}

/// Emits a machine instruction to signal the processor that it is running in a busy-wait spin-loop.
#[inline]
pub fn spin_loop() {}

/// Informs the compiler that the site which is calling this function is not reachable.
#[inline]
#[track_caller]
pub unsafe fn unreachable_unchecked() -> ! {
    crate::rt::panic_str("internal error: entered unreachable code", file!(), line!(), column!())
}

/// Makes a soundness promise to the compiler that `cond` holds.
#[inline]
#[track_caller]
pub unsafe fn assert_unchecked(cond: bool) {
    if !cond {
        crate::rt::panic_str("hint::assert_unchecked must never be called when the condition is false", file!(), line!(), column!());
    }
}

/// Returns `b` (hint that the branch is unpredictable).
#[inline]
pub fn select_unpredictable<T>(b: bool, true_val: T, false_val: T) -> T {
    if b {
        true_val
    } else {
        false_val
    }
}
