// Newer stable syntax/features: &raw const/&raw mut, C string literals,
// const fn with &mut parameters, #[expect] lint attribute, unsafe extern
// blocks with safe items, Wrapping/Saturating number types, inline const in
// patterns, and associated const defaults depending on generics.
use std::ffi::CStr;
use std::num::{Saturating, Wrapping};

unsafe extern "C" {
    safe fn abs(x: i32) -> i32;
    pub unsafe fn strlen(s: *const std::ffi::c_char) -> usize;
}

const fn bump(x: &mut u32, by: u32) { *x += by; }
const fn build() -> [u32; 4] {
    let mut a = [1, 2, 3, 4];
    let mut i = 0;
    while i < a.len() { bump(&mut a[i], i as u32 * 10); i += 1; }
    a
}
const BUILT: [u32; 4] = build();

const fn swap_ends(arr: &mut [u8; 5]) {
    let t = arr[0];
    arr[0] = arr[4];
    arr[4] = t;
}
const SWAPPED: [u8; 5] = { let mut a = *b"hello"; swap_ends(&mut a); a };

#[repr(C)]
#[derive(Debug, Default)]
struct Packet { tag: u8, len: u16, payload: [u8; 4] }

trait Sized2 { const BYTES: usize; const DOUBLE: usize = Self::BYTES * 2; }
impl<T> Sized2 for Vec<T> { const BYTES: usize = std::mem::size_of::<T>(); }

#[expect(unused_variables)]
fn expects_unused() -> u8 { let unused = 5; 1 }

fn classify(n: u32) -> &'static str {
    match n {
        x if x == const { 1 << 4 } => "sixteen",
        x if x >= const { u32::MAX / 2 } => "huge",
        _ => "other",
    }
}

fn main() {
    let mut p = Packet::default();
    let raw_len: *const u16 = &raw const p.len;
    let raw_tag: *mut u8 = &raw mut p.tag;
    unsafe {
        *raw_tag = 9;
        println!("raw ptrs: len={} tag={}", *raw_len, *raw_tag);
        let pay = &raw mut p.payload;
        (*pay)[2] = 0xAB;
    }
    println!("{:?}", p);

    let cs: &CStr = c"hello, c-string";
    println!("cstr {:?} len={} bytes_with_nul={} to_str={:?}", cs, cs.count_bytes(), cs.to_bytes_with_nul().len(), cs.to_str());
    let n = unsafe { strlen(cs.as_ptr()) };
    println!("strlen via extern = {} abs(-42) = {}", n, abs(-42));
    let with_escape = c"tab\there\x41";
    println!("escape cstr bytes {:?}", with_escape.to_bytes());
    println!("CStr from bytes: {:?} {:?}", CStr::from_bytes_with_nul(b"ok\0"), CStr::from_bytes_with_nul(b"no nul").is_err());
    println!("CStr until nul: {:?}", CStr::from_bytes_until_nul(b"ab\0cd\0").unwrap());

    println!("const fn &mut: {:?} {:?}", BUILT, std::str::from_utf8(&SWAPPED).unwrap());
    println!("assoc const defaults: {} {} {}", <Vec<u64> as Sized2>::BYTES, <Vec<u16> as Sized2>::DOUBLE, <Vec<()> as Sized2>::DOUBLE);
    println!("expect attr fn: {}", expects_unused());
    println!("{} {} {}", classify(16), classify(u32::MAX), classify(3));

    let mut w = Wrapping(250u8);
    w += Wrapping(10);
    let w2 = w * Wrapping(3) - Wrapping(100);
    println!("Wrapping: {} {} {:?} shl {}", w, w2, -Wrapping(1i8), Wrapping(1u8) << 9);
    let mut s = Saturating(250u8);
    s += Saturating(10);
    let s2 = Saturating(5u8) - Saturating(10);
    let s3 = Saturating(i8::MIN) * Saturating(-1);
    println!("Saturating: {} {} {} {:?}", s, s2, s3, Saturating(100i16) / Saturating(-3));
    let total: Wrapping<u16> = (0..1000u16).map(Wrapping).sum();
    println!("wrapping sum {}", total);
    println!("unbounded shifts: {} {} {}", 1u32.unbounded_shl(40), u64::MAX.unbounded_shr(64), (-8i32).unbounded_shr(50));
    println!("char consts: {:?} {:?} {}", char::MIN, char::MAX, char::REPLACEMENT_CHARACTER);
    println!("next_up/down: {} {} {}", 1.0f64.next_up() - 1.0, 0.0f32.next_down(), f64::MAX.next_up());
    let opt = Some(3);
    println!("as_slice {:?} {:?} inspect: {:?}", opt.as_slice(), None::<u8>.as_slice(), opt.inspect(|v| println!("  inspecting {}", v)).map(|v| v + 1));
    let r: Result<u8, String> = Err("e".into());
    let _ = r.inspect_err(|e| println!("  inspect_err {}", e)).is_err_and(|e| e.len() == 1);
    let leaked: &'static mut [i32] = vec![1, 2, 3].leak();
    leaked[0] = 100;
    let sleak: &'static str = String::from("leaked string").leak();
    println!("leaked {:?} {}", leaked, sleak);
    let mut v = vec![1, 2, 3];
    v.extend_from_within(1..);
    v.extend_from_within(..2);
    println!("extend_from_within {:?}", v);
}
