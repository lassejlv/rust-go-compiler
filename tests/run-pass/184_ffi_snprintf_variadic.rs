use std::ffi::{c_char, c_double, c_int, c_long, CStr};

extern "C" {
    fn snprintf(buf: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    fn strtod(s: *const c_char, end: *mut *mut c_char) -> c_double;
    fn atol(s: *const c_char) -> c_long;
    fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    fn strcat(dst: *mut c_char, src: *const c_char) -> *mut c_char;
    fn malloc(n: usize) -> *mut u8;
    fn calloc(n: usize, size: usize) -> *mut u8;
    fn free(p: *mut u8);
}

fn fmt_c(fmt: &CStr, f: impl FnOnce(*mut c_char, usize, *const c_char) -> c_int) -> (i32, String) {
    let mut buf = [0 as c_char; 64];
    let n = f(buf.as_mut_ptr(), buf.len(), fmt.as_ptr());
    let s = unsafe { CStr::from_ptr(buf.as_ptr()) }.to_string_lossy().into_owned();
    (n, s)
}

fn main() {
    let (n, s) = fmt_c(c"int=%d neg=%d", |b, l, f| unsafe { snprintf(b, l, f, 42 as c_int, -7 as c_int) });
    println!("{} {:?}", n, s);
    let (n, s) = fmt_c(c"[%5d|%-5d|%05d]", |b, l, f| unsafe { snprintf(b, l, f, 12 as c_int, 34 as c_int, 56 as c_int) });
    println!("{} {:?}", n, s);
    let (n, s) = fmt_c(c"%.3f %e", |b, l, f| unsafe { snprintf(b, l, f, 3.14159 as c_double, 12500.0 as c_double) });
    println!("{} {:?}", n, s);
    let (n, s) = fmt_c(c"%s and %c", |b, l, f| unsafe { snprintf(b, l, f, c"strings".as_ptr(), 'Z' as c_int) });
    println!("{} {:?}", n, s);
    let (n, s) = fmt_c(c"%x %X %o %ld", |b, l, f| unsafe { snprintf(b, l, f, 255 as c_int, 3054 as c_int, 8 as c_int, 1234567890123 as c_long) });
    println!("{} {:?}", n, s);
    let mut small = [0 as c_char; 6];
    let n = unsafe { snprintf(small.as_mut_ptr(), small.len(), c"%s".as_ptr(), c"truncated".as_ptr()) };
    println!("{} {:?}", n, unsafe { CStr::from_ptr(small.as_ptr()) });

    let mut end: *mut c_char = std::ptr::null_mut();
    let input = c"2.5e3xyz";
    let d = unsafe { strtod(input.as_ptr(), &mut end) };
    println!("{} rest {:?}", d, unsafe { CStr::from_ptr(end) });
    println!("{}", unsafe { atol(c"-9876543210".as_ptr()) });

    let mut dst = [0 as c_char; 32];
    unsafe {
        strncpy(dst.as_mut_ptr(), c"foo".as_ptr(), 31);
        strcat(dst.as_mut_ptr(), c"-bar".as_ptr());
        strcat(dst.as_mut_ptr(), c"-baz".as_ptr());
    }
    println!("{:?}", unsafe { CStr::from_ptr(dst.as_ptr()) });

    unsafe {
        let p = malloc(16);
        println!("malloc ok: {}", !p.is_null());
        for i in 0..16 {
            *p.add(i) = (i as u8) * 3;
        }
        let s = std::slice::from_raw_parts(p, 16);
        println!("{:?}", s);
        free(p);
        let z = calloc(4, 4);
        let zs = std::slice::from_raw_parts(z as *const u32, 4);
        println!("calloc zeroed: {:?}", zs);
        free(z);
    }
}
