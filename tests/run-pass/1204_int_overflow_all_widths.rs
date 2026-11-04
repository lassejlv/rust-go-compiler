macro_rules! probe {
    ($t:ty) => {{
        let a: $t = <$t>::MAX;
        let b: $t = <$t>::MIN;
        let one: $t = 1;
        let two: $t = 2;
        println!(
            "{}: wrap_add={} wrap_sub={} wrap_mul={} wrap_neg={} chk_add={:?} chk_sub={:?} chk_mul={:?} sat_add={} sat_sub={} sat_mul={}",
            stringify!($t),
            a.wrapping_add(one),
            b.wrapping_sub(one),
            a.wrapping_mul(two),
            b.wrapping_neg(),
            a.checked_add(one),
            b.checked_sub(one),
            (a / two).checked_mul(two),
            a.saturating_add(two),
            b.saturating_sub(two),
            a.saturating_mul(two)
        );
        println!(
            "  ovf_add={:?} ovf_sub={:?} ovf_mul={:?} ovf_neg={:?} shl={:?} shr={:?} chk_div={:?} chk_rem={:?} pow={:?} wpow={} cnt={} {} {} {}",
            a.overflowing_add(a),
            b.overflowing_sub(one),
            a.overflowing_mul(a),
            b.overflowing_neg(),
            one.overflowing_shl(70),
            a.overflowing_shr(3),
            a.checked_div(0 as $t),
            b.checked_rem(one),
            two.checked_pow(200),
            two.wrapping_pow(9),
            a.count_ones(),
            b.leading_zeros(),
            two.trailing_zeros(),
            a.rotate_left(5)
        );
        println!(
            "  swap={} rev={} be={} le={} min/max={}/{} div_euclid={} rem_euclid={} abs_diff={} ilog2={} isqrt-ish={}",
            (one << 3).swap_bytes(),
            one.reverse_bits(),
            a.to_be(),
            b.to_le(),
            b,
            a,
            (b / 3).div_euclid(7 as $t),
            (b / 3).rem_euclid(7 as $t),
            a.abs_diff(b),
            a.ilog2(),
            (100 as $t).pow(1)
        );
    }};
}

macro_rules! signed_extra {
    ($t:ty) => {{
        let b: $t = <$t>::MIN;
        let m: $t = -7;
        println!(
            "  {}: abs={:?} wabs={} uabs={} signum={} {} chk_neg={:?} sat_neg={} sat_abs={} is_neg={} div_euclid={} rem_euclid={} ovf_div={:?} wdiv={} chk_abs={:?}",
            stringify!($t),
            m.checked_abs(),
            b.wrapping_abs(),
            b.unsigned_abs(),
            m.signum(),
            (0 as $t).signum(),
            b.checked_neg(),
            b.saturating_neg(),
            b.saturating_abs(),
            m.is_negative(),
            m.div_euclid(2),
            m.rem_euclid(2),
            b.overflowing_div(-1),
            b.wrapping_div(-1),
            b.checked_abs()
        );
        println!("  m/2={} m%2={} m>>1={} m.pow(2)={}", m / 2, m % 2, m >> 1, m.pow(2));
    }};
}

fn main() {
    probe!(u8);
    probe!(u16);
    probe!(u32);
    probe!(u64);
    probe!(u128);
    probe!(usize);
    probe!(i8);
    signed_extra!(i8);
    probe!(i16);
    signed_extra!(i16);
    probe!(i32);
    signed_extra!(i32);
    probe!(i64);
    signed_extra!(i64);
    probe!(i128);
    signed_extra!(i128);
    probe!(isize);

    // i128/u128 arithmetic
    let big: u128 = 0xDEAD_BEEF_CAFE_BABE_1234_5678_9ABC_DEF0;
    println!("{} {:x} {:o} {:b}", big, big, big >> 64, big & 0xff);
    println!("{}", big.wrapping_mul(big));
    println!("{}", (big / 1_000_000_007) % 1_000_000_009);
    let s: i128 = -170141183460469231731687303715884105728;
    println!("{} {} {}", s, s + 1, (s + 1).abs());
    println!("{:?}", "340282366920938463463374607431768211455".parse::<u128>());
    println!("{:?}", "340282366920938463463374607431768211456".parse::<u128>().is_err());
    println!("{:?}", "-170141183460469231731687303715884105728".parse::<i128>());
    println!("{}", u128::MAX.count_ones() + i128::MIN.leading_zeros());
    println!("{}", (12345678901234567890123456789u128).to_string().len());
    println!("{:>45}|", i128::MIN);
    println!("{:+e}", 1234567u128 as f64);

    // from_str_radix and to string radix
    println!("{:?} {:?} {:?}", i32::from_str_radix("-ff", 16), u8::from_str_radix("100000000", 2), i64::from_str_radix("zz", 36));
    println!("{:?}", u64::from_str_radix("", 10).is_err());
    println!("{:?} {:?}", "+12".parse::<u8>(), "-0".parse::<i8>());
    println!("{:?}", "256".parse::<u8>().map_err(|e| e.to_string()));
    println!("{:?}", "-129".parse::<i8>().map_err(|e| e.to_string()));
    println!("{:?}", "".parse::<i8>().map_err(|e| e.to_string()));
    println!("{:?}", "1x".parse::<i8>().map_err(|e| e.to_string()));

    // TryFrom
    use std::convert::TryFrom;
    println!("{:?} {:?} {:?}", u8::try_from(300i32).is_err(), i8::try_from(-128i64), u32::try_from(-1i8).is_err());
    println!("{:?} {:?}", u64::try_from(u128::MAX).map_err(|e| e.to_string()), i128::try_from(u128::MAX).is_err());
    println!("{:?}", usize::try_from(5u64));
    let x: i16 = 200u8.into();
    println!("{}", x + i16::from(true) + i16::from(7u8));
}
