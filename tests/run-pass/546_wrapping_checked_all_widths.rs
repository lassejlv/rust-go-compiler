// wrapping/checked/saturating/overflowing ops on all integer widths via macro
macro_rules! ops_for {
    ($($t:ty),*) => {$(
        {
            let a: $t = <$t>::MAX;
            let b: $t = <$t>::MIN;
            let one: $t = 1;
            let two: $t = 2;
            println!("== {} ==", stringify!($t));
            println!("wrap add {} sub {} mul {}", a.wrapping_add(one), b.wrapping_sub(one), a.wrapping_mul(two));
            println!("wrap neg {} shl {} shr {}", b.wrapping_neg(), one.wrapping_shl(<$t>::BITS + 1), a.wrapping_shr(<$t>::BITS * 2 - 1));
            println!("checked add {:?} sub {:?} mul {:?} div {:?} rem {:?}", a.checked_add(one), b.checked_sub(one), a.checked_mul(two), a.checked_div(0), b.checked_rem(two));
            println!("checked neg {:?} shl {:?} pow {:?}", b.checked_neg(), one.checked_shl(<$t>::BITS), two.checked_pow(<$t>::BITS - 2));
            println!("sat add {} sub {} mul {} pow {}", a.saturating_add(one), b.saturating_sub(one), a.saturating_mul(two), two.saturating_pow(200));
            println!("ovf add {:?} sub {:?} mul {:?} neg {:?}", a.overflowing_add(one), b.overflowing_sub(one), a.overflowing_mul(two), b.overflowing_neg());
            println!("ovf pow {:?} shl {:?}", two.overflowing_pow(<$t>::BITS), one.overflowing_shl(<$t>::BITS + 3));
            println!("bits: lz {} tz {} ones {} zeros {}", a.leading_zeros(), b.trailing_zeros(), a.count_ones(), b.count_zeros());
            println!("rotl {} rotr {} swap {} rev {}", one.rotate_left(3), one.rotate_right(1), (a / 3).swap_bytes(), one.reverse_bits());
            println!("pow {} div_euclid {} rem_euclid {}", two.pow(5), (a - 6).div_euclid(7), (b + 3).rem_euclid(5));
        }
    )*};
}

macro_rules! signed_extra {
    ($($t:ty),*) => {$(
        {
            let m: $t = <$t>::MIN;
            let x: $t = -7;
            println!("{}: abs {:?} wabs {} uabs {} sig {} {} {}", stringify!($t), m.checked_abs(), m.wrapping_abs(), m.unsigned_abs(), x.signum(), (0 as $t).signum(), (5 as $t).signum());
            println!("  div_euclid {} rem_euclid {} / {} % {}", x.div_euclid(2), x.rem_euclid(2), x / 2, x % 2);
            println!("  sat neg {} sat abs {} ovf abs {:?} is_neg {}", m.saturating_neg(), m.saturating_abs(), m.overflowing_abs(), x.is_negative());
            println!("  abs_diff {} {}", m.abs_diff(<$t>::MAX), x.abs_diff(7));
        }
    )*};
}

fn main() {
    ops_for!(u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, usize, isize);
    signed_extra!(i8, i16, i32, i64, i128);
    println!("{}", 200u8.wrapping_add(100).wrapping_mul(3));
    let mut h: u32 = 2166136261;
    for b in b"hello" {
        h ^= *b as u32;
        h = h.wrapping_mul(16777619);
    }
    println!("fnv1a {:#010x}", h);
    let mut x: u64 = 88172645463325252;
    let mut outs = vec![];
    for _ in 0..4 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        outs.push(x % 1000);
    }
    println!("xorshift {:?}", outs);
    let (lo, carry) = u64::MAX.overflowing_add(5);
    println!("{} {}", lo, carry);
    println!("{:?}", 10u32.checked_next_power_of_two());
    println!("{} {}", 17u32.next_power_of_two(), 64u64.is_power_of_two());
    println!("{} {}", 7u32.div_ceil(2), 100i32.midpoint(-50));
    println!("{:?} {:?}", 5u8.checked_sub(5), 0u8.checked_sub(1));
    println!("{}", i32::MAX.wrapping_add(1) == i32::MIN);
    println!("{:?}", 1u32.checked_add_signed(-2));
    println!("{:?}", 10u8.saturating_add_signed(-20));
    println!("{}", 5i32.wrapping_sub_unsigned(10));
    println!("{:?}", 300i32.checked_ilog10());
    let mut acc = 0u8;
    let mut carries = 0;
    for b in [200u8, 100, 50, 250, 1] {
        let (s, c) = acc.overflowing_add(b);
        acc = s;
        carries += c as u32;
    }
    println!("acc {} carries {}", acc, carries);
    let big = [u64::MAX, u64::MAX, 2];
    let (mut lo, mut hi) = (0u64, 0u64);
    for &x in &big {
        let (s, c) = lo.overflowing_add(x);
        lo = s;
        hi += c as u64;
    }
    println!("128-bit sum hi={} lo={}", hi, lo);
    println!("{}", ((hi as u128) << 64 | lo as u128) == big.iter().map(|&x| x as u128).sum::<u128>());
    println!("{:?}", i8::MIN.checked_div(-1));
    println!("{:?}", i8::MIN.overflowing_div(-1));
    println!("{}", i8::MIN.wrapping_div(-1));
    println!("{:?}", i8::MIN.overflowing_rem(-1));
    println!("{}", i16::MIN.saturating_div(-1));
}
