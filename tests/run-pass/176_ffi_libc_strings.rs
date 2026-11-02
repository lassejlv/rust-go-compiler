use std::ffi::{c_char, c_int, CStr, CString};

extern "C" {
    fn abs(x: c_int) -> c_int;
    fn labs(x: i64) -> i64;
    fn strlen(s: *const c_char) -> usize;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int;
    fn atoi(s: *const c_char) -> c_int;
    fn strtol(s: *const c_char, end: *mut *mut c_char, base: c_int) -> i64;
    fn toupper(c: c_int) -> c_int;
    fn isdigit(c: c_int) -> c_int;
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
}

fn sign(x: c_int) -> &'static str {
    if x < 0 {
        "<"
    } else if x > 0 {
        ">"
    } else {
        "="
    }
}

fn main() {
    unsafe {
        println!("abs(-42) = {}", abs(-42));
        println!("labs = {}", labs(-9_000_000_000));
    }
    let s = CString::new("hello ffi").unwrap();
    let n = unsafe { strlen(s.as_ptr()) };
    println!("strlen = {}", n);
    println!("bytes_with_nul = {:?}", s.as_bytes_with_nul().len());

    let words = ["apple", "banana", "apple", "app"];
    let cs: Vec<CString> = words.iter().map(|w| CString::new(*w).unwrap()).collect();
    for i in 0..cs.len() - 1 {
        let r = unsafe { strcmp(cs[i].as_ptr(), cs[i + 1].as_ptr()) };
        println!("{} {} {}", words[i], sign(r), words[i + 1]);
    }
    let r = unsafe { strncmp(cs[0].as_ptr(), cs[3].as_ptr(), 3) };
    println!("strncmp first 3 equal: {}", r == 0);

    for t in ["123", "  -77xyz", "abc", "2147483647"] {
        let c = CString::new(t).unwrap();
        println!("atoi({:?}) = {}", t, unsafe { atoi(c.as_ptr()) });
    }
    let hex = CString::new("ff rest").unwrap();
    let mut end: *mut c_char = std::ptr::null_mut();
    let v = unsafe { strtol(hex.as_ptr(), &mut end, 16) };
    let rest = unsafe { CStr::from_ptr(end) };
    println!("strtol = {} rest = {:?}", v, rest.to_str().unwrap());

    let upper: String = "MiXeD 9!".bytes().map(|b| unsafe { toupper(b as c_int) } as u8 as char).collect();
    println!("{}", upper);
    let digits = "a1b22c333".bytes().filter(|&b| unsafe { isdigit(b as c_int) } != 0).count();
    println!("digits {}", digits);

    let path = CString::new("dir/sub/file.txt").unwrap();
    let p = unsafe { strchr(path.as_ptr(), '/' as c_int) };
    let tail = unsafe { CStr::from_ptr(p) }.to_string_lossy();
    println!("from first slash: {}", tail);
    let missing = unsafe { strchr(path.as_ptr(), 'z' as c_int) };
    println!("missing is null: {}", missing.is_null());

    let lit: &CStr = c"c string literal";
    println!("{:?} {}", lit, lit.to_bytes().len());
    let owned: CString = lit.to_owned();
    let back: String = owned.into_string().unwrap();
    println!("{}", back);
    println!("{:?}", CString::new("in\0side").is_err());
    let from_bytes = CStr::from_bytes_with_nul(b"xyz\0").unwrap();
    println!("{:?}", from_bytes.to_str());
    let raw = CString::new("roundtrip").unwrap().into_raw();
    let again = unsafe { CString::from_raw(raw) };
    println!("{:?}", again);
}
