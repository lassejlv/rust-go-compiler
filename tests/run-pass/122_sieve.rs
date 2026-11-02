fn sieve(n: usize) -> Vec<usize> {
    let mut is_prime = vec![true; n + 1];
    is_prime[0] = false;
    if n >= 1 {
        is_prime[1] = false;
    }
    let mut i = 2;
    while i * i <= n {
        if is_prime[i] {
            let mut j = i * i;
            while j <= n {
                is_prime[j] = false;
                j += i;
            }
        }
        i += 1;
    }
    is_prime.iter().enumerate().filter(|(_, &p)| p).map(|(i, _)| i).collect()
}

fn factorize(mut n: u64) -> Vec<(u64, u32)> {
    let mut out = Vec::new();
    let mut p = 2;
    while p * p <= n {
        let mut e = 0;
        while n % p == 0 {
            n /= p;
            e += 1;
        }
        if e > 0 {
            out.push((p, e));
        }
        p += if p == 2 { 1 } else { 2 };
    }
    if n > 1 {
        out.push((n, 1));
    }
    out
}

fn totient(n: u64) -> u64 {
    factorize(n).iter().fold(n, |acc, &(p, _)| acc / p * (p - 1))
}

fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    let mut d = 2;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += 1;
    }
    true
}

fn main() {
    let primes = sieve(100);
    println!("primes <= 100 ({}): {:?}", primes.len(), primes);
    let big = sieve(1_000_000);
    println!("pi(10^6) = {}", big.len());
    println!("largest prime <= 10^6 = {}", big.last().unwrap());
    let sum: u64 = big.iter().map(|&p| p as u64).sum();
    println!("sum of primes <= 10^6 = {}", sum);
    let twins = big.windows(2).filter(|w| w[1] - w[0] == 2).count();
    println!("twin prime pairs <= 10^6 = {}", twins);
    let max_gap = big.windows(2).map(|w| (w[1] - w[0], w[0])).max().unwrap();
    println!("max gap = {} after {}", max_gap.0, max_gap.1);
    for n in [360u64, 97, 1001, 65536, 600851475143, 1] {
        let f = factorize(n);
        let s: Vec<String> = f.iter().map(|(p, e)| if *e > 1 { format!("{}^{}", p, e) } else { p.to_string() }).collect();
        println!("{} = {} (phi = {})", n, if s.is_empty() { "1".to_string() } else { s.join(" * ") }, totient(n));
    }
    let goldbach: Vec<(u64, u64)> = (4..=30u64)
        .step_by(2)
        .map(|n| {
            let p = (2..=n / 2).find(|&p| is_prime(p) && is_prime(n - p)).unwrap();
            (p, n - p)
        })
        .collect();
    println!("goldbach: {:?}", goldbach);
    let mersenne: Vec<u32> = (2..32).filter(|&e| is_prime(e as u64) && is_prime((1u64 << e) - 1)).collect();
    println!("mersenne exponents < 32: {:?}", mersenne);
}
