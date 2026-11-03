macro_rules! probe {
    ($t:ty) => {{
        let a: $t = <$t>::MAX - 3;
        let b: $t = 7;
        let mn: $t = <$t>::MIN;
        println!("{:>5}: max={} min={} bits={}", stringify!($t), <$t>::MAX, mn, <$t>::BITS);
        println!("  checked_add {:?} checked_sub {:?} checked_mul {:?} checked_div0 {:?}",
            a.checked_add(b), mn.checked_sub(1), a.checked_mul(2), b.checked_div(0));
        println!("  wrapping add {} sub {} mul {} neg {} shl {}",
            a.wrapping_add(b), mn.wrapping_sub(1), a.wrapping_mul(3), b.wrapping_neg(), b.wrapping_shl(<$t>::BITS + 1));
        println!("  saturating add {} sub {} mul {}",
            a.saturating_add(b), mn.saturating_sub(b), a.saturating_mul(2));
        println!("  overflowing add {:?} sub {:?} mul {:?} neg {:?}",
            a.overflowing_add(b), mn.overflowing_sub(1), a.overflowing_mul(2), mn.overflowing_neg());
        println!("  pow {:?} {:?} {} {}", b.checked_pow(5), b.checked_pow(60), b.wrapping_pow(40), b.saturating_pow(70));
        println!("  ones={} lz={} tz={} rotl={} swap={} rev={}",
            a.count_ones(), b.leading_zeros(), (b as $t).wrapping_shl(3).trailing_zeros(), b.rotate_left(3), b.swap_bytes(), b.reverse_bits());
        println!("  rem_euclid {} div_euclid {} checked_rem {:?} {:?}", (b).rem_euclid(3), a.div_euclid(5), a.checked_rem(0), mn.checked_rem(<$t>::MAX));
    }};
}

macro_rules! signed_extra {
    ($t:ty) => {{
        let mn = <$t>::MIN;
        let x: $t = -17;
        println!("  [signed {}] abs {:?} wabs {} uabs {} sign {} {} div_euclid {} rem_euclid {} checked_div(min,-1) {:?} wrapping_div {} overflowing_div {:?} checked_neg {:?} sat_neg {}",
            stringify!($t), mn.checked_abs(), mn.wrapping_abs(), mn.unsigned_abs(), x.signum(), (0 as $t).signum(),
            x.div_euclid(5), x.rem_euclid(5), mn.checked_div(-1), mn.wrapping_div(-1), mn.overflowing_div(-1), mn.checked_neg(), mn.saturating_neg());
        println!("  is_negative {} is_positive {} abs_diff {} pow {}", x.is_negative(), x.is_positive(), x.abs_diff(<$t>::MAX), (x / 4).pow(3));
    }};
}

fn main() {
    probe!(u8); probe!(u16); probe!(u32); probe!(u64); probe!(u128); probe!(usize);
    probe!(i8); signed_extra!(i8);
    probe!(i16); signed_extra!(i16);
    probe!(i32); signed_extra!(i32);
    probe!(i64); signed_extra!(i64);
    probe!(i128); signed_extra!(i128);
    probe!(isize); signed_extra!(isize);

    // u128/i128 heavy arithmetic
    let f: u128 = (1..=30u128).product();
    println!("30! = {}", f);
    println!("30! hex = {:#x}", f);
    println!("30! / 7! = {} rem {}", f / 5040, f % 1_000_000_007);
    let big: i128 = -170141183460469231731687303715884105728;
    println!("{} {} {:?}", big, big + 1, big.checked_mul(-1));
    let m: u128 = 0xFFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF;
    println!("{} {} {}", m.count_ones(), (m >> 64) as u64, (m as u64).wrapping_add(1));
    let sq = ((u64::MAX as u128) * (u64::MAX as u128)).to_string();
    let (hi, lo) = sq.split_at(10);
    println!("{} {}", hi, lo);
    let v: i128 = i64::MIN as i128 * 3;
    println!("{} {} {}", v, v as i64, v as u8);
    // casts truncating and sign-extending
    println!("{} {} {} {} {}", 300i32 as u8, -1i32 as u32, -1i8 as u128, 255u8 as i8, (1u64 << 40) as i32);
    println!("{} {} {} {}", 3.99f64 as i32, -3.99f64 as i32, 1e20f64 as i32, f64::NAN as u8);
    println!("{} {} {}", -1e10f64 as u64, 1e40f64 as u128, 255.9f32 as u8);
    // from_str_radix and to/from bytes
    println!("{:?} {:?} {:?}", i64::from_str_radix("-zz", 36), u8::from_str_radix("100000000", 2).is_err(), u32::from_str_radix("ff_ff", 16).is_err());
    println!("{:?} {:?}", 0x1234_5678u32.to_be_bytes(), 0x1234_5678u32.to_le_bytes());
    println!("{} {}", u32::from_be_bytes([0xde, 0xad, 0xbe, 0xef]), i16::from_le_bytes([0xff, 0x7f]));
    println!("{:?} {:?}", 200u8.checked_next_power_of_two(), 100u32.next_power_of_two());
    println!("{} {} {}", 64u32.is_power_of_two(), 17u64.ilog2(), 1000u32.ilog10());
}
