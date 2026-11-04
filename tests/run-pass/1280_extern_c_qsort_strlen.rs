// extern "C" calls into libc: strlen, qsort with extern "C" callback, bsearch, memcmp, abs;
// #[no_mangle] functions; function pointers; CStr/CString.
use std::ffi::{c_char, c_int, c_void, CStr, CString};

extern "C" {
    fn strlen(s: *const c_char) -> usize;
    fn qsort(base: *mut c_void, n: usize, size: usize, cmp: extern "C" fn(*const c_void, *const c_void) -> c_int);
    fn bsearch(key: *const c_void, base: *const c_void, n: usize, size: usize,
               cmp: extern "C" fn(*const c_void, *const c_void) -> c_int) -> *mut c_void;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    fn abs(x: c_int) -> c_int;
    fn atoi(s: *const c_char) -> c_int;
}

extern "C" fn cmp_i32(a: *const c_void, b: *const c_void) -> c_int {
    let (a, b) = unsafe { (*(a as *const i32), *(b as *const i32)) };
    a.cmp(&b) as c_int
}

extern "C" fn cmp_desc_by_len(a: *const c_void, b: *const c_void) -> c_int {
    let (a, b) = unsafe { (&*(a as *const &str), &*(b as *const &str)) };
    match b.len().cmp(&a.len()) {
        std::cmp::Ordering::Equal => a.cmp(b) as c_int,
        o => o as c_int,
    }
}

#[no_mangle]
pub extern "C" fn rg_add(a: i32, b: i32) -> i32 {
    a.wrapping_add(b)
}

#[no_mangle]
pub static RG_MAGIC: u32 = 0xC0FFEE;

fn apply(f: fn(i32, i32) -> i32, a: i32, b: i32) -> i32 {
    f(a, b)
}
fn apply_c(f: extern "C" fn(i32, i32) -> i32, a: i32, b: i32) -> i32 {
    f(a, b)
}

fn main() {
    let cs = CString::new("hello, world").unwrap();
    println!("strlen {}", unsafe { strlen(cs.as_ptr()) });
    let lit = c"literal c str";
    println!("strlen lit {} {:?}", unsafe { strlen(lit.as_ptr()) }, lit);
    let mut nums = [42i32, -7, 19, 0, 3, 3, -100, 8];
    unsafe {
        qsort(nums.as_mut_ptr() as *mut c_void, nums.len(), std::mem::size_of::<i32>(), cmp_i32);
    }
    println!("{:?}", nums);
    let key = 19i32;
    let found = unsafe {
        bsearch(&key as *const i32 as *const c_void, nums.as_ptr() as *const c_void, nums.len(), 4, cmp_i32)
    };
    if !found.is_null() {
        let idx = (found as usize - nums.as_ptr() as usize) / 4;
        println!("found 19 at {}", idx);
    }
    let key = 5i32;
    let nf = unsafe { bsearch(&key as *const i32 as *const c_void, nums.as_ptr() as *const c_void, nums.len(), 4, cmp_i32) };
    println!("5 found: {}", !nf.is_null());

    let mut words = ["pear", "fig", "banana", "kiwi", "apple", "date"];
    unsafe {
        qsort(words.as_mut_ptr() as *mut c_void, words.len(), std::mem::size_of::<&str>(), cmp_desc_by_len);
    }
    println!("{:?}", words);
    let a = [1u8, 2, 3];
    let b = [1u8, 2, 4];
    let r = unsafe { memcmp(a.as_ptr() as *const c_void, b.as_ptr() as *const c_void, 3) };
    println!("memcmp sign {}", r.signum());
    println!("abs {} atoi {}", unsafe { abs(-17) }, unsafe { atoi(c"  -345xyz".as_ptr()) });

    println!("{} {}", apply(|a, b| rg_add(a, b), 2, 3), apply_c(rg_add, i32::MAX, 1));
    let ops: [(&str, fn(i32, i32) -> i32); 3] = [("add", |a, b| a + b), ("mul", |a, b| a * b), ("max", std::cmp::max)];
    for (n, f) in ops.iter() {
        println!("{} {}", n, f(6, 7));
    }
    let fp: extern "C" fn(i32, i32) -> i32 = rg_add;
    println!("fn ptr eq {}", fp as usize == (rg_add as extern "C" fn(i32, i32) -> i32) as usize);
    println!("magic {:#x}", RG_MAGIC);

    let owned = CString::new(vec![b'a', b'b', b'c']).unwrap();
    let raw = owned.into_raw();
    unsafe {
        *raw = b'X' as c_char;
        let back = CString::from_raw(raw);
        println!("{:?} {}", back, back.to_str().unwrap());
    }
    let cstr = CStr::from_bytes_with_nul(b"bytes\0").unwrap();
    println!("{:?} {} {:?}", cstr.to_bytes(), cstr.count_bytes(), cstr.to_string_lossy());
    println!("nul err {}", CString::new("a\0b").is_err());
    let p = c"ptr".as_ptr();
    let back = unsafe { CStr::from_ptr(p) };
    println!("{:?}", back.to_str());
}
