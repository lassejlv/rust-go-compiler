// u128/i128 arithmetic: mulmod via u128, modular exponentiation, Miller-Rabin, factorials,
// 128-bit hashing (FNV-1a 128), wide multiplication (u64 x u64 -> u128), and formatting.

fn mul_mod(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128 * b as u128) % m as u128) as u64
}

fn pow_mod(mut b: u64, mut e: u64, m: u64) -> u64 {
    let mut r = 1u64 % m;
    b %= m;
    while e > 0 {
        if e & 1 == 1 {
            r = mul_mod(r, b, m);
        }
        b = mul_mod(b, b, m);
        e >>= 1;
    }
    r
}

fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    for p in [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        if n % p == 0 {
            return n == p;
        }
    }
    let mut d = n - 1;
    let mut s = 0;
    while d % 2 == 0 {
        d /= 2;
        s += 1;
    }
    'witness: for a in [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        let mut x = pow_mod(a, d, n);
        if x == 1 || x == n - 1 {
            continue;
        }
        for _ in 0..s - 1 {
            x = mul_mod(x, x, n);
            if x == n - 1 {
                continue 'witness;
            }
        }
        return false;
    }
    true
}

fn factorial(n: u32) -> Option<u128> {
    (1..=n as u128).try_fold(1u128, |acc, x| acc.checked_mul(x))
}

fn fnv1a_128(data: &[u8]) -> u128 {
    const OFFSET: u128 = 0x6c62272e07bb014262b821756295c58d;
    const PRIME: u128 = 0x0000000001000000000000000000013B;
    let mut h = OFFSET;
    for b in data {
        h ^= *b as u128;
        h = h.wrapping_mul(PRIME);
    }
    h
}

fn wide_mul(a: u64, b: u64) -> (u64, u64) {
    let p = a as u128 * b as u128;
    ((p >> 64) as u64, p as u64)
}

fn isqrt(n: u128) -> u128 {
    if n < 2 {
        return n;
    }
    let mut x = 1u128 << ((128 - n.leading_zeros() + 1) / 2);
    loop {
        let y = (x + n / x) / 2;
        if y >= x {
            return x;
        }
        x = y;
    }
}

fn gcd(a: u128, b: u128) -> u128 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

fn main() {
    println!("{}", u128::MAX);
    println!("{}", i128::MIN);
    println!("{:x}", u128::MAX / 3);
    println!("{:#b}", 1u128 << 100);
    println!("{:>45}|", i128::MAX);
    println!("{:040}", 12345678901234567890123u128);

    println!("{}", mul_mod(u64::MAX - 1, u64::MAX - 2, 1_000_000_007));
    println!("{}", pow_mod(2, 1_000_000, 1_000_000_007));
    println!("{}", pow_mod(3, u64::MAX, 18446744073709551557));
    let candidates = [1u64, 2, 97, 561, 1_000_000_007, 18446744073709551557, 18446744073709551559, 4_294_967_297, 2_305_843_009_213_693_951];
    for c in candidates {
        println!("is_prime({}) = {}", c, is_prime(c));
    }
    for n in [0, 1, 20, 25, 34, 35] {
        println!("{}! = {:?}", n, factorial(n));
    }
    for s in ["", "a", "foobar", "The quick brown fox"] {
        println!("fnv128({:?}) = {:032x}", s, fnv1a_128(s.as_bytes()));
    }
    let (hi, lo) = wide_mul(0xdead_beef_cafe_babe, 0x1234_5678_9abc_def0);
    println!("wide = {:016x}{:016x}", hi, lo);
    println!("isqrt: {} {} {} {}", isqrt(0), isqrt(99), isqrt(1u128 << 100), isqrt(u128::MAX));
    let a: u128 = 2u128.pow(64) * 3 * 7;
    let b: u128 = 2u128.pow(70) * 7 * 11;
    println!("gcd = {} ({})", gcd(a, b), gcd(a, b) == 2u128.pow(64) * 7);
    let x: i128 = -170141183460469231731687303715884105728;
    println!("{:?} {:?} {}", x.checked_neg(), x.checked_abs(), x.wrapping_neg() == x);
    println!("{} {}", (x / 7) * 7 + x % 7 == x, x.rem_euclid(7));
    println!("{:?}", u128::from_str_radix("ffffffffffffffffffffffffffffffff", 16));
    println!("{:?}", "340282366920938463463374607431768211456".parse::<u128>().map_err(|e| e.to_string()));
    println!("{} {} {}", (1u128 << 127).leading_zeros(), u128::MAX.count_ones(), (12345u128 << 64).trailing_zeros());
    let f = (1u128 << 80) as f64;
    println!("{} {}", f, (f * 3.0) as u128);
    let bytes = 0x0102030405060708090a0b0c0d0e0f10u128.to_be_bytes();
    println!("{:?}", &bytes[..4]);
    println!("{}", u128::from_le_bytes(bytes) == 0x100f0e0d0c0b0a090807060504030201);
}
