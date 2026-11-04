// Generic extern "C" trampolines passing Rust closures as void* user data,
// #[repr(C)] structs by value across the C ABI (div/ldiv), variadic snprintf, malloc/free.
use std::ffi::{c_char, c_int, c_long, c_void, CStr};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct DivT {
    quot: c_int,
    rem: c_int,
}
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct LDivT {
    quot: c_long,
    rem: c_long,
}

extern "C" {
    fn div(n: c_int, d: c_int) -> DivT;
    fn ldiv(n: c_long, d: c_long) -> LDivT;
    fn snprintf(buf: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn memset(p: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn qsort_r(base: *mut c_void, n: usize, size: usize,
               cmp: unsafe extern "C" fn(*const c_void, *const c_void, *mut c_void) -> c_int, arg: *mut c_void);
}

unsafe extern "C" fn tramp_cmp<T, F: FnMut(&T, &T) -> std::cmp::Ordering>(a: *const c_void, b: *const c_void, data: *mut c_void) -> c_int {
    let f = &mut *(data as *mut F);
    f(&*(a as *const T), &*(b as *const T)) as c_int
}

fn sort_by_c<T, F: FnMut(&T, &T) -> std::cmp::Ordering>(v: &mut [T], mut f: F) {
    unsafe {
        qsort_r(v.as_mut_ptr() as *mut c_void, v.len(), std::mem::size_of::<T>(), tramp_cmp::<T, F>,
                &mut f as *mut F as *mut c_void);
    }
}

// a "C library" implemented in Rust that takes a callback + user data
extern "C" fn for_each_c(n: c_int, cb: extern "C" fn(c_int, *mut c_void), data: *mut c_void) {
    for i in 0..n {
        cb(i, data);
    }
}
extern "C" fn trampoline<F: FnMut(i32)>(x: c_int, data: *mut c_void) {
    let f = unsafe { &mut *(data as *mut F) };
    f(x);
}
fn for_each<F: FnMut(i32)>(n: i32, mut f: F) {
    for_each_c(n, trampoline::<F>, &mut f as *mut F as *mut c_void);
}

fn main() {
    println!("{:?}", unsafe { div(17, 5) });
    println!("{:?}", unsafe { div(-17, 5) });
    println!("{:?}", unsafe { ldiv(10_000_000_007, 1000) });

    let mut buf = [0 as c_char; 64];
    let n = unsafe {
        snprintf(buf.as_mut_ptr(), buf.len(), c"%d-%s-%.2f-%x-%ld".as_ptr(), 42 as c_int, c"str".as_ptr(), 3.14159f64, 255 as c_int, -5i64 as c_long)
    };
    let s = unsafe { CStr::from_ptr(buf.as_ptr()) };
    println!("{} {:?}", n, s);

    let mut v = vec![(3, "c"), (1, "a"), (2, "b"), (1, "z")];
    let mut calls = 0;
    sort_by_c(&mut v, |a, b| {
        calls += 1;
        b.0.cmp(&a.0).then(a.1.cmp(b.1))
    });
    println!("{:?} called={}", v, calls > 0);
    let mut words = vec![String::from("ccc"), String::from("a"), String::from("bb")];
    sort_by_c(&mut words, |a, b| a.len().cmp(&b.len()));
    println!("{:?}", words);

    let mut acc = Vec::new();
    for_each(5, |x| acc.push(x * x));
    println!("{:?}", acc);
    let mut sum = 0i64;
    for_each(100, |x| sum += x as i64);
    println!("sum {}", sum);

    unsafe {
        let p = malloc(16) as *mut u8;
        memset(p as *mut c_void, 0xAB, 16);
        let sl = std::slice::from_raw_parts(p, 16);
        println!("{}", sl.iter().all(|&b| b == 0xAB));
        *p.add(3) = 1;
        println!("{:?}", &std::slice::from_raw_parts(p, 6)[..]);
        free(p as *mut c_void);
    }
}
