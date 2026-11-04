// Arbitrary precision integers (sign + magnitude, base 2^32 limbs) with the
// std operator traits, FromStr/Display/Hex, Sum/Product, and cross-checks
// against u128 arithmetic.
use std::cmp::Ordering;
use std::fmt;
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Rem, Shl, Shr, Sub, SubAssign};
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
struct BigUint {
    limbs: Vec<u32>, // little endian, no trailing zeros
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct BigInt {
    neg: bool,
    mag: BigUint,
}

impl BigUint {
    fn zero() -> Self {
        BigUint { limbs: Vec::new() }
    }
    fn one() -> Self {
        BigUint { limbs: vec![1] }
    }
    fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }
    fn normalize(mut self) -> Self {
        while self.limbs.last() == Some(&0) {
            self.limbs.pop();
        }
        self
    }
    fn bits(&self) -> u64 {
        match self.limbs.last() {
            None => 0,
            Some(&top) => (self.limbs.len() as u64 - 1) * 32 + (32 - top.leading_zeros() as u64),
        }
    }
    fn add_ref(&self, o: &BigUint) -> BigUint {
        let (a, b) = if self.limbs.len() >= o.limbs.len() { (self, o) } else { (o, self) };
        let mut out = Vec::with_capacity(a.limbs.len() + 1);
        let mut carry = 0u64;
        for i in 0..a.limbs.len() {
            let s = a.limbs[i] as u64 + *b.limbs.get(i).unwrap_or(&0) as u64 + carry;
            out.push(s as u32);
            carry = s >> 32;
        }
        if carry > 0 {
            out.push(carry as u32);
        }
        BigUint { limbs: out }
    }
    // requires self >= o
    fn sub_ref(&self, o: &BigUint) -> BigUint {
        assert!(*self >= *o, "BigUint subtraction underflow");
        let mut out = Vec::with_capacity(self.limbs.len());
        let mut borrow = 0i64;
        for (i, &x) in self.limbs.iter().enumerate() {
            let mut d = x as i64 - borrow - *o.limbs.get(i).unwrap_or(&0) as i64;
            if d < 0 {
                d += 1 << 32;
                borrow = 1;
            } else {
                borrow = 0;
            }
            out.push(d as u32);
        }
        BigUint { limbs: out }.normalize()
    }
    fn mul_ref(&self, o: &BigUint) -> BigUint {
        if self.is_zero() || o.is_zero() {
            return BigUint::zero();
        }
        let mut out = vec![0u32; self.limbs.len() + o.limbs.len()];
        for (i, &a) in self.limbs.iter().enumerate() {
            let mut carry = 0u64;
            for (j, &b) in o.limbs.iter().enumerate() {
                let t = a as u64 * b as u64 + out[i + j] as u64 + carry;
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
    fn divmod_small(&self, d: u32) -> (BigUint, u32) {
        assert!(d != 0, "division by zero");
        let mut out = vec![0u32; self.limbs.len()];
        let mut rem = 0u64;
        for i in (0..self.limbs.len()).rev() {
            let cur = (rem << 32) | self.limbs[i] as u64;
            out[i] = (cur / d as u64) as u32;
            rem = cur % d as u64;
        }
        (BigUint { limbs: out }.normalize(), rem as u32)
    }
    // schoolbook binary long division (simple and obviously correct)
    fn divmod(&self, d: &BigUint) -> (BigUint, BigUint) {
        assert!(!d.is_zero(), "division by zero");
        if self < d {
            return (BigUint::zero(), self.clone());
        }
        if d.limbs.len() == 1 {
            let (q, r) = self.divmod_small(d.limbs[0]);
            return (q, BigUint::from(r as u64));
        }
        let shift = self.bits() - d.bits();
        let mut rem = self.clone();
        let mut q = BigUint::zero();
        let mut dd = d.clone() << shift as usize;
        for s in (0..=shift).rev() {
            if rem >= dd {
                rem = rem.sub_ref(&dd);
                q = q.set_bit(s);
            }
            dd = dd >> 1;
        }
        (q, rem)
    }
    fn set_bit(mut self, b: u64) -> BigUint {
        let li = (b / 32) as usize;
        if self.limbs.len() <= li {
            self.limbs.resize(li + 1, 0);
        }
        self.limbs[li] |= 1 << (b % 32);
        self
    }
    fn pow(&self, mut e: u32) -> BigUint {
        let mut base = self.clone();
        let mut acc = BigUint::one();
        while e > 0 {
            if e & 1 == 1 {
                acc = acc.mul_ref(&base);
            }
            base = base.mul_ref(&base);
            e >>= 1;
        }
        acc
    }
    fn to_u128(&self) -> Option<u128> {
        if self.limbs.len() > 4 {
            return None;
        }
        Some(self.limbs.iter().rev().fold(0u128, |a, &l| (a << 32) | l as u128))
    }
    fn isqrt(&self) -> BigUint {
        if self.is_zero() {
            return BigUint::zero();
        }
        // Newton iteration
        let mut x = BigUint::one() << ((self.bits() / 2 + 1) as usize);
        loop {
            let y = (x.clone() + self / &x) >> 1;
            if y >= x {
                return x;
            }
            x = y;
        }
    }
    fn gcd(a: &BigUint, b: &BigUint) -> BigUint {
        let (mut a, mut b) = (a.clone(), b.clone());
        while !b.is_zero() {
            let r = &a % &b;
            a = b;
            b = r;
        }
        a
    }
}

impl From<u64> for BigUint {
    fn from(v: u64) -> Self {
        BigUint { limbs: vec![v as u32, (v >> 32) as u32] }.normalize()
    }
}

impl From<u128> for BigUint {
    fn from(v: u128) -> Self {
        BigUint { limbs: (0..4).map(|i| (v >> (32 * i)) as u32).collect() }.normalize()
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

impl Shl<usize> for BigUint {
    type Output = BigUint;
    fn shl(self, n: usize) -> BigUint {
        if self.is_zero() {
            return self;
        }
        let (words, bits) = (n / 32, n % 32);
        let mut out = vec![0u32; words];
        let mut carry = 0u32;
        for &l in &self.limbs {
            if bits == 0 {
                out.push(l);
            } else {
                out.push((l << bits) | carry);
                carry = l >> (32 - bits);
            }
        }
        if carry != 0 {
            out.push(carry);
        }
        BigUint { limbs: out }
    }
}

impl Shr<usize> for BigUint {
    type Output = BigUint;
    fn shr(self, n: usize) -> BigUint {
        let (words, bits) = (n / 32, n % 32);
        if words >= self.limbs.len() {
            return BigUint::zero();
        }
        let src = &self.limbs[words..];
        let mut out = Vec::with_capacity(src.len());
        for i in 0..src.len() {
            let hi = if bits == 0 { 0 } else { src.get(i + 1).map_or(0, |&h| h << (32 - bits)) };
            out.push((src[i] >> bits) | hi);
        }
        BigUint { limbs: out }.normalize()
    }
}

macro_rules! forward_binop {
    ($tr:ident, $m:ident, $imp:ident) => {
        impl $tr<BigUint> for BigUint {
            type Output = BigUint;
            fn $m(self, o: BigUint) -> BigUint {
                self.$imp(&o)
            }
        }
        impl<'a> $tr<&'a BigUint> for BigUint {
            type Output = BigUint;
            fn $m(self, o: &'a BigUint) -> BigUint {
                self.$imp(o)
            }
        }
        impl<'a, 'b> $tr<&'b BigUint> for &'a BigUint {
            type Output = BigUint;
            fn $m(self, o: &'b BigUint) -> BigUint {
                self.$imp(o)
            }
        }
    };
}
forward_binop!(Add, add, add_ref);
forward_binop!(Sub, sub, sub_ref);
forward_binop!(Mul, mul, mul_ref);

impl<'a, 'b> Div<&'b BigUint> for &'a BigUint {
    type Output = BigUint;
    fn div(self, o: &'b BigUint) -> BigUint {
        self.divmod(o).0
    }
}
impl<'a, 'b> Rem<&'b BigUint> for &'a BigUint {
    type Output = BigUint;
    fn rem(self, o: &'b BigUint) -> BigUint {
        self.divmod(o).1
    }
}
impl AddAssign<&BigUint> for BigUint {
    fn add_assign(&mut self, o: &BigUint) {
        *self = self.add_ref(o);
    }
}
impl MulAssign<u32> for BigUint {
    fn mul_assign(&mut self, o: u32) {
        let mut carry = 0u64;
        for l in self.limbs.iter_mut() {
            let t = *l as u64 * o as u64 + carry;
            *l = t as u32;
            carry = t >> 32;
        }
        if carry > 0 {
            self.limbs.push(carry as u32);
        }
        if o == 0 {
            self.limbs.clear();
        }
    }
}

impl Sum for BigUint {
    fn sum<I: Iterator<Item = BigUint>>(it: I) -> BigUint {
        it.fold(BigUint::zero(), |a, b| a + b)
    }
}
impl<'a> Product<&'a BigUint> for BigUint {
    fn product<I: Iterator<Item = &'a BigUint>>(it: I) -> BigUint {
        it.fold(BigUint::one(), |a, b| a * b)
    }
}

impl fmt::Display for BigUint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_zero() {
            return f.pad_integral(true, "", "0");
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
        f.pad_integral(true, "", &s)
    }
}

impl fmt::LowerHex for BigUint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        for (i, l) in self.limbs.iter().rev().enumerate() {
            if i == 0 {
                s += &format!("{:x}", l);
            } else {
                s += &format!("{:08x}", l);
            }
        }
        if s.is_empty() {
            s.push('0');
        }
        f.pad_integral(true, "0x", &s)
    }
}

#[derive(Debug, PartialEq)]
enum ParseBigError {
    Empty,
    InvalidDigit { pos: usize, ch: char },
}

impl fmt::Display for ParseBigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "cannot parse integer from empty string"),
            Self::InvalidDigit { pos, ch } => write!(f, "invalid digit {:?} at position {}", ch, pos),
        }
    }
}

impl FromStr for BigUint {
    type Err = ParseBigError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ParseBigError::Empty);
        }
        let mut acc = BigUint::zero();
        for (pos, ch) in s.char_indices() {
            if ch == '_' {
                continue;
            }
            let d = ch.to_digit(10).ok_or(ParseBigError::InvalidDigit { pos, ch })?;
            acc *= 10;
            acc += &BigUint::from(d as u64);
        }
        Ok(acc)
    }
}

impl BigInt {
    fn from_parts(neg: bool, mag: BigUint) -> Self {
        let neg = neg && !mag.is_zero();
        BigInt { neg, mag }
    }
}

impl From<i64> for BigInt {
    fn from(v: i64) -> Self {
        BigInt::from_parts(v < 0, BigUint::from(v.unsigned_abs()))
    }
}

impl FromStr for BigInt {
    type Err = ParseBigError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.strip_prefix('-') {
            Some(rest) => Ok(BigInt::from_parts(true, rest.parse()?)),
            None => Ok(BigInt::from_parts(false, s.trim_start_matches('+').parse()?)),
        }
    }
}

impl Add for BigInt {
    type Output = BigInt;
    fn add(self, o: BigInt) -> BigInt {
        if self.neg == o.neg {
            return BigInt::from_parts(self.neg, self.mag + o.mag);
        }
        match self.mag.cmp(&o.mag) {
            Ordering::Less => BigInt::from_parts(o.neg, o.mag - self.mag),
            _ => BigInt::from_parts(self.neg, self.mag - o.mag),
        }
    }
}
impl Neg for BigInt {
    type Output = BigInt;
    fn neg(self) -> BigInt {
        BigInt::from_parts(!self.neg, self.mag)
    }
}
impl Sub for BigInt {
    type Output = BigInt;
    fn sub(self, o: BigInt) -> BigInt {
        self + (-o)
    }
}
impl SubAssign for BigInt {
    fn sub_assign(&mut self, o: BigInt) {
        let me = std::mem::replace(self, BigInt::from(0));
        *self = me - o;
    }
}
impl Mul for BigInt {
    type Output = BigInt;
    fn mul(self, o: BigInt) -> BigInt {
        BigInt::from_parts(self.neg ^ o.neg, self.mag * o.mag)
    }
}
impl Div for BigInt {
    type Output = BigInt;
    // truncating division like Rust's primitive ints
    fn div(self, o: BigInt) -> BigInt {
        BigInt::from_parts(self.neg ^ o.neg, &self.mag / &o.mag)
    }
}
impl Rem for BigInt {
    type Output = BigInt;
    fn rem(self, o: BigInt) -> BigInt {
        BigInt::from_parts(self.neg, &self.mag % &o.mag)
    }
}
impl PartialOrd for BigInt {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
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
impl fmt::Display for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad_integral(!self.neg, "", &self.mag.to_string())
    }
}

fn factorial(n: u32) -> BigUint {
    (1..=n as u64).map(BigUint::from).collect::<Vec<_>>().iter().product()
}

fn fib(n: usize) -> BigUint {
    let (mut a, mut b) = (BigUint::zero(), BigUint::one());
    for _ in 0..n {
        let c = &a + &b;
        a = std::mem::replace(&mut b, c);
    }
    a
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

fn main() {
    let f30 = factorial(30);
    println!("30! = {}", f30);
    println!("30! hex = {:x} bits={}", f30, f30.bits());
    let f100 = factorial(100);
    let s = f100.to_string();
    println!("100! has {} digits, digit sum {}, trailing zeros {}", s.len(), s.bytes().map(|b| (b - b'0') as u32).sum::<u32>(), s.len() - s.trim_end_matches('0').len());
    println!("fib(200) = {}", fib(200));
    println!("2^521-1 = {}", BigUint::from(2u64).pow(521) - BigUint::one());
    let sum: BigUint = (1..=50u64).map(|i| BigUint::from(i).pow(10)).sum();
    println!("sum i^10 for i<=50 = {}", sum);

    // cross-check against u128
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let mut checked = 0;
    for _ in 0..2000 {
        let a = (rng.next() as u128) << 40 | rng.next() as u128 >> 10;
        let b = (rng.next() >> (rng.next() % 60)) as u128 + 1;
        let (ba, bb) = (BigUint::from(a), BigUint::from(b));
        assert_eq!((&ba + &bb).to_u128(), a.checked_add(b));
        assert_eq!((&ba * &bb).to_u128(), a.checked_mul(b));
        let (q, r) = ba.divmod(&bb);
        assert_eq!((q.to_u128().unwrap(), r.to_u128().unwrap()), (a / b, a % b), "divmod {} {}", a, b);
        assert_eq!(ba.to_string(), a.to_string());
        assert_eq!(ba.to_string().parse::<BigUint>().unwrap(), ba);
        if a >= b {
            assert_eq!((&ba - &bb).to_u128(), Some(a - b));
        }
        assert_eq!(ba.clone().cmp(&bb), a.cmp(&b));
        assert_eq!((ba.clone() << 7 >> 7), ba);
        checked += 1;
    }
    println!("u128 cross-checks passed: {}", checked);

    // big division identities
    let a = factorial(60) + BigUint::from(12345u64);
    let b = fib(150);
    let (q, r) = a.divmod(&b);
    assert_eq!(&(&q * &b) + &r, a);
    assert!(r < b);
    println!("60!+12345 = q*fib(150)+r: q={} r={}", q, r);
    println!("isqrt(10^50) = {}", BigUint::from(10u64).pow(50).isqrt());
    println!("isqrt(2*10^40) = {}", (BigUint::from(2u64) * BigUint::from(10u64).pow(40)).isqrt());
    println!("gcd(fib(120), fib(90)) = {} (fib(30) = {})", BigUint::gcd(&fib(120), &fib(90)), fib(30));

    // signed
    let x: BigInt = "-123456789012345678901234567890".parse().unwrap();
    let y: BigInt = "987654321098765432109876543210".parse().unwrap();
    println!("x+y={} x-y={} x*y={}", x.clone() + y.clone(), x.clone() - y.clone(), x.clone() * y.clone());
    println!("y/x={} y%x={} x/7={} x%7={}", y.clone() / x.clone(), y.clone() % x.clone(), x.clone() / BigInt::from(7), x.clone() % BigInt::from(7));
    let mut z = BigInt::from(5);
    z -= BigInt::from(12);
    println!("z={} |{:>8}| |{:<8}| |{:+}| |{:08}|", z, z, z, BigInt::from(42), BigInt::from(-42));
    let mut v: Vec<BigInt> = ["5", "-3", "100000000000000000000", "-100000000000000000000", "0", "-0", "+7"].iter().map(|s| s.parse().unwrap()).collect();
    v.sort();
    println!("{}", v.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(" < "));
    for bad in ["", "12a4", "-", "1 2"] {
        match bad.parse::<BigInt>() {
            Ok(v) => println!("{:?} -> {}", bad, v),
            Err(e) => println!("{:?} -> error: {} ({:?})", bad, e, e),
        }
    }
    // checked comparison with primitive overflow semantics
    let m = BigUint::from(u64::MAX) * BigUint::from(u64::MAX);
    println!("u64::MAX^2 = {} = {:#x} fits u128: {:?}", m, m, m.to_u128());
    println!("{}", BigUint::from(u128::MAX) + BigUint::one());
}
