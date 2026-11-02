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

    // ---- heap memory (size is never 0)
    pub fn alloc(size: usize, align: usize) -> *mut u8;
    pub fn realloc(p: *mut u8, old_size: usize, align: usize, new_size: usize) -> *mut u8;
    pub fn dealloc(p: *mut u8, size: usize, align: usize);

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
