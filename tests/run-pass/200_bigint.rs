use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Mul, Sub};

const BASE: u64 = 1 << 32;

#[derive(Clone, Debug, PartialEq, Eq)]
struct BigUint {
    limbs: Vec<u32>, // little endian
}

impl BigUint {
    fn from_u64(mut v: u64) -> Self {
        let mut limbs = Vec::new();
        while v > 0 {
            limbs.push(v as u32);
            v >>= 32;
        }
        BigUint { limbs }
    }
    fn zero() -> Self {
        BigUint { limbs: vec![] }
    }
    fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }
    fn trim(mut self) -> Self {
        while self.limbs.last() == Some(&0) {
            self.limbs.pop();
        }
        self
    }
    fn mul_small(&self, m: u32) -> Self {
        let mut carry = 0u64;
        let mut out = Vec::with_capacity(self.limbs.len() + 1);
        for &l in &self.limbs {
            let v = l as u64 * m as u64 + carry;
            out.push(v as u32);
            carry = v >> 32;
        }
        if carry > 0 {
            out.push(carry as u32);
        }
        BigUint { limbs: out }.trim()
    }
    fn divmod_small(&self, d: u32) -> (Self, u32) {
        let mut out = vec![0u32; self.limbs.len()];
        let mut rem = 0u64;
        for i in (0..self.limbs.len()).rev() {
            let cur = (rem << 32) | self.limbs[i] as u64;
            out[i] = (cur / d as u64) as u32;
            rem = cur % d as u64;
        }
        (BigUint { limbs: out }.trim(), rem as u32)
    }
    fn bits(&self) -> u32 {
        match self.limbs.last() {
            None => 0,
            Some(&top) => (self.limbs.len() as u32 - 1) * 32 + (32 - top.leading_zeros()),
        }
    }
    fn digit_sum(&self) -> u32 {
        self.to_string().bytes().map(|b| (b - b'0') as u32).sum()
    }
    fn pow(&self, mut e: u32) -> Self {
        let mut base = self.clone();
        let mut acc = BigUint::from_u64(1);
        while e > 0 {
            if e & 1 == 1 {
                acc = &acc * &base;
            }
            base = &base * &base;
            e >>= 1;
        }
        acc
    }
}

impl<'a> Add for &'a BigUint {
    type Output = BigUint;
    fn add(self, o: &BigUint) -> BigUint {
        let n = self.limbs.len().max(o.limbs.len());
        let mut out = Vec::with_capacity(n + 1);
        let mut carry = 0u64;
        for i in 0..n {
            let a = *self.limbs.get(i).unwrap_or(&0) as u64;
            let b = *o.limbs.get(i).unwrap_or(&0) as u64;
            let s = a + b + carry;
            out.push((s % BASE) as u32);
            carry = s / BASE;
        }
        if carry > 0 {
            out.push(carry as u32);
        }
        BigUint { limbs: out }
    }
}

impl<'a> Sub for &'a BigUint {
    type Output = BigUint;
    fn sub(self, o: &BigUint) -> BigUint {
        assert!(*self >= *o, "underflow");
        let mut out = Vec::with_capacity(self.limbs.len());
        let mut borrow = 0i64;
        for i in 0..self.limbs.len() {
            let mut d = self.limbs[i] as i64 - *o.limbs.get(i).unwrap_or(&0) as i64 - borrow;
            if d < 0 {
                d += BASE as i64;
                borrow = 1;
            } else {
                borrow = 0;
            }
            out.push(d as u32);
        }
        BigUint { limbs: out }.trim()
    }
}

impl<'a> Mul for &'a BigUint {
    type Output = BigUint;
    fn mul(self, o: &BigUint) -> BigUint {
        if self.is_zero() || o.is_zero() {
            return BigUint::zero();
        }
        let mut out = vec![0u64; self.limbs.len() + o.limbs.len() + 1];
        for (i, &a) in self.limbs.iter().enumerate() {
            let mut carry = 0u64;
            for (j, &b) in o.limbs.iter().enumerate() {
                let cur = out[i + j] + a as u64 * b as u64 + carry;
                out[i + j] = cur & 0xFFFF_FFFF;
                carry = cur >> 32;
            }
            let mut k = i + o.limbs.len();
            while carry > 0 {
                let cur = out[k] + carry;
                out[k] = cur & 0xFFFF_FFFF;
                carry = cur >> 32;
                k += 1;
            }
        }
        BigUint { limbs: out.into_iter().map(|x| x as u32).collect() }.trim()
    }
}

impl PartialOrd for BigUint {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for BigUint {
    fn cmp(&self, o: &Self) -> Ordering {
        self.limbs.len().cmp(&o.limbs.len()).then_with(|| self.limbs.iter().rev().cmp(o.limbs.iter().rev()))
    }
}

impl fmt::Display for BigUint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_zero() {
            return f.pad("0");
        }
        let mut chunks = Vec::new();
        let mut cur = self.clone();
        while !cur.is_zero() {
            let (q, r) = cur.divmod_small(1_000_000_000);
            chunks.push(r);
            cur = q;
        }
        let mut s = chunks.last().unwrap().to_string();
        for c in chunks.iter().rev().skip(1) {
            s.push_str(&format!("{:09}", c));
        }
        f.pad(&s)
    }
}

fn factorial(n: u32) -> BigUint {
    (2..=n).fold(BigUint::from_u64(1), |acc, k| acc.mul_small(k))
}

fn fibonacci(n: u32) -> BigUint {
    let (mut a, mut b) = (BigUint::zero(), BigUint::from_u64(1));
    for _ in 0..n {
        let c = &a + &b;
        a = b;
        b = c;
    }
    a
}

fn main() {
    let f50 = factorial(50);
    println!("50! = {}", f50);
    println!("50! bits {} limbs {} digit sum {}", f50.bits(), f50.limbs.len(), f50.digit_sum());
    let f100 = factorial(100);
    println!("100! digits {} digit sum {}", f100.to_string().len(), f100.digit_sum());
    let fib200 = fibonacci(200);
    println!("fib(200) = {}", fib200);
    println!("fib(93) = {}", fibonacci(93));
    println!("fib(93) fits u64: {}", fibonacci(93) == BigUint::from_u64(12200160415121876738));
    let two = BigUint::from_u64(2);
    let p128 = two.pow(128);
    println!("2^128 = {}", p128);
    println!("2^128 - 1 == u128::MAX: {}", (&p128 - &BigUint::from_u64(1)).to_string() == u128::MAX.to_string());
    println!("2^256 = {}", two.pow(256));
    let a = BigUint::from_u64(u64::MAX);
    let sq = &a * &a;
    println!("(2^64-1)^2 = {}", sq);
    println!("check: {}", (u64::MAX as u128) * (u64::MAX as u128) == sq.to_string().parse::<u128>().unwrap());
    let diff = &f50 - &factorial(49);
    println!("50! - 49! = {}", diff);
    println!("49 * 49! == diff: {}", factorial(49).mul_small(49) == diff);
    println!("{:?}", fibonacci(100).cmp(&factorial(25)));
    println!("[{:>12}] [{:<5}]", BigUint::from_u64(123456789), BigUint::zero());
    let (q, r) = f50.divmod_small(1_000_003);
    println!("50! mod 1000003 = {}, quotient digits {}", r, q.to_string().len());
    let mut v = vec![factorial(20), fibonacci(90), two.pow(64), BigUint::from_u64(7)];
    v.sort();
    println!("{:?}", v.iter().map(|b| b.to_string()).collect::<Vec<_>>());
}
