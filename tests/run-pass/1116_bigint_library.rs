// Arbitrary-precision integer library: sign-magnitude with u32 limbs,
// add/sub/mul (schoolbook + Karatsuba), divmod (Knuth D), pow, gcd,
// parsing/printing in several radixes, and applications (factorials,
// Fibonacci, binomials, RSA toy, digits of e).
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};
use std::str::FromStr;

#[derive(Clone, PartialEq, Eq, Debug)]
struct BigUint { d: Vec<u32> } // little-endian limbs, no trailing zeros

impl BigUint {
    fn zero() -> Self { BigUint { d: vec![] } }
    fn from_u64(mut n: u64) -> Self {
        let mut d = Vec::new();
        while n > 0 { d.push(n as u32); n >>= 32; }
        BigUint { d }
    }
    fn is_zero(&self) -> bool { self.d.is_empty() }
    fn trim(mut self) -> Self { while self.d.last() == Some(&0) { self.d.pop(); } self }
    fn bits(&self) -> u64 {
        match self.d.last() { None => 0, Some(&top) => (self.d.len() as u64 - 1) * 32 + (32 - top.leading_zeros() as u64) }
    }
    fn to_u64(&self) -> Option<u64> {
        match self.d.len() { 0 => Some(0), 1 => Some(self.d[0] as u64), 2 => Some(self.d[0] as u64 | (self.d[1] as u64) << 32), _ => None }
    }

    fn add_ref(&self, o: &Self) -> Self {
        let (a, b) = if self.d.len() >= o.d.len() { (self, o) } else { (o, self) };
        let mut out = Vec::with_capacity(a.d.len() + 1);
        let mut carry = 0u64;
        for i in 0..a.d.len() {
            let s = a.d[i] as u64 + *b.d.get(i).unwrap_or(&0) as u64 + carry;
            out.push(s as u32);
            carry = s >> 32;
        }
        if carry > 0 { out.push(carry as u32); }
        BigUint { d: out }
    }

    // requires self >= o
    fn sub_ref(&self, o: &Self) -> Self {
        let mut out = Vec::with_capacity(self.d.len());
        let mut borrow = 0i64;
        for i in 0..self.d.len() {
            let mut s = self.d[i] as i64 - *o.d.get(i).unwrap_or(&0) as i64 - borrow;
            borrow = if s < 0 { s += 1 << 32; 1 } else { 0 };
            out.push(s as u32);
        }
        assert_eq!(borrow, 0, "subtraction underflow");
        BigUint { d: out }.trim()
    }

    fn mul_school(a: &[u32], b: &[u32]) -> Vec<u32> {
        if a.is_empty() || b.is_empty() { return vec![]; }
        let mut out = vec![0u32; a.len() + b.len()];
        for (i, &x) in a.iter().enumerate() {
            let mut carry = 0u64;
            for (j, &y) in b.iter().enumerate() {
                let t = x as u64 * y as u64 + out[i + j] as u64 + carry;
                out[i + j] = t as u32;
                carry = t >> 32;
            }
            let mut k = i + b.len();
            while carry > 0 {
                let t = out[k] as u64 + carry;
                out[k] = t as u32;
                carry = t >> 32;
                k += 1;
            }
        }
        out
    }

    fn shl_limbs(&self, n: usize) -> Self {
        if self.is_zero() { return self.clone(); }
        let mut d = vec![0u32; n];
        d.extend_from_slice(&self.d);
        BigUint { d }
    }

    fn karatsuba(a: &BigUint, b: &BigUint) -> BigUint {
        let n = a.d.len().max(b.d.len());
        if a.d.len() < 24 || b.d.len() < 24 {
            return BigUint { d: Self::mul_school(&a.d, &b.d) }.trim();
        }
        let h = n / 2;
        let split = |x: &BigUint| -> (BigUint, BigUint) {
            let lo = BigUint { d: x.d[..h.min(x.d.len())].to_vec() }.trim();
            let hi = BigUint { d: if x.d.len() > h { x.d[h..].to_vec() } else { vec![] } };
            (lo, hi)
        };
        let (a0, a1) = split(a);
        let (b0, b1) = split(b);
        let z0 = Self::karatsuba(&a0, &b0);
        let z2 = Self::karatsuba(&a1, &b1);
        let z1 = Self::karatsuba(&a0.add_ref(&a1), &b0.add_ref(&b1)).sub_ref(&z0).sub_ref(&z2);
        z2.shl_limbs(2 * h).add_ref(&z1.shl_limbs(h)).add_ref(&z0)
    }

    fn divmod_small(&self, m: u32) -> (BigUint, u32) {
        let mut out = vec![0u32; self.d.len()];
        let mut rem = 0u64;
        for i in (0..self.d.len()).rev() {
            let cur = (rem << 32) | self.d[i] as u64;
            out[i] = (cur / m as u64) as u32;
            rem = cur % m as u64;
        }
        (BigUint { d: out }.trim(), rem as u32)
    }

    fn shl_bits(&self, s: u32) -> BigUint {
        if s == 0 { return self.clone(); }
        let mut out = Vec::with_capacity(self.d.len() + 1);
        let mut carry = 0u32;
        for &x in &self.d { out.push((x << s) | carry); carry = x >> (32 - s); }
        if carry != 0 { out.push(carry); }
        BigUint { d: out }
    }
    fn shr_bits(&self, s: u32) -> BigUint {
        if s == 0 { return self.clone(); }
        let mut out = vec![0u32; self.d.len()];
        for i in 0..self.d.len() {
            out[i] = (self.d[i] >> s) | self.d.get(i + 1).map_or(0, |&n| n << (32 - s));
        }
        BigUint { d: out }.trim()
    }

    /// Knuth algorithm D
    fn divmod(&self, v: &BigUint) -> (BigUint, BigUint) {
        assert!(!v.is_zero(), "division by zero");
        if self.cmp(v) == Ordering::Less { return (BigUint::zero(), self.clone()); }
        if v.d.len() == 1 {
            let (q, r) = self.divmod_small(v.d[0]);
            return (q, BigUint::from_u64(r as u64));
        }
        let s = v.d.last().unwrap().leading_zeros();
        let vn = v.shl_bits(s);
        let mut un = self.shl_bits(s);
        if un.d.len() == self.d.len() { un.d.push(0); }
        let n = vn.d.len();
        let m = un.d.len() - n;
        let mut q = vec![0u32; m];
        let b: u64 = 1 << 32;
        for j in (0..m).rev() {
            let num = (un.d[j + n] as u64) << 32 | un.d[j + n - 1] as u64;
            let mut qhat = num / vn.d[n - 1] as u64;
            let mut rhat = num % vn.d[n - 1] as u64;
            while qhat >= b || qhat * vn.d[n - 2] as u64 > (rhat << 32 | un.d[j + n - 2] as u64) {
                qhat -= 1;
                rhat += vn.d[n - 1] as u64;
                if rhat >= b { break; }
            }
            let mut borrow: i64 = 0;
            let mut carry: u64 = 0;
            for i in 0..n {
                let p = qhat * vn.d[i] as u64 + carry;
                carry = p >> 32;
                let t = un.d[i + j] as i64 - borrow - (p & 0xffff_ffff) as i64;
                un.d[i + j] = t as u32;
                borrow = if t < 0 { 1 } else { 0 };
            }
            let t = un.d[j + n] as i64 - borrow - carry as i64;
            un.d[j + n] = t as u32;
            if t < 0 {
                qhat -= 1;
                let mut c = 0u64;
                for i in 0..n {
                    let s2 = un.d[i + j] as u64 + vn.d[i] as u64 + c;
                    un.d[i + j] = s2 as u32;
                    c = s2 >> 32;
                }
                un.d[j + n] = un.d[j + n].wrapping_add(c as u32);
            }
            q[j] = qhat as u32;
        }
        let r = BigUint { d: un.d[..n].to_vec() }.trim().shr_bits(s);
        (BigUint { d: q }.trim(), r)
    }

    fn pow(&self, mut e: u32) -> BigUint {
        let mut base = self.clone();
        let mut acc = BigUint::from_u64(1);
        while e > 0 {
            if e & 1 == 1 { acc = &acc * &base; }
            e >>= 1;
            if e > 0 { base = &base * &base; }
        }
        acc
    }

    fn modpow(&self, e: &BigUint, m: &BigUint) -> BigUint {
        let mut result = BigUint::from_u64(1);
        let mut base = self.divmod(m).1;
        for i in 0..e.bits() {
            if e.d[(i / 32) as usize] >> (i % 32) & 1 == 1 { result = (&result * &base).divmod(m).1; }
            base = (&base * &base).divmod(m).1;
        }
        result
    }

    fn to_str_radix(&self, radix: u32) -> String {
        if self.is_zero() { return "0".into(); }
        let mut chunk = radix;
        let mut digits_per = 1;
        while (chunk as u64) * (radix as u64) < (1 << 32) { chunk *= radix; digits_per += 1; }
        let mut parts = Vec::new();
        let mut cur = self.clone();
        while !cur.is_zero() {
            let (q, r) = cur.divmod_small(chunk);
            parts.push(r);
            cur = q;
        }
        let mut s = String::new();
        for (i, p) in parts.iter().rev().enumerate() {
            let mut digits = Vec::new();
            let mut x = *p;
            for _ in 0..digits_per { digits.push(std::char::from_digit(x % radix, radix).unwrap()); x /= radix; }
            let chunk_str: String = digits.iter().rev().collect();
            if i == 0 { s.push_str(chunk_str.trim_start_matches('0')); } else { s.push_str(&chunk_str); }
        }
        s
    }

    fn from_str_radix(s: &str, radix: u32) -> Result<BigUint, String> {
        if s.is_empty() { return Err("empty".into()); }
        let mut acc = BigUint::zero();
        let r = BigUint::from_u64(radix as u64);
        for c in s.chars() {
            if c == '_' { continue; }
            let d = c.to_digit(radix).ok_or_else(|| format!("invalid digit {:?} for radix {}", c, radix))?;
            acc = (&acc * &r).add_ref(&BigUint::from_u64(d as u64));
        }
        Ok(acc)
    }
}

impl Ord for BigUint {
    fn cmp(&self, o: &Self) -> Ordering {
        self.d.len().cmp(&o.d.len()).then_with(|| self.d.iter().rev().cmp(o.d.iter().rev()))
    }
}
impl PartialOrd for BigUint { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }

impl<'a> Mul<&'a BigUint> for &'a BigUint {
    type Output = BigUint;
    fn mul(self, o: &BigUint) -> BigUint { BigUint::karatsuba(self, o) }
}

impl fmt::Display for BigUint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { f.pad_integral(true, "", &self.to_str_radix(10)) }
}
impl fmt::LowerHex for BigUint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { f.pad_integral(true, "0x", &self.to_str_radix(16)) }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct BigInt { neg: bool, mag: BigUint }

impl BigInt {
    fn new(neg: bool, mag: BigUint) -> Self { BigInt { neg: neg && !mag.is_zero(), mag } }
    fn from_i64(n: i64) -> Self { BigInt::new(n < 0, BigUint::from_u64(n.unsigned_abs())) }
    fn abs(&self) -> BigInt { BigInt::new(false, self.mag.clone()) }
}

impl FromStr for BigInt {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let (neg, rest) = match s.strip_prefix('-') { Some(r) => (true, r), None => (false, s.strip_prefix('+').unwrap_or(s)) };
        let (radix, digits) = if let Some(h) = rest.strip_prefix("0x") { (16, h) } else if let Some(b) = rest.strip_prefix("0b") { (2, b) } else { (10, rest) };
        Ok(BigInt::new(neg, BigUint::from_str_radix(digits, radix)?))
    }
}

impl fmt::Display for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { f.pad_integral(!self.neg, "", &self.mag.to_str_radix(10)) }
}

impl Ord for BigInt {
    fn cmp(&self, o: &Self) -> Ordering {
        match (self.neg, o.neg) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => self.mag.cmp(&o.mag),
            (true, true) => o.mag.cmp(&self.mag),
        }
    }
}
impl PartialOrd for BigInt { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }

impl Add for &BigInt {
    type Output = BigInt;
    fn add(self, o: &BigInt) -> BigInt {
        if self.neg == o.neg { return BigInt::new(self.neg, self.mag.add_ref(&o.mag)); }
        match self.mag.cmp(&o.mag) {
            Ordering::Equal => BigInt::from_i64(0),
            Ordering::Greater => BigInt::new(self.neg, self.mag.sub_ref(&o.mag)),
            Ordering::Less => BigInt::new(o.neg, o.mag.sub_ref(&self.mag)),
        }
    }
}
impl Neg for &BigInt { type Output = BigInt; fn neg(self) -> BigInt { BigInt::new(!self.neg, self.mag.clone()) } }
impl Sub for &BigInt { type Output = BigInt; fn sub(self, o: &BigInt) -> BigInt { self + &(-o) } }
impl Mul for &BigInt { type Output = BigInt; fn mul(self, o: &BigInt) -> BigInt { BigInt::new(self.neg != o.neg, &self.mag * &o.mag) } }
// truncating division like Rust's primitive integers
impl Div for &BigInt { type Output = BigInt; fn div(self, o: &BigInt) -> BigInt { BigInt::new(self.neg != o.neg, self.mag.divmod(&o.mag).0) } }
impl Rem for &BigInt { type Output = BigInt; fn rem(self, o: &BigInt) -> BigInt { BigInt::new(self.neg, self.mag.divmod(&o.mag).1) } }

fn gcd(a: &BigUint, b: &BigUint) -> BigUint {
    let (mut a, mut b) = (a.clone(), b.clone());
    while !b.is_zero() { let r = a.divmod(&b).1; a = b; b = r; }
    a
}

fn factorial(n: u32) -> BigUint {
    // product tree for balanced multiplication sizes
    fn prod(lo: u64, hi: u64) -> BigUint {
        if hi - lo < 8 { return (lo..=hi).fold(BigUint::from_u64(1), |a, k| &a * &BigUint::from_u64(k)); }
        let mid = (lo + hi) / 2;
        &prod(lo, mid) * &prod(mid + 1, hi)
    }
    if n < 2 { BigUint::from_u64(1) } else { prod(2, n as u64) }
}

fn digit_sum(s: &str) -> u32 { s.bytes().map(|b| (b - b'0') as u32).sum() }

fn main() {
    let a: BigInt = "123456789012345678901234567890".parse().unwrap();
    let b: BigInt = "-987654321098765432109876543210".parse().unwrap();
    println!("a+b = {}", &a + &b);
    println!("a-b = {}", &a - &b);
    println!("a*b = {}", &a * &b);
    println!("b/a = {}  b%a = {}", &b / &a, &b % &a);
    println!("check: {}", &(&(&b / &a) * &a) + &(&b % &a) == b);
    for (x, y) in [(7i64, 3i64), (-7, 3), (7, -3), (-7, -3), (0, 5)] {
        let (bx, by) = (BigInt::from_i64(x), BigInt::from_i64(y));
        let ok = (&bx / &by).to_string() == (x / y).to_string() && (&bx % &by).to_string() == (x % y).to_string();
        print!("{}/{}={},{} {} ", x, y, &bx / &by, &bx % &by, if ok { "ok" } else { "BAD" });
    }
    println!();

    let f100 = factorial(100);
    let s = f100.to_string();
    println!("100! = {}", s);
    println!("100! has {} digits, digit sum {}, {} bits, trailing zeros {}", s.len(), digit_sum(&s), f100.bits(), s.len() - s.trim_end_matches('0').len());
    let f1000 = factorial(1000);
    let s = f1000.to_string();
    println!("1000! has {} digits, digit sum {}, starts {} ends {}", s.len(), digit_sum(&s), &s[..20], &s[s.len() - 260..s.len() - 249]);
    let q = f1000.divmod(&factorial(998)).0;
    println!("1000!/998! = {}", q);
    let schoolbook = BigUint { d: BigUint::mul_school(&f1000.d, &f100.d) }.trim();
    println!("karatsuba == schoolbook: {}", &f1000 * &f100 == schoolbook);

    let two = BigUint::from_u64(2);
    let m127 = two.pow(127).sub_ref(&BigUint::from_u64(1));
    println!("2^127-1 = {}", m127);
    println!("hex: {:x}  padded: [{:>45}]  [{:<#40x}]", m127, m127, BigUint::from_u64(255));
    println!("2^127-1 as u128 = {}", u128::MAX >> 1);
    println!("2^64 = {}  to_u64 {:?}  2^64-1 to_u64 {:?}", two.pow(64), two.pow(64).to_u64(), two.pow(64).sub_ref(&BigUint::from_u64(1)).to_u64());
    // fermat test on mersenne prime
    let base = BigUint::from_u64(3);
    let e = m127.sub_ref(&BigUint::from_u64(1));
    println!("3^(p-1) mod p == 1: {}", base.modpow(&e, &m127) == BigUint::from_u64(1));
    let m67 = two.pow(67).sub_ref(&BigUint::from_u64(1));
    println!("2^67-1 fermat: {}", base.modpow(&m67.sub_ref(&BigUint::from_u64(1)), &m67) == BigUint::from_u64(1));

    // fibonacci
    let (mut x, mut y) = (BigUint::zero(), BigUint::from_u64(1));
    for _ in 0..500 { let z = x.add_ref(&y); x = y; y = z; }
    println!("F(500) = {}", x);
    println!("gcd(F(500), F(300)) = {}", gcd(&x, &{ let (mut p, mut q) = (BigUint::zero(), BigUint::from_u64(1)); for _ in 0..300 { let z = p.add_ref(&q); p = q; q = z; } p }));

    // binomial(200, 100)
    let c = factorial(200).divmod(&(&factorial(100) * &factorial(100))).0;
    println!("C(200,100) = {}", c);

    // toy RSA
    let p = BigUint::from_str_radix("170141183460469231731687303715884105727", 10).unwrap();
    let q2 = BigUint::from_str_radix("2305843009213693951", 10).unwrap();
    let n = &p * &q2;
    let phi = &p.sub_ref(&BigUint::from_u64(1)) * &q2.sub_ref(&BigUint::from_u64(1));
    let e = BigUint::from_u64(65537);
    println!("gcd(e, phi) = {}", gcd(&e, &phi));
    // modular inverse via extended Euclid on BigInt
    let (mut old_r, mut r) = (BigInt::new(false, e.clone()), BigInt::new(false, phi.clone()));
    let (mut old_s, mut s) = (BigInt::from_i64(1), BigInt::from_i64(0));
    while r != BigInt::from_i64(0) {
        let qq = &old_r / &r;
        let nr = &old_r - &(&qq * &r);
        old_r = std::mem::replace(&mut r, nr);
        let ns = &old_s - &(&qq * &s);
        old_s = std::mem::replace(&mut s, ns);
    }
    let phi_i = BigInt::new(false, phi.clone());
    let d = &(&(&old_s % &phi_i) + &phi_i) % &phi_i;
    let msg = BigUint::from_str_radix("48656c6c6f2c20525341", 16).unwrap();
    let ct = msg.modpow(&e, &n);
    let pt = ct.modpow(&d.mag, &n);
    println!("n = {}\nct = {:x}\nround trip: {} ({:x})", n, ct, pt == msg, pt);

    // e to 60 digits: sum 1/k! scaled
    let scale = BigUint::from_u64(10).pow(70);
    let mut term = scale.clone();
    let mut sum = BigUint::zero();
    let mut k = 1u32;
    while !term.is_zero() { sum = sum.add_ref(&term); term = term.divmod_small(k).0; k += 1; }
    let es = sum.to_string();
    println!("e = {}.{} ({} terms)", &es[..1], &es[1..61], k);

    let mut sorted: Vec<BigInt> = ["5", "-3", "0x10", "-0b1010", "123456789123456789123", "-123456789123456789123", "0"].iter().map(|s| s.parse().unwrap()).collect();
    sorted.sort();
    println!("sorted: {}", sorted.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" "));
    println!("max abs: {}", sorted.iter().map(|x| x.abs()).max().unwrap());
    println!("{:?} {:?} {:?}", "12a".parse::<BigInt>(), "".parse::<BigInt>(), "0xZZ".parse::<BigInt>());
    println!("[{:>8}] [{:<8}] [{:+}] [{:08}]", BigInt::from_i64(-42), BigInt::from_i64(42), BigInt::from_i64(42), BigInt::from_i64(-42));
    let r = std::panic::catch_unwind(|| BigUint::from_u64(1).divmod(&BigUint::zero()));
    println!("div by zero panics: {}", r.is_err());
}
