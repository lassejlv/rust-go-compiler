use std::num::IntErrorKind;

fn main() {
    for (s, radix) in [("ff", 16), ("-101", 2), ("zz", 36), ("777", 8), ("Z", 36), ("+42", 10), ("1_000", 10), ("", 10), ("-", 10), ("12a", 10), ("80", 8)] {
        match i64::from_str_radix(s, radix) {
            Ok(v) => println!("{:>6} base {:>2} = {}", s, radix, v),
            Err(e) => println!("{:>6} base {:>2} err {:?} ({})", s, radix, e.kind(), e),
        }
    }
    println!("{:?}", "256".parse::<u8>().map_err(|e| e.kind().clone()));
    println!("{:?}", "-129".parse::<i8>().map_err(|e| *e.kind() == IntErrorKind::NegOverflow));
    println!("{:?}", "-1".parse::<u32>().map_err(|e| e.to_string()));
    println!("{:?}", "340282366920938463463374607431768211455".parse::<u128>());
    println!("{:?}", "-170141183460469231731687303715884105728".parse::<i128>());
    println!("{:?}", u64::from_str_radix("FFFFFFFFFFFFFFFF", 16));
    println!("{:?}", i32::from_str_radix("-80000000", 16));
    let n: u32 = 0xDEAD_BEEF;
    println!("{:b}", n);
    println!("{:o} {:x} {:X} {:#x} {:#X} {:#o} {:#b}", n, n, n, n, n, 8, 5);
    println!("[{:#012x}] [{:08b}] [{:>10x}] [{:<10o}] [{:^10X}]", 255, 5u8, 4096, 64, 0xabc);
    println!("{:x} {:b}", -1i8, -2i16);
    println!("{:#x}", i64::MIN);
    println!("{:+} {:+} {:+.2}", 5, -5, 1.0f64);
    println!("{:e} {:E} {:e}", 1234.5f64, 0.00012f32, 7u32);
    println!("{:x?} {:X?}", vec![10u8, 255], [171u16]);
    println!("{:#?}", (0x10u8,));
    fn to_base(mut n: u64, b: u32) -> String {
        if n == 0 { return "0".into(); }
        let mut s = vec![];
        while n > 0 {
            s.push(std::char::from_digit((n % b as u64) as u32, b).unwrap());
            n /= b as u64;
        }
        s.iter().rev().collect()
    }
    for b in [2, 3, 7, 16, 36] {
        print!("{} ", to_base(1_000_000, b));
    }
    println!();
    println!("{:?}", u8::try_from(300i32).is_err());
    println!("{:?}", i16::try_from(-5i64));
    println!("{}", u32::MAX.count_ones() + 0u32.leading_zeros() + 8u32.trailing_zeros());
    println!("{} {} {}", 0b1011u8.reverse_bits(), 0x1234u16.swap_bytes(), 0x12345678u32.rotate_left(8));
    println!("{:?} {:?}", 255u8.checked_add(1), 200u8.wrapping_mul(3));
    println!("{:?} {:?}", 100i8.overflowing_add(100), (-128i8).saturating_sub(1));
    println!("{} {}", 17i32.rem_euclid(-5), (-17i32).div_euclid(5));
    println!("{} {}", 2u64.pow(40), 3i32.pow(4));
    println!("{:?} {:?}", 10u32.checked_next_power_of_two(), 1000u32.ilog10());
    println!("{} {}", u16::from_be_bytes([1, 2]), i32::from_le_bytes([0xff, 0xff, 0xff, 0xff]));
    println!("{:?} {:?}", 0x0102u16.to_le_bytes(), 1.5f32.to_be_bytes());
    println!("{} {}", 7u32.isqrt(), 10i32.abs_diff(-5));
}
