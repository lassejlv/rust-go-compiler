use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int};

extern "C" {
    fn snprintf(buf: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    fn getenv(name: *const c_char) -> *const c_char;
    fn sqrt(x: c_double) -> c_double;
    fn pow(x: c_double, y: c_double) -> c_double;
    fn floor(x: c_double) -> c_double;
}

fn fmt_c(fmt: &str, f: impl FnOnce(*mut c_char, usize, *const c_char) -> c_int) -> (i32, String) {
    let mut buf = vec![0 as c_char; 64];
    let cf = CString::new(fmt).unwrap();
    let n = f(buf.as_mut_ptr(), buf.len(), cf.as_ptr());
    let s = unsafe { CStr::from_ptr(buf.as_ptr()) }.to_string_lossy().into_owned();
    (n, s)
}

fn main() {
    let (n, s) = fmt_c("%d|%5d|%-4d|%x", |b, l, f| unsafe { snprintf(b, l, f, 42 as c_int, 7 as c_int, 3 as c_int, 255 as c_int) });
    println!("{} [{}]", n, s);
    let (n, s) = fmt_c("%.3f %e", |b, l, f| unsafe { snprintf(b, l, f, 3.14159265 as c_double, 12345.678 as c_double) });
    println!("{} [{}]", n, s);
    let word = CString::new("world").unwrap();
    let (n, s) = fmt_c("hello %s! %c%c", |b, l, f| unsafe { snprintf(b, l, f, word.as_ptr(), 'o' as c_int, 'k' as c_int) });
    println!("{} [{}]", n, s);
    let (n, s) = fmt_c("%ld %lu %08.2f", |b, l, f| unsafe { snprintf(b, l, f, -9000000000i64, 18000000000u64, 2.5f64) });
    println!("{} [{}]", n, s);
    let mut small = [0 as c_char; 6];
    let fmt = CString::new("%s").unwrap();
    let long = CString::new("truncated text").unwrap();
    let n = unsafe { snprintf(small.as_mut_ptr(), small.len(), fmt.as_ptr(), long.as_ptr()) };
    println!("{} [{}]", n, unsafe { CStr::from_ptr(small.as_ptr()) }.to_str().unwrap());

    std::env::set_var("RUSTGO_FFI_TEST_VAR", "value-123");
    let name = CString::new("RUSTGO_FFI_TEST_VAR").unwrap();
    let p = unsafe { getenv(name.as_ptr()) };
    if p.is_null() {
        println!("not found");
    } else {
        println!("getenv = {}", unsafe { CStr::from_ptr(p) }.to_str().unwrap());
    }
    let missing = CString::new("RUSTGO_DEFINITELY_MISSING_VAR_X").unwrap();
    println!("missing null = {}", unsafe { getenv(missing.as_ptr()) }.is_null());
    println!("sqrt {} pow {} floor {}", unsafe { sqrt(2.0) }, unsafe { pow(2.0, 10.0) }, unsafe { floor(-2.5) });
}
