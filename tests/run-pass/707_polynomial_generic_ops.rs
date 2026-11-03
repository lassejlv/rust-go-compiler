// Generic polynomial arithmetic over a Ring trait, with operator overloading
// on both owned values and references, plus a modular-integer ring.
use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

trait Ring: Clone + PartialEq + fmt::Display + Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Neg<Output = Self> {
    fn zero() -> Self;
    fn one() -> Self;
    fn is_zero(&self) -> bool { *self == Self::zero() }
}

impl Ring for i64 {
    fn zero() -> Self { 0 }
    fn one() -> Self { 1 }
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct Mod<const M: u64>(u64);

impl<const M: u64> Mod<M> {
    fn new(v: i64) -> Self { Mod(v.rem_euclid(M as i64) as u64) }
    fn pow(self, mut e: u64) -> Self {
        let mut b = self;
        let mut r = Mod(1);
        while e > 0 {
            if e & 1 == 1 { r = r * b; }
            b = b * b;
            e >>= 1;
        }
        r
    }
    fn inv(self) -> Self { self.pow(M - 2) }
}
impl<const M: u64> Add for Mod<M> { type Output = Self; fn add(self, o: Self) -> Self { Mod((self.0 + o.0) % M) } }
impl<const M: u64> Sub for Mod<M> { type Output = Self; fn sub(self, o: Self) -> Self { Mod((self.0 + M - o.0) % M) } }
impl<const M: u64> Mul for Mod<M> { type Output = Self; fn mul(self, o: Self) -> Self { Mod(self.0 * o.0 % M) } }
impl<const M: u64> Neg for Mod<M> { type Output = Self; fn neg(self) -> Self { Mod((M - self.0) % M) } }
impl<const M: u64> fmt::Display for Mod<M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.0) }
}
impl<const M: u64> Ring for Mod<M> {
    fn zero() -> Self { Mod(0) }
    fn one() -> Self { Mod(1) }
}

#[derive(Clone, PartialEq, Debug)]
struct Poly<T> { c: Vec<T> } // c[i] is coefficient of x^i

impl<T: Ring> Poly<T> {
    fn new(mut c: Vec<T>) -> Self {
        while c.last().map_or(false, |x| x.is_zero()) { c.pop(); }
        Poly { c }
    }
    fn degree(&self) -> Option<usize> { if self.c.is_empty() { None } else { Some(self.c.len() - 1) } }
    fn eval(&self, x: &T) -> T {
        self.c.iter().rev().fold(T::zero(), |acc, k| acc * x.clone() + k.clone())
    }
    fn derivative(&self) -> Self where T: From<i64> {
        Poly::new(self.c.iter().enumerate().skip(1).map(|(i, k)| T::from(i as i64) * k.clone()).collect())
    }
    fn compose(&self, other: &Poly<T>) -> Poly<T> {
        let mut acc = Poly::new(vec![]);
        for k in self.c.iter().rev() {
            acc = &(&acc * other) + &Poly::new(vec![k.clone()]);
        }
        acc
    }
}

impl<'a, T: Ring> Add for &'a Poly<T> {
    type Output = Poly<T>;
    fn add(self, o: &Poly<T>) -> Poly<T> {
        let n = self.c.len().max(o.c.len());
        Poly::new((0..n).map(|i| {
            let a = self.c.get(i).cloned().unwrap_or_else(T::zero);
            let b = o.c.get(i).cloned().unwrap_or_else(T::zero);
            a + b
        }).collect())
    }
}
impl<'a, T: Ring> Sub for &'a Poly<T> {
    type Output = Poly<T>;
    fn sub(self, o: &Poly<T>) -> Poly<T> { self + &(-o) }
}
impl<'a, T: Ring> Neg for &'a Poly<T> {
    type Output = Poly<T>;
    fn neg(self) -> Poly<T> { Poly::new(self.c.iter().cloned().map(|x| -x).collect()) }
}
impl<'a, T: Ring> Mul for &'a Poly<T> {
    type Output = Poly<T>;
    fn mul(self, o: &Poly<T>) -> Poly<T> {
        if self.c.is_empty() || o.c.is_empty() { return Poly::new(vec![]); }
        let mut r = vec![T::zero(); self.c.len() + o.c.len() - 1];
        for (i, a) in self.c.iter().enumerate() {
            for (j, b) in o.c.iter().enumerate() {
                r[i + j] = r[i + j].clone() + a.clone() * b.clone();
            }
        }
        Poly::new(r)
    }
}
impl<T: Ring> Add for Poly<T> { type Output = Poly<T>; fn add(self, o: Self) -> Self { &self + &o } }
impl<T: Ring> Mul for Poly<T> { type Output = Poly<T>; fn mul(self, o: Self) -> Self { &self * &o } }

impl<T: Ring> fmt::Display for Poly<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.c.is_empty() { return write!(f, "0"); }
        let mut first = true;
        for (i, k) in self.c.iter().enumerate().rev() {
            if k.is_zero() { continue; }
            if !first { write!(f, " + ")?; }
            first = false;
            match i {
                0 => write!(f, "{}", k)?,
                1 if *k == T::one() => write!(f, "x")?,
                1 => write!(f, "{}x", k)?,
                _ if *k == T::one() => write!(f, "x^{}", i)?,
                _ => write!(f, "{}x^{}", k, i)?,
            }
        }
        Ok(())
    }
}

type F7 = Mod<7>;
type P97 = Mod<97>;

fn interpolate<const M: u64>(pts: &[(i64, i64)]) -> Poly<Mod<M>> {
    let mut res = Poly::new(vec![]);
    for (i, &(xi, yi)) in pts.iter().enumerate() {
        let mut term = Poly::new(vec![Mod::<M>::new(yi)]);
        for (j, &(xj, _)) in pts.iter().enumerate() {
            if i == j { continue; }
            let denom = (Mod::<M>::new(xi) - Mod::new(xj)).inv();
            let lin = Poly::new(vec![-Mod::new(xj) * denom, denom]);
            term = &term * &lin;
        }
        res = res + term;
    }
    res
}

impl<const M: u64> From<i64> for Mod<M> { fn from(v: i64) -> Self { Mod::new(v) } }

fn main() {
    let p = Poly::new(vec![1i64, -3, 0, 2]);
    let q = Poly::new(vec![-1i64, 1]);
    println!("p = {}", p);
    println!("q = {}", q);
    println!("p+q = {}", &p + &q);
    println!("p-q = {}", &p - &q);
    println!("p*q = {}", &p * &q);
    println!("p-p = {} degree {:?}", &p - &p, (&p - &p).degree());
    println!("p' = {}  p'' = {}", p.derivative(), p.derivative().derivative());
    println!("p(q(x)) = {}", p.compose(&q));
    for x in -2..=2 { print!("p({})={} ", x, p.eval(&x)); }
    println!();
    let a = Poly::new(vec![F7::new(3), F7::new(5), F7::new(1)]);
    let b = Poly::new(vec![F7::new(-1), F7::new(2)]);
    println!("over F7: a={}  b={}  a*b={}  a+b={}", a, b, &a * &b, &a + &b);
    println!("a(x) for x in F7: {:?}", (0..7).map(|x| a.eval(&F7::new(x)).0).collect::<Vec<_>>());
    println!("inverses mod 7: {:?}", (1..7).map(|x| F7::new(x).inv().0).collect::<Vec<_>>());
    let pts = [(1, 5), (2, 11), (3, 21), (4, 35)];
    let ip = interpolate::<97>(&pts);
    println!("interp mod 97: {} degree {:?}", ip, ip.degree());
    println!("check: {:?}", pts.iter().map(|&(x, _)| ip.eval(&P97::new(x)).0).collect::<Vec<_>>());
    println!("debug: {:?}", Poly::new(vec![F7::new(10), F7::new(0)]));
    let big = (0..5).fold(Poly::new(vec![1i64]), |acc, _| &acc * &Poly::new(vec![1, 1]));
    println!("(1+x)^5 = {}", big);
    println!("2^100 mod 97 = {}", P97::new(2).pow(100));
}
