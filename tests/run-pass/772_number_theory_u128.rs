// Number theory with u64/u128: modular multiplication via u128, deterministic
// Miller-Rabin, Pollard's rho factorization (fixed seeds), extended gcd,
// modular inverse, CRT, Euler's totient, and formatting of big results.
fn mulmod(a: u64, b: u64, m: u64) -> u64 { ((a as u128 * b as u128) % m as u128) as u64 }
fn powmod(mut b: u64, mut e: u64, m: u64) -> u64 {
    let mut r = 1 % m;
    b %= m;
    while e > 0 { if e & 1 == 1 { r = mulmod(r, b, m); } b = mulmod(b, b, m); e >>= 1; }
    r
}
fn is_prime(n: u64) -> bool {
    if n < 2 { return false; }
    for p in [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] { if n % p == 0 { return n == p; } }
    let (mut d, mut s) = (n - 1, 0);
    while d % 2 == 0 { d /= 2; s += 1; }
    'w: for a in [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        let mut x = powmod(a, d, n);
        if x == 1 || x == n - 1 { continue; }
        for _ in 1..s { x = mulmod(x, x, n); if x == n - 1 { continue 'w; } }
        return false;
    }
    true
}
fn gcd(a: u64, b: u64) -> u64 { if b == 0 { a } else { gcd(b, a % b) } }
fn rho(n: u64, seed: u64) -> u64 {
    if n % 2 == 0 { return 2; }
    let f = |x: u64| (mulmod(x, x, n) + seed) % n;
    let (mut x, mut y, mut d) = (2u64, 2u64, 1u64);
    while d == 1 { x = f(x); y = f(f(y)); d = gcd(x.abs_diff(y), n); }
    d
}
fn factor(n: u64, out: &mut Vec<u64>) {
    if n == 1 { return; }
    if is_prime(n) { out.push(n); return; }
    for p in [2u64, 3, 5, 7] { if n % p == 0 { out.push(p); return factor(n / p, out); } }
    let mut seed = 1;
    loop {
        let d = rho(n, seed);
        if d != n { factor(d, out); factor(n / d, out); return; }
        seed += 1;
    }
}
fn egcd(a: i128, b: i128) -> (i128, i128, i128) { if b == 0 { (a, 1, 0) } else { let (g, x, y) = egcd(b, a % b); (g, y, x - (a / b) * y) } }
fn modinv(a: u64, m: u64) -> Option<u64> {
    let (g, x, _) = egcd(a as i128, m as i128);
    if g != 1 { None } else { Some(x.rem_euclid(m as i128) as u64) }
}
fn crt(rs: &[(u64, u64)]) -> Option<(u128, u128)> {
    let mut x: u128 = 0;
    let mut m: u128 = 1;
    for &(r, mi) in rs {
        let mi = mi as u128;
        let inv = modinv((m % mi) as u64, mi as u64)? as u128;
        let t = ((r as u128 + mi - x % mi) % mi) * inv % mi;
        x += m * t;
        m *= mi;
    }
    Some((x % m, m))
}
fn totient(n: u64) -> u64 {
    let mut fs = vec![];
    factor(n, &mut fs);
    fs.sort();
    fs.dedup();
    fs.iter().fold(n, |acc, p| acc / p * (p - 1))
}

fn main() {
    let candidates = [1u64, 2, 97, 561, 7919, 1_000_000_007, 4_294_967_291, 4_294_967_297, 999_999_999_989, 18_446_744_073_709_551_557, 3_215_031_751];
    for c in candidates { print!("{}:{} ", c, if is_prime(c) { 'P' } else { 'c' }); }
    println!();
    for n in [360u64, 1_234_567_890, 600_851_475_143, 999_999_000_001, 18_446_744_073_709_551_615, 2u64.pow(61) - 1, 1_000_000_016_000_000_063] {
        let mut f = vec![];
        factor(n, &mut f);
        f.sort();
        let check: u128 = f.iter().map(|&x| x as u128).product();
        println!("{} = {:?} (ok={})", n, f, check == n as u128);
    }
    println!("powmod {} {} {}", powmod(2, 1_000_000, 1_000_000_007), powmod(3, u64::MAX, 18_446_744_073_709_551_557), powmod(10, 0, 1));
    println!("modinv {:?} {:?} {:?}", modinv(3, 11), modinv(10, 1_000_000_007), modinv(6, 9));
    println!("crt {:?}", crt(&[(2, 3), (3, 5), (2, 7)]));
    println!("crt big {:?}", crt(&[(1, 1_000_000_007), (2, 998_244_353), (3, 999_999_937)]));
    println!("crt non-coprime {:?}", crt(&[(1, 4), (3, 6)]));
    println!("totients {:?}", [1u64, 9, 36, 97, 1_000_000, 600_851_475_143].map(totient));
    let big: u128 = (1..=30u128).product();
    println!("30! = {} hex {:x} bits {} trailing zeros {}", big, big, 128 - big.leading_zeros(), big.trailing_zeros());
    println!("u128 ops: {} {} {:?} {:?}", u128::MAX / 3, (u128::MAX >> 64) as u64, 7u128.checked_pow(45), 7u128.checked_pow(46));
    println!("i128: {} {} {}", i128::MIN, (i128::MIN + 1).abs(), (-17i128).rem_euclid(5));
    let sq = |n: u128| { let mut x = (n as f64).sqrt() as u128; while x * x > n { x -= 1; } while (x + 1) * (x + 1) <= n { x += 1; } x };
    println!("isqrt u128: {} {}", sq(u64::MAX as u128 * 1000), sq(10u128.pow(30) + 12345));
}
