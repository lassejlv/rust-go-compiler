// Breadth test of integer operations across widths and signedness.
fn main() {
    println!("pow {} {} {}", 3i32.pow(4), 2u64.pow(63), (-2i8).pow(7));
    println!("checked_pow {:?} {:?} checked_add {:?} {:?}", 10u32.checked_pow(9), 10u32.checked_pow(10), i32::MAX.checked_add(1), 5u8.checked_sub(6));
    println!("checked_mul {:?} checked_div {:?} {:?} {:?} checked_rem {:?} checked_neg {:?} {:?}", 255u8.checked_mul(2), 7i32.checked_div(0), i32::MIN.checked_div(-1), 7i32.checked_div(-2), (-7i32).checked_rem(3), i8::MIN.checked_neg(), 5u8.checked_neg());
    println!("checked_shl {:?} {:?} checked_abs {:?} checked_ilog2 {:?}", 1u8.checked_shl(7), 1u8.checked_shl(8), i16::MIN.checked_abs(), 0u32.checked_ilog2());
    println!("wrapping add {} sub {} mul {} neg {} shl {} abs {} div {}", 250u8.wrapping_add(10), 0u16.wrapping_sub(1), 100i8.wrapping_mul(3), i32::MIN.wrapping_neg(), 1u32.wrapping_shl(33), i64::MIN.wrapping_abs(), i8::MIN.wrapping_div(-1));
    println!("saturating {} {} {} {} {}", 250u8.saturating_add(10), 5u8.saturating_sub(10), i8::MIN.saturating_sub(1), (-100i8).saturating_mul(2), i32::MIN.saturating_abs());
    println!("overflowing {:?} {:?} {:?} {:?}", 250u8.overflowing_add(10), 5i8.overflowing_sub(-128), i32::MIN.overflowing_neg(), 0x8000_0000u32.overflowing_mul(2));
    println!("leading_zeros {} {} {} trailing {} {} count_ones {} count_zeros {}", 1u32.leading_zeros(), 0u8.leading_zeros(), (-1i16).leading_zeros(), 8u32.trailing_zeros(), 0u64.trailing_zeros(), 0xF0F0u16.count_ones(), 7u8.count_zeros());
    println!("leading_ones {} trailing_ones {}", 0xF800_0000u32.leading_ones(), 0b0111u8.trailing_ones());
    println!("rotate_left {:#010x} rotate_right {:#06x} swap_bytes {:#x} reverse_bits {:#010b}", 0x1234_5678u32.rotate_left(8), 0xabcdu16.rotate_right(4), 0x1122_3344u32.swap_bytes(), 0b0000_0110u8.reverse_bits());
    println!("to_be_bytes {:?} to_le_bytes {:?} to_ne==le {}", 0x0102_0304u32.to_be_bytes(), 0x0102_0304u32.to_le_bytes(), 1u16.to_ne_bytes() == 1u16.to_le_bytes());
    println!("from_le_bytes {:#x} from_be_bytes {} i16 {}", u32::from_le_bytes([0x78, 0x56, 0x34, 0x12]), u64::from_be_bytes([0, 0, 0, 0, 0, 0, 1, 0]), i16::from_be_bytes([0xff, 0xfe]));
    println!("to_be {:#x} from_be {:#x}", 0x1234u16.to_be(), u16::from_be(0x3412));
    println!("abs_diff {} {} {} {}", 3u8.abs_diff(250), (-100i8).abs_diff(100), i32::MIN.abs_diff(i32::MAX), 10u64.abs_diff(3));
    println!("rem_euclid {} {} {} div_euclid {} {} {}", (-7i32).rem_euclid(3), 7i32.rem_euclid(-3), (-7i32).rem_euclid(-3), (-7i32).div_euclid(3), 7i32.div_euclid(-3), (-7i32).div_euclid(-3));
    println!("plain / % {} {} {} {}", -7 / 3, -7 % 3, 7 / -3, 7 % -3);
    println!("isqrt {} {} {} {}", 0u32.isqrt(), 99u32.isqrt(), 100u64.isqrt(), u64::MAX.isqrt());
    println!("ilog2 {} {} ilog10 {} {} {} ilog {}", 1u32.ilog2(), 1023u32.ilog2(), 999u32.ilog10(), 1000u64.ilog10(), u64::MAX.ilog10(), 243u32.ilog(3));
    println!("signum {} {} {} is_positive {} is_negative {}", (-5i32).signum(), 0i64.signum(), 9i8.signum(), 3i32.is_positive(), 0i32.is_negative());
    println!("is_power_of_two {} {} next_power_of_two {} {} checked {:?}", 64u32.is_power_of_two(), 0u32.is_power_of_two(), 65u32.next_power_of_two(), 0u8.next_power_of_two(), 200u8.checked_next_power_of_two());
    println!("div_ceil {} {} next_multiple_of {} {}", 7u32.div_ceil(2), 8u32.div_ceil(2), 13u32.next_multiple_of(5), 15u32.next_multiple_of(5));
    println!("midpoint {} {} {}", 0u8.midpoint(255), 10u32.midpoint(13), (-3i32).midpoint(4));
    println!("unsigned_abs {} {} cast_signed {} cast_unsigned {}", i8::MIN.unsigned_abs(), (-5i64).unsigned_abs(), 200u8.cast_signed(), (-1i16).cast_unsigned());
    println!("min/max/clamp {} {} {} {}", 3.min(9), 3.max(9), 15.clamp(0, 10), (-5i8).clamp(-3, 3));
    println!("MIN/MAX {} {} {} {} BITS {} {}", i8::MIN, u16::MAX, i128::MIN, u128::MAX, u64::BITS, i8::BITS);
    println!("from_str_radix {:?} {:?} {:?} {:?}", i32::from_str_radix("-ff", 16), u8::from_str_radix("101", 2), u64::from_str_radix("zz", 36), i8::from_str_radix("80", 16).map_err(|e| e.to_string()));
    println!("casts {} {} {} {} {} {} {}", 300i32 as u8, -1i32 as u32, 3.99f64 as i32, -3.99f64 as i32, 1e20f64 as i32, f64::NAN as i64, -1.0f64 as u8);
    println!("casts2 {} {} {} {} {}", 255u8 as i8, i64::MIN as i32, u32::MAX as f32, (1u64 << 53) as f64 + 1.0, 'A' as u8 + 1);
    println!("i128 {} {} {}", i128::MAX / 7, (-170141183460469231731687303715884105728i128).wrapping_sub(1), 12345678901234567890123456789u128 * 3);
    println!("u128 ops {:#x} {} {}", u128::MAX >> 64, (1u128 << 100).trailing_zeros(), (u128::MAX).count_ones());
    println!("i128 div/rem {} {} {} {}", -100i128 / 7, -100i128 % 7, (-100i128).rem_euclid(7), i128::MAX.leading_zeros());
    println!("formatting {:+} {:08.3} {:x} {:X} {:o} {:b} {:#b}", 42, -1.5, 255u8, 3054i32, 8, 5u8, -2i8);
    println!("negative hex {:x} {:x} {:#x}", -1i8, -1i32, i64::MIN);
    println!("width/fill {:>6} {:<6}| {:^7} {:06} {:+06} {:#06x}", 42, 42, -42, -42, 42, 255);
    println!("u128 fmt {:>40} {:e}", u128::MAX, 1234567u64);
    println!("parse edge {:?} {:?} {:?} {:?}", "-0".parse::<i32>(), "+0".parse::<u8>(), "-0".parse::<u8>(), "340282366920938463463374607431768211455".parse::<u128>());
    println!("swap via xor {:?}", { let (mut a, mut b) = (5u8, 9u8); a ^= b; b ^= a; a ^= b; (a, b) });
    println!("bit tricks {} {} {}", (0b1011_0000u8 & 0b1011_0000u8.wrapping_neg()), 37u32 & !(1 << 2), (12u32 ^ 10) >> 1);
    println!("shifts {} {} {} {}", -16i32 >> 2, (-16i32 as u32) >> 28, 1i64 << 62, 0x80u8 >> 7);
    println!("sum/product overflow-free {} {}", (1..=20u64).product::<u64>(), (1..=1000u32).map(|x| x as u64 * x as u64).sum::<u64>());
    println!("gcd/lcm {} {}", gcd(1071, 462), lcm(21, 6));
    println!("checked chain {:?}", 2u32.checked_pow(10).and_then(|x| x.checked_mul(4_000_000)).map(|x| x + 1));
    println!("strict-ish via checked {:?}", [u8::MAX, 1].iter().try_fold(0u8, |a, &b| a.checked_add(b)));
    let r = std::panic::catch_unwind(|| {
        let v: Vec<u8> = vec![200, 100];
        v.iter().sum::<u8>()
    });
    println!("overflow in sum panics in debug: {}", r.is_err());
    let r2 = std::panic::catch_unwind(|| {
        let z = std::hint::black_box(0);
        10 / z
    });
    println!("div by zero panics: {}", r2.is_err());
    let r3 = std::panic::catch_unwind(|| {
        let x = std::hint::black_box(i32::MIN);
        x / -1
    });
    println!("MIN / -1 panics: {}", r3.is_err());
    let r4 = std::panic::catch_unwind(|| {
        let s = std::hint::black_box(40u32);
        1u32 << s
    });
    println!("shift overflow panics: {}", r4.is_err());
}

fn gcd(a: u64, b: u64) -> u64 { if b == 0 { a } else { gcd(b, a % b) } }
fn lcm(a: u64, b: u64) -> u64 { a / gcd(a, b) * b }
