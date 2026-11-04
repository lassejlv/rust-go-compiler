// Layout facts observable through size_of with niches, plus transmutes between
// arrays/integers/structs and reading enum discriminants through pointers.
use std::mem::{size_of, transmute};
use std::num::{NonZeroU16, NonZeroU8};
use std::ptr::NonNull;

#[allow(dead_code)]
enum Three {
    A,
    B,
    C,
}
#[allow(dead_code)]
enum WithData {
    X(u32),
    Y(u8),
}
#[allow(dead_code)]
#[repr(C)]
struct Pair {
    a: u16,
    b: u8,
}
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

fn main() {
    println!("bool {} Option<bool> {} Option<Option<bool>> {}", size_of::<bool>(), size_of::<Option<bool>>(), size_of::<Option<Option<bool>>>());
    println!("char {} Option<char> {}", size_of::<char>(), size_of::<Option<char>>());
    println!("Option<NonZeroU8> {} Option<NonZeroU16> {}", size_of::<Option<NonZeroU8>>(), size_of::<Option<NonZeroU16>>());
    println!("Option<NonNull<u8>> {} Option<&[u8]> {} Option<Box<[u8]>> {}", size_of::<Option<NonNull<u8>>>(), size_of::<Option<&[u8]>>(), size_of::<Option<Box<[u8]>>>());
    println!("Option<Vec<u8>> {} Option<String> {}", size_of::<Option<Vec<u8>>>(), size_of::<Option<String>>());
    println!("Three {} Option<Three> {}", size_of::<Three>(), size_of::<Option<Three>>());
    println!("WithData {} Option<WithData> {}", size_of::<WithData>(), size_of::<Option<WithData>>());
    println!("Pair {} [Pair;3] {}", size_of::<Pair>(), size_of::<[Pair; 3]>());
    println!("Result<u32,()> {} Result<(),Box<u8>> {}", size_of::<Result<u32, ()>>(), size_of::<Result<(), Box<u8>>>());
    println!("Option<u64> {} Option<(u8,bool)> {}", size_of::<Option<u64>>(), size_of::<Option<(u8, bool)>>());
    println!("Option<fn()> {} Option<Option<&u8>> {}", size_of::<Option<fn()>>(), size_of::<Option<Option<&u8>>>());

    let v = Vec3 { x: 1.0, y: -2.0, z: 0.5 };
    let arr: [f32; 3] = unsafe { transmute(v) };
    println!("{:?}", arr);
    let back: Vec3 = unsafe { transmute([3.0f32, 4.0, 5.0]) };
    println!("{:?}", back);
    let bytes: [u8; 8] = unsafe { transmute(0x0102030405060708u64) };
    println!("{:?}", bytes);
    let n: u64 = unsafe { transmute(bytes) };
    println!("{:#x}", n);
    let words: [u16; 4] = unsafe { transmute(n) };
    println!("{:x?}", words);
    let x = 7i32;
    let addr: usize = unsafe { transmute(&x) };
    println!("addr matches {}", addr == &x as *const i32 as usize);
    let neg: u32 = unsafe { transmute(-1i32) };
    println!("{}", neg);
    let f: f64 = unsafe { transmute(0x400921fb54442d18u64) };
    println!("{}", f);
    let opt: Option<NonZeroU8> = unsafe { transmute(0u8) };
    let opt2: Option<NonZeroU8> = unsafe { transmute(9u8) };
    println!("{:?} {:?}", opt, opt2);
    let b: bool = unsafe { transmute(1u8) };
    println!("{}", b);
    let c: char = unsafe { transmute(0x41u32) };
    println!("{}", c);
    let tup: (u32, u32) = unsafe { transmute([1u32, 2]) };
    println!("{:?}", tup);
    let sl: &[u8] = &[1, 2, 3, 4];
    let (p, l): (*const u8, usize) = unsafe { transmute(sl) };
    println!("len {} first {}", l, unsafe { *p });
    let fptr: fn(i32) -> i32 = |x| x * 3;
    let raw: usize = unsafe { transmute(fptr) };
    let back: fn(i32) -> i32 = unsafe { transmute(raw) };
    println!("{}", back(14));
}
