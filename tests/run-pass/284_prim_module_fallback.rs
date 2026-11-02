// A path starting with a module named like a primitive type falls back to
// the primitive type when the item is not found in the module.
#[allow(deprecated)]
use std::u8;
use core::char;
use std::str;
fn f() -> u8 {
    u8::MAX
}
fn main() {
    let c: char = char::from(66u8);
    println!("{} {:?} {} {}", char::from(65u8), char::from_u32(67), c, f());
    println!("{:?} {}", str::from_utf8(b"hi"), u8::max_value());
    let n: u8 = u8::from(3u8);
    println!("{} {}", n, char::from_digit(7, 10).unwrap());
}
