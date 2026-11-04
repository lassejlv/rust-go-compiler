//! Compiler intrinsics. Every function declared here is implemented
//! directly by the code generator.

extern "rust-intrinsic" {
    // ---- type information
    pub fn size_of<T>() -> usize;
    pub fn align_of<T>() -> usize;
    pub fn size_of_val<T: ?Sized>(p: *const T) -> usize;
    pub fn align_of_val<T: ?Sized>(p: *const T) -> usize;
    pub fn needs_drop<T: ?Sized>() -> bool;
    pub fn type_name<T: ?Sized>() -> &'static str;
    // ---- #[track_caller]
    pub fn caller_location() -> &'static crate::panic::Location<'static>;
    /// Whether our caller was itself #[track_caller] (location propagated).
    pub fn caller_location_tracked() -> bool;
    /// true unless compiled with --release (integer overflow checks on)
    pub fn overflow_checks() -> bool;

    // ---- heap memory (size is never 0)
    pub fn alloc(size: usize, align: usize) -> *mut u8;
    pub fn realloc(p: *mut u8, old_size: usize, align: usize, new_size: usize) -> *mut u8;
    pub fn dealloc(p: *mut u8, size: usize, align: usize);
    // the system allocator itself (alloc/realloc/dealloc go through the
    // `#[global_allocator]` when one is installed)
    pub fn sys_alloc(size: usize, align: usize) -> *mut u8;
    pub fn sys_realloc(p: *mut u8, old_size: usize, align: usize, new_size: usize) -> *mut u8;
    pub fn sys_dealloc(p: *mut u8, size: usize, align: usize);
    pub fn set_global_alloc(
        alloc: fn(usize, usize) -> *mut u8,
        dealloc: fn(*mut u8, usize, usize),
        realloc: fn(*mut u8, usize, usize, usize) -> *mut u8,
    );

    // ---- raw memory
    pub fn copy<T>(src: *const T, dst: *mut T, count: usize);
    pub fn copy_nonoverlapping<T>(src: *const T, dst: *mut T, count: usize);
    pub fn write_bytes<T>(dst: *mut T, val: u8, count: usize);
    pub fn compare_bytes(a: *const u8, b: *const u8, n: usize) -> i32;
    pub fn read<T>(p: *const T) -> T;
    pub fn write<T>(p: *mut T, v: T);
    pub fn drop_in_place<T: ?Sized>(p: *mut T);
    pub fn forget<T>(v: T);
    pub fn transmute<T, U>(v: T) -> U;
    pub fn uninit<T>() -> T;
    pub fn zeroed<T>() -> T;

    // ---- pointers and fat pointers
    pub fn ptr_add<T>(p: *const T, n: usize) -> *const T;
    pub fn ptr_add_mut<T>(p: *mut T, n: usize) -> *mut T;
    pub fn ptr_offset<T>(p: *const T, n: isize) -> *const T;
    /// `p` with its address replaced by `addr` (keeping any metadata).
    pub fn ptr_with_addr<T: ?Sized>(p: *const T, addr: usize) -> *const T;
    pub fn ptr_with_addr_mut<T: ?Sized>(p: *mut T, addr: usize) -> *mut T;
    pub fn ptr_offset_mut<T>(p: *mut T, n: isize) -> *mut T;
    pub fn ptr_diff<T>(a: *const T, b: *const T) -> isize;
    pub fn slice_from_raw_parts<T>(p: *const T, len: usize) -> *const [T];
    pub fn slice_from_raw_parts_mut<T>(p: *mut T, len: usize) -> *mut [T];
    pub fn slice_len<T>(p: *const [T]) -> usize;
    pub fn slice_ptr<T>(p: *const [T]) -> *const T;
    pub fn str_from_raw_parts(p: *const u8, len: usize) -> *const str;
    pub fn str_len(p: *const str) -> usize;
    pub fn str_ptr(p: *const str) -> *const u8;

    // ---- enums
    pub fn discriminant_value<T>(v: &T) -> i64;

    // ---- process / io
    pub fn abort() -> !;
    pub fn exit(code: i32) -> !;
    pub fn write_stdout(p: *const u8, len: usize);
    pub fn write_stderr(p: *const u8, len: usize);
    pub fn flush_stdout();
    pub fn panic_impl(msg: &str, file: &str, line: u32, col: u32) -> !;

    // ---- panics and unwinding
    /// Like `panic_impl`, but the payload is a `&'static str` (not a `String`).
    pub fn panic_impl_static(msg: &str, file: &str, line: u32, col: u32) -> !;
    /// Unwinds with the given payload message without invoking the panic hook.
    pub fn panic_resume(msg: &str, is_static: bool) -> !;
    /// Calls `f(data)`; returns true if it panicked (the panic was caught).
    pub fn try_call(f: fn(*mut u8), data: *mut u8) -> bool;
    /// The message of the last panic caught by `try_call` on this thread.
    pub fn panic_message() -> &'static str;
    /// Whether the last caught panic had a `&'static str` payload.
    pub fn panic_message_is_static() -> bool;
    /// Whether the current thread is unwinding.
    pub fn thread_panicking() -> bool;

    // ---- threads (handles are opaque; `thread_spawn` returns 0 on failure)
    pub fn thread_spawn(entry: fn(*mut u8), data: *mut u8, stack_size: usize) -> u64;
    pub fn thread_join(handle: u64);
    pub fn thread_is_finished(handle: u64) -> bool;
    pub fn thread_retain(handle: u64);
    pub fn thread_release(handle: u64);
    pub fn thread_set_name(p: *const u8, len: usize);
    pub fn thread_sleep_ns(ns: u64);
    pub fn thread_yield();
    pub fn thread_is_main() -> bool;
    pub fn available_parallelism() -> usize;

    // ---- futex (timeout_ns = u64::MAX: no timeout; wait returns false on timeout)
    pub fn futex_wait(p: *const u32, expected: u32, timeout_ns: u64) -> bool;
    pub fn futex_wake(p: *const u32) -> bool;
    pub fn futex_wake_all(p: *const u32);

    // ---- thread-local storage (a destroyed slot reads as the address 1)
    pub fn tls_new_key() -> usize;
    pub fn tls_get(key: usize) -> *mut u8;
    pub fn tls_set(key: usize, p: *mut u8, dtor: fn(*mut u8));

    // ---- atomics (sequentially consistent; T = integer, bool or thin pointer)
    pub fn atomic_load<T>(p: *const T) -> T;
    pub fn atomic_store<T>(p: *mut T, v: T);
    pub fn atomic_xchg<T>(p: *mut T, v: T) -> T;
    pub fn atomic_cxchg<T>(p: *mut T, old: T, new: T) -> (T, bool);
    pub fn atomic_cxchg_weak<T>(p: *mut T, old: T, new: T) -> (T, bool);
    pub fn atomic_xadd<T>(p: *mut T, v: T) -> T;
    pub fn atomic_xsub<T>(p: *mut T, v: T) -> T;
    pub fn atomic_and<T>(p: *mut T, v: T) -> T;
    pub fn atomic_or<T>(p: *mut T, v: T) -> T;
    pub fn atomic_xor<T>(p: *mut T, v: T) -> T;
    pub fn atomic_nand<T>(p: *mut T, v: T) -> T;
    pub fn atomic_max<T>(p: *mut T, v: T) -> T;
    pub fn atomic_min<T>(p: *mut T, v: T) -> T;
    pub fn atomic_fence();

    // ---- time
    pub fn time_monotonic_ns() -> u64;
    pub fn time_realtime_ns() -> u64;

    // ---- process environment
    pub fn args_count() -> usize;
    pub fn arg_ptr(i: usize) -> *const u8;
    pub fn arg_len(i: usize) -> usize;
    /// The process's `argv` (a null-terminated array of C strings).
    pub fn argv_ptr() -> *const u8;
    /// Returns a NUL-terminated value, or null when the variable is not set.
    pub fn env_get(name: *const u8, len: usize) -> *const u8;
    pub fn env_count() -> usize;
    /// The i-th `KEY=VALUE` entry (NUL-terminated).
    pub fn env_entry(i: usize) -> *const u8;
    pub fn cstr_len(p: *const u8) -> usize;
    pub fn process_id() -> u32;
    /// Buffers stdin data; returns the number of available bytes (0 = EOF, < 0 = error).
    pub fn stdin_fill() -> isize;
    pub fn stdin_ptr() -> *const u8;
    pub fn stdin_consume(n: usize);

    // ---- integer arithmetic (T = any integer type)
    pub fn wrapping_add<T>(a: T, b: T) -> T;
    pub fn wrapping_sub<T>(a: T, b: T) -> T;
    pub fn wrapping_mul<T>(a: T, b: T) -> T;
    pub fn add_with_overflow<T>(a: T, b: T) -> (T, bool);
    pub fn sub_with_overflow<T>(a: T, b: T) -> (T, bool);
    pub fn mul_with_overflow<T>(a: T, b: T) -> (T, bool);
    pub fn unchecked_div<T>(a: T, b: T) -> T;
    pub fn unchecked_rem<T>(a: T, b: T) -> T;
    pub fn unchecked_shl<T>(a: T, b: u32) -> T;
    pub fn unchecked_shr<T>(a: T, b: u32) -> T;
    pub fn rotate_left<T>(a: T, n: u32) -> T;
    pub fn rotate_right<T>(a: T, n: u32) -> T;
    pub fn ctpop<T>(a: T) -> u32;
    pub fn ctlz<T>(a: T) -> u32;
    pub fn cttz<T>(a: T) -> u32;
    pub fn bswap<T>(a: T) -> T;

    // ---- f64 math
    pub fn sqrtf64(x: f64) -> f64;
    pub fn powif64(x: f64, n: i32) -> f64;
    pub fn powff64(x: f64, y: f64) -> f64;
    pub fn expf64(x: f64) -> f64;
    pub fn exp2f64(x: f64) -> f64;
    pub fn logf64(x: f64) -> f64;
    pub fn log2f64(x: f64) -> f64;
    pub fn log10f64(x: f64) -> f64;
    pub fn sinf64(x: f64) -> f64;
    pub fn cosf64(x: f64) -> f64;
    pub fn tanf64(x: f64) -> f64;
    pub fn asinf64(x: f64) -> f64;
    pub fn acosf64(x: f64) -> f64;
    pub fn atanf64(x: f64) -> f64;
    pub fn atan2f64(y: f64, x: f64) -> f64;
    pub fn sinhf64(x: f64) -> f64;
    pub fn coshf64(x: f64) -> f64;
    pub fn tanhf64(x: f64) -> f64;
    pub fn floorf64(x: f64) -> f64;
    pub fn ceilf64(x: f64) -> f64;
    pub fn roundf64(x: f64) -> f64;
    pub fn truncf64(x: f64) -> f64;
    pub fn fabsf64(x: f64) -> f64;
    pub fn copysignf64(x: f64, y: f64) -> f64;
    pub fn fmaf64(a: f64, b: f64, c: f64) -> f64;
    pub fn cbrtf64(x: f64) -> f64;
    pub fn hypotf64(x: f64, y: f64) -> f64;

    // ---- f32 math
    pub fn sqrtf32(x: f32) -> f32;
    pub fn powif32(x: f32, n: i32) -> f32;
    pub fn powff32(x: f32, y: f32) -> f32;
    pub fn expf32(x: f32) -> f32;
    pub fn exp2f32(x: f32) -> f32;
    pub fn logf32(x: f32) -> f32;
    pub fn log2f32(x: f32) -> f32;
    pub fn log10f32(x: f32) -> f32;
    pub fn sinf32(x: f32) -> f32;
    pub fn cosf32(x: f32) -> f32;
    pub fn tanf32(x: f32) -> f32;
    pub fn asinf32(x: f32) -> f32;
    pub fn acosf32(x: f32) -> f32;
    pub fn atanf32(x: f32) -> f32;
    pub fn atan2f32(y: f32, x: f32) -> f32;
    pub fn sinhf32(x: f32) -> f32;
    pub fn coshf32(x: f32) -> f32;
    pub fn tanhf32(x: f32) -> f32;
    pub fn floorf32(x: f32) -> f32;
    pub fn ceilf32(x: f32) -> f32;
    pub fn roundf32(x: f32) -> f32;
    pub fn truncf32(x: f32) -> f32;
    pub fn fabsf32(x: f32) -> f32;
    pub fn copysignf32(x: f32, y: f32) -> f32;
    pub fn fmaf32(a: f32, b: f32, c: f32) -> f32;
    pub fn cbrtf32(x: f32) -> f32;
    pub fn hypotf32(x: f32, y: f32) -> f32;

    // ---- float text conversion
    pub fn fmt_f64(v: f64, is_f32: bool, mode: u8, precision: isize, buf: *mut u8) -> usize;
    pub fn parse_f64(p: *const u8, len: usize, out: *mut f64) -> bool;
}
