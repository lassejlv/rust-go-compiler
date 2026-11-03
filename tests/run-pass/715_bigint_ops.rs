// Arbitrary-precision unsigned/signed integers on Vec<u32> limbs with
// Add/Sub/Mul/Div by small, comparison, Display, parsing and FromStr.
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Mul, Sub};
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq)]
struct BigUint { limbs: Vec<u32> } // little endian

impl BigUint {
    fn zero() -> Self { BigUint { limbs: vec![] } }
    fn from_u64(mut v: u64) -> Self {
        let mut limbs = vec![];
        while v > 0 { limbs.push(v as u32); v >>= 32; }
        BigUint { limbs }
    }
    fn normalize(mut self) -> Self {
        while self.limbs.last() == Some(&0) { self.limbs.pop(); }
        self
    }
    fn is_zero(&self) -> bool { self.limbs.is_empty() }
    fn divrem_small(&self, d: u32) -> (BigUint, u32) {
        let mut out = vec![0u32; self.limbs.len()];
        let mut rem = 0u64;
        for i in (0..self.limbs.len()).rev() {
            let cur = (rem << 32) | self.limbs[i] as u64;
            out[i] = (cur / d as u64) as u32;
            rem = cur % d as u64;
        }
        (BigUint { limbs: out }.normalize(), rem as u32)
    }
    fn mul_small(&self, m: u32, add: u32) -> BigUint {
        let mut carry = add as u64;
        let mut out = Vec::with_capacity(self.limbs.len() + 1);
        for &l in &self.limbs {
            let t = l as u64 * m as u64 + carry;
            out.push(t as u32);
            carry = t >> 32;
        }
        if carry > 0 { out.push(carry as u32); }
        BigUint { limbs: out }.normalize()
    }
    fn pow(&self, mut e: u32) -> BigUint {
        let mut base = self.clone();
        let mut acc = BigUint::from_u64(1);
        while e > 0 {
            if e & 1 == 1 { acc = &acc * &base; }
            base = &base * &base;
            e >>= 1;
        }
        acc
    }
    fn bits(&self) -> u32 {
        match self.limbs.last() { None => 0, Some(&top) => (self.limbs.len() as u32 - 1) * 32 + (32 - top.leading_zeros()) }
    }
    fn digit_sum(&self) -> u32 { self.to_string().bytes().map(|b| (b - b'0') as u32).sum() }
}

impl Ord for BigUint {
    fn cmp(&self, o: &Self) -> Ordering {
        self.limbs.len().cmp(&o.limbs.len()).then_with(|| self.limbs.iter().rev().cmp(o.limbs.iter().rev()))
    }
}
impl PartialOrd for BigUint { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }

impl<'a> Add<&'a BigUint> for &'a BigUint {
    type Output = BigUint;
    fn add(self, o: &BigUint) -> BigUint {
        let n = self.limbs.len().max(o.limbs.len());
        let mut out = Vec::with_capacity(n + 1);
        let mut carry = 0u64;
        for i in 0..n {
            let s = *self.limbs.get(i).unwrap_or(&0) as u64 + *o.limbs.get(i).unwrap_or(&0) as u64 + carry;
            out.push(s as u32);
            carry = s >> 32;
        }
        if carry > 0 { out.push(carry as u32); }
        BigUint { limbs: out }
    }
}
impl<'a> Sub<&'a BigUint> for &'a BigUint {
    type Output = Option<BigUint>;
    fn sub(self, o: &BigUint) -> Option<BigUint> {
        if self < o { return None; }
        let mut out = Vec::with_capacity(self.limbs.len());
        let mut borrow = 0i64;
        for i in 0..self.limbs.len() {
            let mut d = self.limbs[i] as i64 - *o.limbs.get(i).unwrap_or(&0) as i64 - borrow;
            borrow = if d < 0 { d += 1 << 32; 1 } else { 0 };
            out.push(d as u32);
        }
        Some(BigUint { limbs: out }.normalize())
    }
}
impl<'a> Mul<&'a BigUint> for &'a BigUint {
    type Output = BigUint;
    fn mul(self, o: &BigUint) -> BigUint {
        if self.is_zero() || o.is_zero() { return BigUint::zero(); }
        let mut out = vec![0u32; self.limbs.len() + o.limbs.len()];
        for (i, &a) in self.limbs.iter().enumerate() {
            let mut carry = 0u64;
            for (j, &b) in o.limbs.iter().enumerate() {
                let t = out[i + j] as u64 + a as u64 * b as u64 + carry;
                out[i + j] = t as u32;
                carry = t >> 32;
            }
            let mut k = i + o.limbs.len();
            while carry > 0 {
                let t = out[k] as u64 + carry;
                out[k] = t as u32;
                carry = t >> 32;
                k += 1;
            }
        }
        BigUint { limbs: out }.normalize()
    }
}

impl fmt::Display for BigUint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_zero() { return f.pad_integral(true, "", "0"); }
        let mut parts = Vec::new();
        let mut cur = self.clone();
        while !cur.is_zero() {
            let (q, r) = cur.divrem_small(1_000_000_000);
            parts.push(r);
            cur = q;
        }
        let mut s = parts.last().unwrap().to_string();
        for p in parts.iter().rev().skip(1) { s.push_str(&format!("{:09}", p)); }
        f.pad_integral(true, "", &s)
    }
}

impl fmt::LowerHex for BigUint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::new();
        for (i, l) in self.limbs.iter().rev().enumerate() {
            if i == 0 { s.push_str(&format!("{:x}", l)); } else { s.push_str(&format!("{:08x}", l)); }
        }
        if s.is_empty() { s.push('0'); }
        f.pad_integral(true, "0x", &s)
    }
}

impl FromStr for BigUint {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        if s.is_empty() { return Err("empty".into()); }
        s.chars().try_fold(BigUint::zero(), |acc, c| {
            c.to_digit(10).map(|d| acc.mul_small(10, d)).ok_or_else(|| format!("bad digit {:?}", c))
        })
    }
}

fn factorial(n: u32) -> BigUint { (2..=n).fold(BigUint::from_u64(1), |a, k| a.mul_small(k, 0)) }

fn fib(n: usize) -> BigUint {
    let (mut a, mut b) = (BigUint::zero(), BigUint::from_u64(1));
    for _ in 0..n { let c = &a + &b; a = b; b = c; }
    a
}

fn main() {
    let f50 = factorial(50);
    println!("50! = {}", f50);
    println!("50! bits={} digit_sum={} limbs={}", f50.bits(), f50.digit_sum(), f50.limbs.len());
    println!("fib(200) = {}", fib(200));
    let two = BigUint::from_u64(2);
    let p = two.pow(127);
    println!("2^127 = {} = {:#x}", p, p);
    let m127 = (&p - &BigUint::from_u64(1)).unwrap();
    println!("M127 = {}", m127);
    println!("u128 check: {}", u128::MAX / 2 == m127.to_string().parse::<u128>().unwrap());
    let a: BigUint = "123456789012345678901234567890".parse().unwrap();
    let b: BigUint = "987654321098765432109876543210".parse().unwrap();
    println!("a*b = {}", &a * &b);
    println!("b-a = {:?}", (&b - &a).map(|x| x.to_string()));
    println!("a-b = {:?}", &a - &b);
    println!("cmp: {:?} {:?} {:?}", a.cmp(&b), b.cmp(&a), a.cmp(&a.clone()));
    println!("parse errors: {:?} {:?}", "12x4".parse::<BigUint>(), "".parse::<BigUint>());
    println!("padded: [{:>12}] [{:<8}] [{:+}] [{:012}]", BigUint::from_u64(12345), BigUint::zero(), BigUint::from_u64(9), BigUint::from_u64(777));
    let (q, r) = f50.divrem_small(1_000_003);
    println!("50! / 1000003 = {} rem {}", q, r);
    let mut sorted: Vec<BigUint> = [fib(90), factorial(20), two.pow(64), BigUint::from_u64(u64::MAX)].to_vec();
    sorted.sort();
    println!("sorted: {:?}", sorted.iter().map(|x| x.to_string()).collect::<Vec<_>>());
    println!("max: {}", sorted.iter().max().unwrap());
}
