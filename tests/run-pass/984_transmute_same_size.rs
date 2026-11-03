use std::mem::transmute;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Pair {
    lo: u32,
    hi: u32,
}

fn main() {
    let f = 1.5f32;
    let bits: u32 = unsafe { transmute(f) };
    println!("{:#010x} {}", bits, bits == f.to_bits());
    let d: f64 = unsafe { transmute(0x4000000000000000u64) };
    println!("{}", d);
    let arr: [u8; 4] = unsafe { transmute(0x01020304u32.to_be()) };
    println!("{:?}", arr);
    let n: u32 = unsafe { transmute([1u8, 0, 0, 0]) };
    println!("{}", n);
    let p: Pair = unsafe { transmute(0x0000000500000007u64) };
    println!("{:?}", p);
    let back: u64 = unsafe { transmute(p) };
    println!("{:#x}", back);
    let i: i8 = unsafe { transmute(200u8) };
    println!("{}", i);
    let words: [u16; 2] = unsafe { transmute([0xffu8, 0x00, 0x34, 0x12]) };
    println!("{:x?}", words);
    let ff: fn(i32) -> i32 = |x| x * 3;
    let raw: *const () = ff as *const ();
    let g: fn(i32) -> i32 = unsafe { transmute(raw) };
    println!("{}", g(14));
    let b: bool = unsafe { transmute(1u8) };
    println!("{}", b);
    let c: char = unsafe { transmute(0x41u32) };
    println!("{}", c);
    let arr2: [[u8; 2]; 2] = unsafe { transmute([1u8, 2, 3, 4]) };
    println!("{:?}", arr2);
    let sref: &[u8] = b"hello";
    let (ptr, len): (*const u8, usize) = unsafe { transmute(sref) };
    let rebuilt = unsafe { std::slice::from_raw_parts(ptr, len) };
    println!("{}", std::str::from_utf8(rebuilt).unwrap());
    let neg: u64 = unsafe { transmute(-1i64) };
    println!("{}", neg);
}
