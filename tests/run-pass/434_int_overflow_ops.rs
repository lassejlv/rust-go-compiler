// Integer overflow behaviours: wrapping_*, checked_*, overflowing_*, saturating_* for add/sub/mul/
// div/rem/neg/shl/shr/pow/abs across widths and signedness; Wrapping<T>; div_euclid/rem_euclid;
// overflow panics caught by catch_unwind (debug build semantics are not relied on: explicit checks only).
use std::num::Wrapping;

macro_rules! show_all {
    ($t:ty, $a:expr, $b:expr) => {{
        let a: $t = $a;
        let b: $t = $b;
        println!(
            "{} {} {}: wadd={} wsub={} wmul={} cadd={:?} csub={:?} cmul={:?} cdiv={:?} oadd={:?} omul={:?} sadd={} ssub={} smul={}",
            stringify!($t), a, b,
            a.wrapping_add(b), a.wrapping_sub(b), a.wrapping_mul(b),
            a.checked_add(b), a.checked_sub(b), a.checked_mul(b), a.checked_div(b),
            a.overflowing_add(b), a.overflowing_mul(b),
            a.saturating_add(b), a.saturating_sub(b), a.saturating_mul(b)
        );
    }};
}

fn main() {
    show_all!(u8, 200, 100);
    show_all!(u8, 3, 0);
    show_all!(i8, -128, -1);
    show_all!(i8, 127, 2);
    show_all!(i16, -300, 200);
    show_all!(u32, 0, 1);
    show_all!(i32, i32::MIN, 2);
    show_all!(u64, u64::MAX, u64::MAX);
    show_all!(i64, i64::MAX, -1);
    show_all!(usize, 5, 7);

    // negation & abs
    println!("{:?} {:?} {} {}", (-128i8).checked_neg(), 5i8.checked_neg(), (-128i8).wrapping_neg(), (-128i8).wrapping_abs());
    println!("{:?} {:?} {}", i32::MIN.overflowing_neg(), i32::MIN.checked_abs(), i32::MIN.saturating_abs());
    println!("{} {} {:?}", 0u8.wrapping_neg(), 1u8.wrapping_neg(), 1u32.overflowing_neg());
    println!("{} {}", (-7i32).unsigned_abs(), i64::MIN.unsigned_abs());
    println!("{} {}", 5u8.abs_diff(200), (-100i8).abs_diff(100));

    // division edge cases
    println!("{:?} {:?} {:?}", i32::MIN.checked_div(-1), i32::MIN.overflowing_div(-1), i32::MIN.wrapping_div(-1));
    println!("{:?} {:?} {}", i32::MIN.checked_rem(-1), i32::MIN.overflowing_rem(-1), i32::MIN.wrapping_rem(-1));
    println!("{:?} {:?}", 10i32.checked_rem(0), 10u8.checked_div(0));
    for (a, b) in [(7i32, 3i32), (-7, 3), (7, -3), (-7, -3)] {
        println!("{}/{}: / {} % {} div_euclid {} rem_euclid {}", a, b, a / b, a % b, a.div_euclid(b), a.rem_euclid(b));
    }

    // shifts
    println!("{} {} {:?} {:?}", 1u8.wrapping_shl(9), 128u8.wrapping_shr(10), 1u32.checked_shl(32), 1u32.overflowing_shl(33));
    println!("{} {} {}", (-16i32) >> 2, (-16i32 as u32) >> 2, (-1i64).wrapping_shr(63));
    println!("{} {}", 0x80u8.rotate_left(1), 0x01u16.rotate_right(1));

    // pow
    println!("{:?} {:?} {} {}", 2u32.checked_pow(31), 2u32.checked_pow(32), 3u8.wrapping_pow(6), 10i16.saturating_pow(5));
    println!("{:?} {:?}", (-2i8).overflowing_pow(7), (-2i8).overflowing_pow(8));
    println!("{} {:?}", 2i64.pow(62), 2i64.checked_pow(63));
    println!("{:?} {:?} {:?}", 1000u32.checked_ilog10(), 0u32.checked_ilog2(), 255u8.checked_next_power_of_two());
    println!("{} {} {}", 17u32.next_power_of_two(), 64u32.is_power_of_two(), 100u64.ilog2());

    // Wrapping
    let mut w = Wrapping(250u8);
    w += Wrapping(10);
    let w2 = w * Wrapping(30);
    let w3 = -Wrapping(1i32 << 31);
    println!("{} {} {} {}", w, w2, w3, (Wrapping(0u16) - Wrapping(1)).0);
    let mut h = Wrapping(0xcbf29ce484222325u64);
    for b in b"hash me" {
        h ^= Wrapping(*b as u64);
        h *= Wrapping(0x100000001b3);
    }
    println!("fnv {:016x}", h.0);

    // mixed-width casts
    println!("{} {} {} {}", 300i32 as u8, -1i32 as u8, 255u8 as i8, -129i16 as i8);
    println!("{} {} {}", u64::MAX as i32, i64::MIN as u32, (1u64 << 40) as u16);
    println!("{} {} {}", -1i8 as u64, -1i8 as u8 as u64, 200u8 as i8 as i32);
    println!("{:?} {:?} {:?}", u8::try_from(256i32), i8::try_from(-128i64), u16::try_from(-1i8).is_err());

    let r = std::panic::catch_unwind(|| {
        let v: Vec<u32> = vec![u32::MAX];
        v[0].checked_add(1).expect("overflow detected")
    });
    println!("panicked: {}", r.is_err());
}
