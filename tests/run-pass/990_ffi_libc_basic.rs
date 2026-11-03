use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_long, c_void};

extern "C" {
    fn strlen(s: *const c_char) -> usize;
    fn abs(x: c_int) -> c_int;
    fn labs(x: c_long) -> c_long;
    fn atoi(s: *const c_char) -> c_int;
    fn strtol(s: *const c_char, end: *mut *mut c_char, base: c_int) -> c_long;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn memset(p: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn toupper(c: c_int) -> c_int;
    fn strncpy(d: *mut c_char, s: *const c_char, n: usize) -> *mut c_char;
}

fn main() {
    let s = CString::new("hello, ffi").unwrap();
    println!("strlen = {}", unsafe { strlen(s.as_ptr()) });
    println!("abs = {} labs = {}", unsafe { abs(-42) }, unsafe { labs(-1234567890123) });
    let n = CString::new("  -317xyz").unwrap();
    println!("atoi = {}", unsafe { atoi(n.as_ptr()) });
    let hex = CString::new("ff zz").unwrap();
    let mut end: *mut c_char = std::ptr::null_mut();
    let v = unsafe { strtol(hex.as_ptr(), &mut end, 16) };
    let rest = unsafe { CStr::from_ptr(end) };
    println!("strtol = {} rest = {:?}", v, rest);
    let a = CString::new("apple").unwrap();
    let b = CString::new("banana").unwrap();
    let c = unsafe { strcmp(a.as_ptr(), b.as_ptr()) };
    println!("strcmp sign = {}", c.signum());
    let mut buf = [1u8; 10];
    unsafe {
        memset(buf.as_mut_ptr() as *mut c_void, 0x7, 4);
        memcpy(buf.as_mut_ptr().add(6) as *mut c_void, [9u8, 8, 7].as_ptr() as *const c_void, 3);
    }
    println!("{:?}", buf);
    let up: String = "shout".bytes().map(|ch| unsafe { toupper(ch as c_int) } as u8 as char).collect();
    println!("{}", up);
    let mut dst = [0 as c_char; 16];
    unsafe { strncpy(dst.as_mut_ptr(), a.as_ptr(), 15); }
    let back = unsafe { CStr::from_ptr(dst.as_ptr()) };
    println!("copied = {}", back.to_str().unwrap());

    let owned = CString::new(vec![b'a', b'b', b'c']).unwrap();
    println!("{:?} {:?}", owned, owned.as_bytes_with_nul());
    let err = CString::new("a\0b");
    println!("nul err: {}", err.is_err());
    if let Err(e) = err {
        println!("nul position {}", e.nul_position());
    }
    let cs = CStr::from_bytes_with_nul(b"lit\0").unwrap();
    println!("{} {}", cs.to_string_lossy(), cs.to_bytes().len());
    let raw = CString::new("roundtrip").unwrap().into_raw();
    unsafe {
        *raw = b'R' as c_char;
        let back = CString::from_raw(raw);
        println!("{}", back.into_string().unwrap());
    }
    let lit: &CStr = c"c-string literal";
    println!("{:?} len={}", lit, lit.count_bytes());
}
