// i128/u128 arithmetic and formatting: Display/Debug/hex/padding, parsing, mul/div, conversions
fn fact(n: u32) -> u128 {
    (1..=n as u128).product()
}

fn isqrt128(n: u128) -> u128 {
    if n < 2 {
        return n;
    }
    let mut x = n;
    let mut y = x / 2 + x % 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

fn mulmod(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128 * b as u128) % m as u128) as u64
}

fn powmod(mut b: u64, mut e: u64, m: u64) -> u64 {
    let mut r = 1u64;
    b %= m;
    while e > 0 {
        if e & 1 == 1 {
            r = mulmod(r, b, m);
        }
        b = mulmod(b, b, m);
        e >>= 1;
    }
    r
}

fn main() {
    println!("{}", u128::MAX);
    println!("{}", i128::MIN);
    println!("{}", i128::MAX);
    println!("{:?}", -1i128);
    for n in [20, 25, 30, 34] {
        println!("{}! = {}", n, fact(n));
    }
    println!("{:?}", fact(34).checked_mul(35));
    let big: u128 = 0xDEAD_BEEF_CAFE_BABE_1234_5678_9ABC_DEF0;
    println!("{:x}", big);
    println!("{:#X}", big);
    println!("{:o}", big >> 64);
    println!("{:>45}|", big);
    println!("{:<45}|", big);
    println!("{:045}", big);
    println!("{:+}", big as i128);
    println!("{:e}", big);
    println!("{}", big / 1_000_000_007);
    println!("{}", big % 1_000_000_007);
    println!("{}", (big as i128).wrapping_neg());
    println!("{}", i128::MIN / 3);
    println!("{}", i128::MIN % 7);
    println!("{}", (-170141183460469231731687303715884105727i128).rem_euclid(1000));
    println!("{}", isqrt128(u128::MAX));
    println!("{}", isqrt128(10u128.pow(30)));
    println!("{}", powmod(2, 1_000_000, 1_000_000_007));
    println!("{}", powmod(123456789, 987654321, 998244353));
    println!("{}", mulmod(u64::MAX, u64::MAX, u64::MAX - 1));
    let parsed: i128 = "-98765432109876543210987654321".parse().unwrap();
    println!("{} {}", parsed, parsed * 3);
    println!("{:?}", "340282366920938463463374607431768211456".parse::<u128>().map_err(|e| e.to_string()));
    println!("{:?}", u128::from_str_radix("ffffffffffffffffffffffffffffffff", 16));
    println!("{}", u128::MAX.count_ones() + (1u128 << 127).leading_zeros() + (1u128 << 100).trailing_zeros());
    println!("{}", (big as u64) as u128 + (big >> 96));
    println!("{}", i128::from(i64::MIN) * 2);
    println!("{:?}", i64::try_from(i128::from(i64::MAX) + 1).is_err());
    println!("{:?}", u8::try_from(255u128));
    println!("{}", (1u128 << 64) as f64);
    println!("{}", 1e30f64 as u128);
    println!("{}", -1e30f64 as i128);
    println!("{}", f64::INFINITY as u128 == u128::MAX);
    println!("{}", (u128::MAX as f32));
    let fibs: Vec<u128> = std::iter::successors(Some((0u128, 1u128)), |&(a, b)| a.checked_add(b).map(|s| (b, s))).map(|p| p.0).collect();
    println!("{} {}", fibs.len(), fibs.last().unwrap());
    let digits: u32 = fibs.last().unwrap().to_string().chars().map(|c| c.to_digit(10).unwrap()).sum();
    println!("{}", digits);
    let mut v: Vec<i128> = vec![i128::MAX, -5, 0, i128::MIN, 1 << 100];
    v.sort();
    println!("{:?}", v);
    println!("{}", v.iter().map(|x| x / (1i128 << 120)).sum::<i128>());
    println!("{}", 12345678901234567890123456789u128.reverse_bits() >> 100);
    println!("{:?}", 3u128.overflowing_mul(u128::MAX / 2));
    println!("{}", i128::MAX.abs_diff(i128::MIN));
    println!("{}", u128::MAX.ilog10());
    println!("{}", 255u128.ilog2());
}
