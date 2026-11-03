fn mulmod(a: u64, b: u64, m: u64) -> u64 { ((a as u128 * b as u128) % m as u128) as u64 }
fn powmod(mut b: u64, mut e: u64, m: u64) -> u64 { let mut r = 1 % m; b %= m; while e > 0 { if e & 1 == 1 { r = mulmod(r, b, m); } b = mulmod(b, b, m); e >>= 1; } r }
fn is_prime(n: u64) -> bool {
    if n < 2 { return false; }
    for p in [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] { if n % p == 0 { return n == p; } }
    let (mut d, mut s) = (n - 1, 0);
    while d % 2 == 0 { d /= 2; s += 1; }
    'outer: for a in [2u64, 325, 9375, 28178, 450775, 9780504, 1795265022] {
        let a = a % n;
        if a == 0 { continue; }
        let mut x = powmod(a, d, n);
        if x == 1 || x == n - 1 { continue; }
        for _ in 1..s { x = mulmod(x, x, n); if x == n - 1 { continue 'outer; } }
        return false;
    }
    true
}
fn gcd(a: u64, b: u64) -> u64 { if b == 0 { a } else { gcd(b, a % b) } }
fn pollard_rho(n: u64) -> u64 {
    if n % 2 == 0 { return 2; }
    let mut c = 1;
    loop {
        let f = |x: u64| (mulmod(x, x, n) + c) % n;
        let (mut x, mut y, mut d) = (2u64, 2u64, 1u64);
        while d == 1 { x = f(x); y = f(f(y)); d = gcd(x.abs_diff(y), n); }
        if d != n { return d; }
        c += 1;
    }
}
fn factor(n: u64, out: &mut Vec<u64>) {
    if n == 1 { return; }
    if is_prime(n) { out.push(n); return; }
    let d = pollard_rho(n);
    factor(d, out); factor(n / d, out);
}
fn ext_gcd(a: i128, b: i128) -> (i128, i128, i128) { if b == 0 { (a, 1, 0) } else { let (g, x, y) = ext_gcd(b, a % b); (g, y, x - (a / b) * y) } }
fn mod_inv(a: i128, m: i128) -> Option<i128> { let (g, x, _) = ext_gcd(a.rem_euclid(m), m); if g != 1 { None } else { Some(x.rem_euclid(m)) } }
fn crt(rems: &[(i128, i128)]) -> Option<(i128, i128)> {
    let mut x = 0i128; let mut m = 1i128;
    for &(r, mi) in rems {
        let (g, p, _) = ext_gcd(m, mi);
        if (r - x) % g != 0 { return None; }
        let l = m / g * mi;
        x = (x + (r - x) / g % (mi / g) * p % (mi / g) * m).rem_euclid(l);
        m = l;
    }
    Some((x, m))
}
type M2 = [[u64; 2]; 2];
fn matmul(a: M2, b: M2, m: u64) -> M2 {
    let mut c = [[0; 2]; 2];
    for i in 0..2 { for j in 0..2 { for k in 0..2 { c[i][j] = (c[i][j] + mulmod(a[i][k], b[k][j], m)) % m; } } }
    c
}
fn fib_mod(n: u64, m: u64) -> u64 { let mut r: M2 = [[1, 0], [0, 1]]; let mut b: M2 = [[1, 1], [1, 0]]; let mut e = n; while e > 0 { if e & 1 == 1 { r = matmul(r, b, m); } b = matmul(b, b, m); e >>= 1; } r[0][1] }
fn totient(n: u64) -> u64 { let mut f = vec![]; factor(n, &mut f); f.sort(); f.dedup(); f.iter().fold(n, |acc, p| acc / p * (p - 1)) }
fn isqrt_u128(n: u128) -> u128 { if n < 2 { return n; } let mut x = (n as f64).sqrt() as u128; while x * x > n { x -= 1; } while (x + 1) * (x + 1) <= n { x += 1; } x }

fn main() {
    let candidates = [1u64, 2, 97, 561, 7919, 1_000_000_007, 4_294_967_297, 999_999_999_989, 18_446_744_073_709_551_557, 3_215_031_751, 341_550_071_728_321];
    for n in candidates { print!("{}:{} ", n, is_prime(n)); }
    println!();
    for n in [360u64, 1_234_567_890, 600_851_475_143, 4_294_967_297, 999_999_000_001, 9_007_199_254_740_993, 1 << 40] {
        let mut f = vec![]; factor(n, &mut f); f.sort();
        println!("{} = {:?} phi={}", n, f, totient(n));
    }
    println!("{} {} {}", powmod(2, 1_000_000, 1_000_000_007), powmod(3, u64::MAX, 998_244_353), powmod(7, 0, 1));
    println!("{:?} {:?} {:?}", mod_inv(3, 11), mod_inv(10, 17), mod_inv(6, 9));
    println!("{:?}", crt(&[(2, 3), (3, 5), (2, 7)]));
    println!("{:?}", crt(&[(1, 4), (3, 6)]));
    println!("{:?}", crt(&[(1, 4), (2, 6)]));
    for n in [10u64, 90, 1000, 1_000_000_000_000] { print!("F{} mod 1e9+7 = {}; ", n, fib_mod(n, 1_000_000_007)); }
    println!();
    println!("{} {} {}", isqrt_u128(u64::MAX as u128 * 3), isqrt_u128(10u128.pow(30) + 12345), isqrt_u128(99));
    let primes: Vec<u64> = (1..200).filter(|&n| is_prime(n)).collect();
    let twin = primes.windows(2).filter(|w| w[1] - w[0] == 2).count();
    println!("{} primes < 200, {} twin pairs, last {:?}", primes.len(), twin, primes.last());
    let goldbach: Vec<(u64, u64)> = (4..=40).step_by(6).map(|n| { let p = primes.iter().find(|&&p| is_prime(n - p)).unwrap(); (*p, n - p) }).collect();
    println!("{:?}", goldbach);
    println!("{}", (1..=30u128).fold(1u128, |a, b| a * b / { let mut x = a; let mut y = b; while y != 0 { let t = y; y = x % y; x = t; } x }));
}
