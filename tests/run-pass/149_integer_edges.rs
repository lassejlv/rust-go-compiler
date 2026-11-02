fn main() {
    // wrapping
    println!("{} {} {}", 250u8.wrapping_add(10), 0u8.wrapping_sub(1), 100i8.wrapping_add(100));
    println!("{} {}", i16::MAX.wrapping_add(1), i32::MIN.wrapping_sub(1));
    println!("{} {}", u64::MAX.wrapping_mul(3), i64::MIN.wrapping_neg());
    println!("{}", (-128i8).wrapping_abs());
    // checked
    println!("{:?} {:?}", 200u8.checked_add(56), 200u8.checked_add(55));
    println!("{:?} {:?}", 5u32.checked_sub(6), i32::MIN.checked_div(-1));
    println!("{:?} {:?}", 10i64.checked_rem(0), 2u16.checked_pow(16));
    println!("{:?}", 2u16.checked_pow(15));
    // saturating
    println!("{} {} {}", 250u8.saturating_add(10), 5u8.saturating_sub(10), (-100i8).saturating_sub(100));
    println!("{} {}", i64::MAX.saturating_mul(2), i32::MIN.saturating_mul(-1));
    // overflowing
    println!("{:?} {:?}", 255u8.overflowing_add(1), 100u8.overflowing_add(1));
    println!("{:?} {:?}", i32::MAX.overflowing_mul(2), 0u16.overflowing_sub(1));
    // i128 / u128
    let big: i128 = 170141183460469231731687303715884105727;
    println!("{}", big);
    println!("{}", i128::MIN);
    println!("{}", u128::MAX);
    let f: u128 = (1..=30).map(|x| x as u128).product();
    println!("30! = {}", f);
    println!("{}", f / 1_000_000_007);
    println!("{}", f % 1_000_000_007);
    println!("{:x}", u128::MAX >> 3);
    println!("{}", (-big) / 7);
    println!("{}", (big as u128).count_ones());
    println!("{:?}", 3u128.checked_mul(u128::MAX / 2));
    // bit ops
    println!("{} {} {}", 1u32.leading_zeros(), 0u8.leading_zeros(), (-1i16).leading_zeros());
    println!("{} {}", 40u64.trailing_zeros(), 0b1011u8.count_ones());
    println!("{} {}", 0x12345678u32.rotate_left(8), 0x12345678u32.swap_bytes());
    println!("{:#x} {:#b}", 0xF0u8.reverse_bits(), 6u8);
    println!("{} {}", u32::MAX.count_zeros(), 1u64 << 40);
    println!("{} {}", 17u32.is_power_of_two(), 17u32.next_power_of_two());
    // pow / euclid
    println!("{} {} {}", 3i32.pow(4), 2u64.pow(63), (-2i64).pow(5));
    println!("{} {}", (-7i32).rem_euclid(3), (-7i32) % 3);
    println!("{} {}", (-7i32).div_euclid(3), (-7i32) / 3);
    println!("{} {}", 7i32.rem_euclid(-3), (-7.5f64).rem_euclid(2.0));
    println!("{} {}", (-5i32).abs(), (-5i32).signum());
    println!("{} {}", 10u32.abs_diff(25), (-3i32).abs_diff(4));
    println!("{}", 1000u32.isqrt());
    println!("{} {}", 255u8.ilog2(), 1000u32.ilog10());
    // casts
    println!("{} {} {}", 300i32 as u8, -1i32 as u8, -1i64 as u32);
    println!("{} {}", 200u8 as i8, 65535u16 as i16);
    println!("{} {}", -1i8 as u64, -1i8 as u8 as u64);
    println!("{} {} {}", 3.99f64 as i32, -3.99f64 as i32, -1.5f64 as u8);
    println!("{} {}", 1e20f64 as i32, -1e20f64 as i64);
    println!("{} {}", f64::NAN as i32, f64::INFINITY as u16);
    println!("{} {}", 300.7f32 as u8, 255.9f32 as u8);
    println!("{} {}", u64::MAX as f64, i64::MIN as f64);
    println!("{} {}", u64::MAX as f32, 16777217i32 as f32);
    println!("{} {}", 'A' as u8, 97u8 as char);
    println!("{} {}", '\u{263A}' as u32, 'é' as u8);
    println!("{} {}", true as i32, false as u8);
    println!("{:?} {:?}", u8::try_from(256i32).is_err(), i8::try_from(-128i64));
    println!("{:?} {:?}", u32::try_from(-1i32).ok(), usize::try_from(42u64));
    println!("{:?}", "  42".trim().parse::<u8>());
    println!("{:?} {:?}", "256".parse::<u8>().is_err(), "-0".parse::<i32>());
    println!("{:?}", i64::from_str_radix("-ff", 16));
    println!("{} {}", i8::MIN, u16::MAX);
    println!("{} {}", isize::BITS, u128::BITS);
}
