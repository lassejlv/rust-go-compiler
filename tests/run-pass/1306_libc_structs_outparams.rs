// libc calls with #[repr(C)] structs and out-parameters: gmtime_r/strftime (struct tm),
// strtol/strtod end pointers, sscanf (variadic out-pointers), memchr, strerror, getenv,
// and a C-layout struct array sorted with qsort by a float key.
use std::ffi::{c_char, c_double, c_int, c_long, c_void, CStr, CString};
use std::ptr;

#[repr(C)]
#[derive(Debug)]
struct Tm {
    tm_sec: c_int,
    tm_min: c_int,
    tm_hour: c_int,
    tm_mday: c_int,
    tm_mon: c_int,
    tm_year: c_int,
    tm_wday: c_int,
    tm_yday: c_int,
    tm_isdst: c_int,
    tm_gmtoff: c_long,
    tm_zone: *const c_char,
}

extern "C" {
    fn gmtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
    fn timegm(tm: *mut Tm) -> i64;
    fn strftime(s: *mut c_char, max: usize, fmt: *const c_char, tm: *const Tm) -> usize;
    fn strtol(s: *const c_char, end: *mut *mut c_char, base: c_int) -> c_long;
    fn strtod(s: *const c_char, end: *mut *mut c_char) -> c_double;
    fn sscanf(s: *const c_char, fmt: *const c_char, ...) -> c_int;
    fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
    fn strerror(e: c_int) -> *const c_char;
    fn getenv(name: *const c_char) -> *const c_char;
    fn qsort(base: *mut c_void, n: usize, size: usize, cmp: extern "C" fn(*const c_void, *const c_void) -> c_int);
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Sample {
    id: u8,
    weight: f64,
    tag: [u8; 3],
}

extern "C" fn by_weight(a: *const c_void, b: *const c_void) -> c_int {
    let (a, b) = unsafe { (&*(a as *const Sample), &*(b as *const Sample)) };
    a.weight.partial_cmp(&b.weight).unwrap() as c_int
}

fn main() {
    let t: i64 = 1_000_000_000;
    let mut tm = Tm { tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 0, tm_mon: 0, tm_year: 0, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_gmtoff: 0, tm_zone: ptr::null() };
    let r = unsafe { gmtime_r(&t, &mut tm) };
    println!("ret same {}", r == &mut tm as *mut Tm);
    println!("{}-{:02}-{:02} {:02}:{:02}:{:02} wday {} yday {}", tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday, tm.tm_hour, tm.tm_min, tm.tm_sec, tm.tm_wday, tm.tm_yday);
    let mut buf = [0 as c_char; 64];
    let n = unsafe { strftime(buf.as_mut_ptr(), buf.len(), c"%Y/%m/%d %H:%M %a %b".as_ptr(), &tm) };
    println!("{} {:?}", n, unsafe { CStr::from_ptr(buf.as_ptr()) });
    tm.tm_mday += 40;
    println!("timegm {}", unsafe { timegm(&mut tm) });
    println!("normalized mon {} mday {}", tm.tm_mon + 1, tm.tm_mday);
    println!("sizeof Tm {}", std::mem::size_of::<Tm>());

    let s = CString::new("  -1234xyz").unwrap();
    let mut end: *mut c_char = ptr::null_mut();
    let v = unsafe { strtol(s.as_ptr(), &mut end, 10) };
    let consumed = end as usize - s.as_ptr() as usize;
    println!("strtol {} consumed {} rest {:?}", v, consumed, unsafe { CStr::from_ptr(end) });
    let hex = unsafe { strtol(c"ff".as_ptr(), ptr::null_mut(), 16) };
    println!("hex {}", hex);
    let d = unsafe { strtod(c"3.75e2 tail".as_ptr(), &mut end) };
    println!("strtod {} rest {:?}", d, unsafe { CStr::from_ptr(end) });

    let mut a: c_int = 0;
    let mut b: c_double = 0.0;
    let mut word = [0 as c_char; 16];
    let got = unsafe { sscanf(c"42 2.5 hello".as_ptr(), c"%d %lf %15s".as_ptr(), &mut a as *mut c_int, &mut b as *mut c_double, word.as_mut_ptr()) };
    println!("sscanf {} -> {} {} {:?}", got, a, b, unsafe { CStr::from_ptr(word.as_ptr()) });

    let hay = b"find the x here";
    let p = unsafe { memchr(hay.as_ptr() as *const c_void, b'x' as c_int, hay.len()) };
    println!("memchr at {}", p as usize - hay.as_ptr() as usize);
    let none = unsafe { memchr(hay.as_ptr() as *const c_void, b'Q' as c_int, hay.len()) };
    println!("memchr none {}", none.is_null());
    println!("strerror(2) {:?}", unsafe { CStr::from_ptr(strerror(2)) });
    let missing = unsafe { getenv(c"RG_1306_DEFINITELY_UNSET".as_ptr()) };
    println!("getenv missing null {}", missing.is_null());

    let mut samples = [
        Sample { id: 1, weight: 2.5, tag: *b"abc" },
        Sample { id: 2, weight: -1.0, tag: *b"def" },
        Sample { id: 3, weight: 10.25, tag: *b"ghi" },
        Sample { id: 4, weight: 0.0, tag: *b"jkl" },
    ];
    unsafe { qsort(samples.as_mut_ptr() as *mut c_void, samples.len(), std::mem::size_of::<Sample>(), by_weight) };
    for s in &samples {
        println!("{} {} {}", s.id, s.weight, std::str::from_utf8(&s.tag).unwrap());
    }
    println!("Sample size {} align {}", std::mem::size_of::<Sample>(), std::mem::align_of::<Sample>());
}
