fn main() {
    let a: u8 = 250;
    println!("wrapping_add: {}", a.wrapping_add(10));
    println!("wrapping_sub: {}", 3u8.wrapping_sub(5));
    println!("wrapping_mul: {}", 100u8.wrapping_mul(3));
    println!("checked_add ok: {:?}", a.checked_add(5));
    println!("checked_add overflow: {:?}", a.checked_add(6));
    println!("checked_sub: {:?}", 0u32.checked_sub(1));
    println!("checked_div by zero: {:?}", 10i32.checked_div(0));
    println!("saturating_add: {}", a.saturating_add(100));
    println!("saturating_sub: {}", 5u8.saturating_sub(10));
    println!("overflowing_add: {:?}", a.overflowing_add(10));
    println!("i32 wrapping: {}", i32::MAX.wrapping_add(1));
    println!("i64 wrapping_neg: {}", i64::MIN.wrapping_neg());

    let x: u32 = 0b1100_1010;
    let y: u32 = 0b1010_0110;
    println!("x & y = {:b}", x & y);
    println!("x | y = {:b}", x | y);
    println!("x ^ y = {:b}", x ^ y);
    println!("!x = {}", !x);
    println!("!0i32 = {}", !0i32);
    println!("x << 4 = {}", x << 4);
    println!("x >> 3 = {}", x >> 3);
    println!("-16i32 >> 2 = {}", -16i32 >> 2);
    println!("0x80000000u32 >> 31 = {}", 0x8000_0000u32 >> 31);
    println!("1u64 << 50 = {}", 1u64 << 50);
    println!("rotate_left = {}", 0x8000_0001u32.rotate_left(4));
    println!("swap_bytes = {:x}", 0x12345678u32.swap_bytes());
    let mut flags = 0u8;
    flags |= 1 << 2;
    flags |= 1 << 5;
    flags &= !(1 << 2);
    flags ^= 1;
    println!("flags = {:08b}", flags);
    // popcount by hand
    let mut v = 0xDEADBEEFu32;
    let mut bits = 0;
    while v != 0 {
        bits += v & 1;
        v >>= 1;
    }
    println!("popcount(0xDEADBEEF) = {}", bits);
    // FNV-1a hash using wrapping ops
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in "hello".bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    println!("fnv1a(hello) = {:x}", hash);
}
