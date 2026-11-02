// 128-bit integer arithmetic: mul/div/rem across the 64-bit boundary, shifts by >64,
// leading/trailing zeros, count_ones, byte conversions, parsing/printing extremes, widening multiply
// emulation, factorials, and a u128-based fixed-point type.
use std::fmt;

#[derive(Clone, Copy, PartialEq, PartialOrd)]
struct Fixed(i128); // 64.64 fixed point

const ONE: i128 = 1 << 64;

impl Fixed {
    fn from_int(n: i64) -> Fixed {
        Fixed((n as i128) << 64)
    }
    fn from_ratio(a: i64, b: i64) -> Fixed {
        Fixed(((a as i128) << 64) / b as i128)
    }
    fn mul(self, o: Fixed) -> Fixed {
        // split to avoid overflow: (a*b) >> 64
        let a = self.0;
        let b = o.0;
        let neg = (a < 0) ^ (b < 0);
        let (ua, ub) = (a.unsigned_abs(), b.unsigned_abs());
        let (ah, al) = (ua >> 64, ua & 0xFFFF_FFFF_FFFF_FFFF);
        let (bh, bl) = (ub >> 64, ub & 0xFFFF_FFFF_FFFF_FFFF);
        let r = (ah * bh << 64) + ah * bl + al * bh + ((al * bl) >> 64);
        Fixed(if neg { -(r as i128) } else { r as i128 })
    }
}

impl fmt::Display for Fixed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let neg = self.0 < 0;
        let v = self.0.unsigned_abs();
        let int = v >> 64;
        let frac = v & (ONE as u128 - 1);
        // 6 decimal digits
        let dec = (frac * 1_000_000 + (1u128 << 63)) >> 64;
        write!(f, "{}{}.{:06}", if neg { "-" } else { "" }, int, dec)
    }
}

fn mul_wide(a: u64, b: u64) -> (u64, u64) {
    let p = a as u128 * b as u128;
    ((p >> 64) as u64, p as u64)
}

fn main() {
    println!("{} {}", u128::MAX, i128::MAX);
    println!("{} {}", i128::MIN, i128::MIN + 1);
    let a: u128 = 0xFFFF_FFFF_FFFF_FFFF;
    let b = a * a;
    println!("a*a = {} = {:#x}", b, b);
    println!("b / 3 = {} rem {}", b / 3, b % 3);
    println!("b / a = {} rem {}", b / a, b % a);
    let big = 340282366920938463463374607431768211455u128;
    println!("{} {}", big / 1_000_000_007, big % 1_000_000_007);
    let s: i128 = -170141183460469231731687303715884105728;
    println!("{} {} {}", s / 7, s % 7, s.rem_euclid(7));
    println!("{:?} {:?}", s.checked_neg(), s.checked_div(-1));
    println!("{:?} {:?}", u128::MAX.checked_add(1), u128::MAX.overflowing_mul(2));
    println!("{} {}", i128::MAX.wrapping_add(1) == i128::MIN, u128::MAX.wrapping_mul(u128::MAX));

    for sh in [0u32, 63, 64, 65, 100, 127] {
        let v = 1u128 << sh;
        println!("1<<{:3} = {:>40} lz={:3} tz={:3}", sh, v, v.leading_zeros(), v.trailing_zeros());
    }
    let neg: i128 = -1 << 100;
    println!("{} {} {}", neg, neg >> 99, (neg as u128) >> 120);
    println!("ones {} {}", u128::MAX.count_ones(), (0x5555_5555_5555_5555_5555u128).count_ones());
    println!("swap {:#x}", 0x0102030405060708090a0b0c0d0e0f10u128.swap_bytes());
    println!("rev bits {:#x}", 1u128.reverse_bits());
    let bytes = 0x1122334455667788_99aabbccddeeff00u128.to_be_bytes();
    println!("be {:02x?}", &bytes[..4]);
    let back = u128::from_le_bytes(bytes);
    println!("le back {:#x}", back);

    println!("{:?}", "123456789012345678901234567890".parse::<u128>());
    println!("{:?}", "-170141183460469231731687303715884105728".parse::<i128>());
    println!("{:?}", "340282366920938463463374607431768211456".parse::<u128>().map_err(|e| e.to_string()));
    println!("{:?}", u128::from_str_radix("ffffffffffffffffffffffffffffffff", 16));
    println!("{:?}", i128::from_str_radix("-1000000000000000000000000000000000000000", 2));

    let mut f: u128 = 1;
    for i in 1..=34u128 {
        f *= i;
    }
    println!("34! = {}", f);
    println!("35! fits? {:?}", f.checked_mul(35));

    println!("{:?}", mul_wide(u64::MAX, u64::MAX));
    println!("{:?}", mul_wide(0xdeadbeef, 0xcafebabe12345678));

    let x = Fixed::from_ratio(1, 3);
    let y = Fixed::from_int(-7);
    println!("{} {} {}", x, y, x.mul(y));
    let pi = Fixed::from_ratio(355, 113);
    println!("pi~{} pi^2~{}", pi, pi.mul(pi));
    println!("cmp {} {}", x < pi, y > x);

    println!("{} {}", (u64::MAX as u128 + 1) as f64, i128::MIN as f64);
    println!("{} {}", 1e30f64 as u128, -1.5e20f64 as i128);
    println!("{} {}", f64::MAX as u128 == u128::MAX, -1.0f64 as u128);
    println!("{}", (12345678901234567890123u128 as f32) as u128);
    let parts: Vec<u32> = (0..4).map(|i| (big >> (i * 32)) as u32).collect();
    println!("{:?}", parts);
    println!("{} {}", i128::MAX.abs_diff(i128::MIN), (-5i128).pow(27));
}
