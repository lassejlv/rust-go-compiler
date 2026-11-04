// A compute-heavy workload mix (used to compare runtime performance):
// complex FFT + polynomial multiplication, sieve of Eratosthenes, hash map
// word counting over generated text, string building, sorting large vectors,
// BTreeMap churn, and a dynamic-programming edit distance -- all with
// deterministic checksums.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::f64::consts::PI;
use std::fmt::Write;
use std::ops::{Add, Mul, Sub};

#[derive(Clone, Copy, Debug, PartialEq)]
struct C {
    re: f64,
    im: f64,
}
impl Add for C {
    type Output = C;
    fn add(self, o: C) -> C {
        C { re: self.re + o.re, im: self.im + o.im }
    }
}
impl Sub for C {
    type Output = C;
    fn sub(self, o: C) -> C {
        C { re: self.re - o.re, im: self.im - o.im }
    }
}
impl Mul for C {
    type Output = C;
    fn mul(self, o: C) -> C {
        C { re: self.re * o.re - self.im * o.im, im: self.re * o.im + self.im * o.re }
    }
}

fn fft(a: &mut [C], invert: bool) {
    let n = a.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            a.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = 2.0 * PI / len as f64 * if invert { -1.0 } else { 1.0 };
        let wl = C { re: ang.cos(), im: ang.sin() };
        for chunk in a.chunks_mut(len) {
            let mut w = C { re: 1.0, im: 0.0 };
            let (lo, hi) = chunk.split_at_mut(len / 2);
            for (u, v) in lo.iter_mut().zip(hi.iter_mut()) {
                let x = *u;
                let y = *v * w;
                *u = x + y;
                *v = x - y;
                w = w * wl;
            }
        }
        len <<= 1;
    }
    if invert {
        for x in a.iter_mut() {
            x.re /= n as f64;
            x.im /= n as f64;
        }
    }
}

fn poly_mul(a: &[u64], b: &[u64]) -> Vec<u64> {
    let n = (a.len() + b.len()).next_power_of_two();
    let mut fa: Vec<C> = a.iter().map(|&x| C { re: x as f64, im: 0.0 }).chain(std::iter::repeat(C { re: 0.0, im: 0.0 })).take(n).collect();
    let mut fb: Vec<C> = b.iter().map(|&x| C { re: x as f64, im: 0.0 }).chain(std::iter::repeat(C { re: 0.0, im: 0.0 })).take(n).collect();
    fft(&mut fa, false);
    fft(&mut fb, false);
    for (x, y) in fa.iter_mut().zip(&fb) {
        *x = *x * *y;
    }
    fft(&mut fa, true);
    fa.iter().take(a.len() + b.len() - 1).map(|c| c.re.round() as u64).collect()
}

fn sieve(n: usize) -> Vec<u32> {
    let mut is = vec![true; n + 1];
    is[0] = false;
    is[1] = false;
    let mut i = 2;
    while i * i <= n {
        if is[i] {
            let mut j = i * i;
            while j <= n {
                is[j] = false;
                j += i;
            }
        }
        i += 1;
    }
    is.iter().enumerate().filter(|(_, &p)| p).map(|(i, _)| i as u32).collect()
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545F4914F6CDD1D)
    }
}

fn main() {
    // FFT polynomial multiplication checked against the naive product
    let mut rng = Rng(88172645463325252);
    let a: Vec<u64> = (0..2000).map(|_| rng.next() % 100).collect();
    let b: Vec<u64> = (0..1500).map(|_| rng.next() % 100).collect();
    let fast = poly_mul(&a, &b);
    let mut naive = vec![0u64; a.len() + b.len() - 1];
    for (i, &x) in a.iter().enumerate() {
        for (j, &y) in b.iter().enumerate() {
            naive[i + j] += x * y;
        }
    }
    println!("fft product ok: {} checksum {}", fast == naive, fast.iter().enumerate().fold(0u64, |h, (i, &v)| h.wrapping_mul(31).wrapping_add(v ^ i as u64)));

    let primes = sieve(1_000_000);
    println!("primes below 1e6: {} last {:?} sum {}", primes.len(), primes.last(), primes.iter().map(|&p| p as u64).sum::<u64>());
    let twins = primes.windows(2).filter(|w| w[1] - w[0] == 2).count();
    let gaps: BTreeMap<u32, usize> = primes.windows(2).map(|w| w[1] - w[0]).fold(BTreeMap::new(), |mut m, g| {
        *m.entry(g).or_insert(0) += 1;
        m
    });
    println!("twin pairs {} max gap {:?}", twins, gaps.iter().next_back());

    // word counting
    let syll = ["ka", "lo", "mi", "ne", "ru", "ta", "shi", "po", "ve", "zu"];
    let mut text = String::with_capacity(1 << 20);
    for i in 0..60_000 {
        let n = 1 + (rng.next() % 3) as usize;
        for _ in 0..n {
            text.push_str(syll[(rng.next() % syll.len() as u64) as usize]);
        }
        text.push(if i % 13 == 12 { '\n' } else { ' ' });
    }
    let mut counts: HashMap<&str, u32> = HashMap::new();
    for w in text.split_whitespace() {
        *counts.entry(w).or_insert(0) += 1;
    }
    let mut top: Vec<(&str, u32)> = counts.iter().map(|(k, v)| (*k, *v)).collect();
    top.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    println!("distinct words {} top {:?}", counts.len(), &top[..5]);
    let uniq_lines: HashSet<&str> = text.lines().collect();
    println!("lines {} unique {}", text.lines().count(), uniq_lines.len());

    // sorting
    let mut big: Vec<u64> = (0..300_000).map(|_| rng.next() >> 20).collect();
    let mut big2 = big.clone();
    big.sort_unstable();
    big2.sort();
    let mut pairs: Vec<(u32, u32)> = (0..100_000).map(|i| ((rng.next() % 1000) as u32, i)).collect();
    pairs.sort_by_key(|p| p.0);
    let stable = pairs.windows(2).all(|w| w[0].0 < w[1].0 || (w[0].0 == w[1].0 && w[0].1 < w[1].1));
    big.dedup();
    println!("sorted equal {} stable {} distinct {} median {}", big == { big2.dedup(); big2 }, stable, big.len(), big[big.len() / 2]);

    // BTreeMap churn
    let mut bt = BTreeMap::new();
    for i in 0..60_000u64 {
        let k = rng.next() % 50_000;
        if i % 3 == 0 {
            bt.remove(&k);
        } else {
            *bt.entry(k).or_insert(0u64) += i;
        }
    }
    let range_sum: u64 = bt.range(10_000..20_000).map(|(_, v)| *v).sum();
    println!("btree size {} first {:?} range sum {}", bt.len(), bt.iter().next(), range_sum);

    // string formatting throughput
    let mut out = String::new();
    for i in 0..30_000 {
        write!(out, "{:>6}|{:<8.3}|{:x};", i, i as f64 / 7.0, i * 31).unwrap();
    }
    let h = out.bytes().fold(0u32, |h, b| h.rotate_left(5) ^ b as u32);
    println!("formatted {} bytes hash {:08x}", out.len(), h);

    // edit distances
    let words: Vec<String> = (0..150).map(|i| {
        let mut s = String::new();
        let mut x = i as u64 * 2654435761;
        for _ in 0..(8 + i % 9) {
            s.push((b'a' + (x % 5) as u8) as char);
            x /= 5;
            x = x.wrapping_add(i as u64 * 7);
        }
        s
    }).collect();
    let mut total = 0;
    let mut best = (usize::MAX, 0, 0);
    for i in 0..words.len() {
        for j in i + 1..words.len() {
            let d = edit_distance(&words[i], &words[j]);
            total += d;
            if d < best.0 {
                best = (d, i, j);
            }
        }
    }
    println!("edit distance total {} closest {:?} {:?}", total, best, (&words[best.1], &words[best.2]));
}
