fn main() {
    let a: u8 = 200;
    let b: u8 = 55;
    println!("u8: {} + {} = {}", a, b, a + b);
    let c: i64 = 1 << 40;
    println!("i64: {}", c * 3);
    let d: u32 = 4_000_000_000;
    println!("u32: {}", d);
    let e: usize = 12345;
    println!("usize: {}", e * 2);
    let f: i8 = -128;
    println!("i8 min: {}", f);
    let g: i16 = 32767;
    println!("i16 max: {}", g);
    let h: u64 = 18446744073709551615;
    println!("u64 max: {}", h);
    println!("u8::MAX = {}, i32::MIN = {}, i64::MAX = {}", u8::MAX, i32::MIN, i64::MAX);
    println!("u16::MAX = {}, usize::MAX = {}", u16::MAX, usize::MAX);
    let x: u32 = 7;
    println!("u32 division: {} {}", x / 2, x % 2);
    let y: i64 = -9_000_000_000;
    println!("i64 negative: {}", y);
    println!("count_ones(255u8) = {}", 255u8.count_ones());
    println!("leading_zeros(1u32) = {}", 1u32.leading_zeros());
    println!("trailing_zeros(48u32) = {}", 48u32.trailing_zeros());
    println!("isqrt-ish: {}", (1000000u64 as f64).sqrt() as u64);
}
