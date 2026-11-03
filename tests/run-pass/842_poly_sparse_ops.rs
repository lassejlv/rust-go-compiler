use std::collections::BTreeMap;
use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

#[derive(Clone, PartialEq, Debug, Default)]
struct Poly { terms: BTreeMap<u32, i64> }

impl Poly {
    fn new(pairs: &[(u32, i64)]) -> Poly { let mut p = Poly::default(); for &(e, c) in pairs { *p.terms.entry(e).or_insert(0) += c; } p.normalize() }
    fn normalize(mut self) -> Poly { self.terms.retain(|_, c| *c != 0); self }
    fn degree(&self) -> Option<u32> { self.terms.keys().next_back().copied() }
    fn eval(&self, x: i64) -> i64 { self.terms.iter().map(|(&e, &c)| c * x.pow(e)).sum() }
    fn derive(&self) -> Poly { Poly { terms: self.terms.iter().filter(|(&e, _)| e > 0).map(|(&e, &c)| (e - 1, c * e as i64)).collect() } }
    fn compose(&self, other: &Poly) -> Poly {
        let mut result = Poly::default();
        for (&e, &c) in &self.terms {
            let mut p = Poly::new(&[(0, c)]);
            for _ in 0..e { p = &p * other; }
            result = &result + &p;
        }
        result
    }
    fn div_rem(&self, d: &Poly) -> Option<(Poly, Poly)> {
        let dd = d.degree()?;
        let lead = d.terms[&dd];
        let mut q = Poly::default();
        let mut r = self.clone();
        while let Some(rd) = r.degree() {
            if rd < dd { break; }
            let rc = r.terms[&rd];
            if rc % lead != 0 { return None; }
            let t = Poly::new(&[(rd - dd, rc / lead)]);
            q = &q + &t;
            r = &r - &(&t * d);
        }
        Some((q, r))
    }
}

impl<'a> Add for &'a Poly { type Output = Poly; fn add(self, o: &Poly) -> Poly { let mut p = self.clone(); for (&e, &c) in &o.terms { *p.terms.entry(e).or_insert(0) += c; } p.normalize() } }
impl<'a> Sub for &'a Poly { type Output = Poly; fn sub(self, o: &Poly) -> Poly { self + &(-o) } }
impl<'a> Neg for &'a Poly { type Output = Poly; fn neg(self) -> Poly { Poly { terms: self.terms.iter().map(|(&e, &c)| (e, -c)).collect() } } }
impl<'a> Mul for &'a Poly {
    type Output = Poly;
    fn mul(self, o: &Poly) -> Poly {
        let mut p = Poly::default();
        for (&e1, &c1) in &self.terms { for (&e2, &c2) in &o.terms { *p.terms.entry(e1 + e2).or_insert(0) += c1 * c2; } }
        p.normalize()
    }
}
impl Add for Poly { type Output = Poly; fn add(self, o: Poly) -> Poly { &self + &o } }
impl Mul<i64> for Poly { type Output = Poly; fn mul(self, k: i64) -> Poly { Poly { terms: self.terms.into_iter().map(|(e, c)| (e, c * k)).collect() }.normalize() } }

impl fmt::Display for Poly {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.terms.is_empty() { return f.pad("0"); }
        let mut s = String::new();
        for (i, (&e, &c)) in self.terms.iter().rev().enumerate() {
            let sign = if c < 0 { "-" } else if i > 0 { "+" } else { "" };
            if i > 0 { s.push(' '); }
            s.push_str(sign);
            if i > 0 { s.push(' '); }
            let a = c.abs();
            let coef = if a == 1 && e > 0 { String::new() } else { a.to_string() };
            let var = match e { 0 => String::new(), 1 => "x".into(), _ => format!("x^{}", e) };
            s += &coef; s += &var;
        }
        f.pad(&s)
    }
}

fn main() {
    let p = Poly::new(&[(2, 1), (1, -3), (0, 2)]);
    let q = Poly::new(&[(1, 1), (0, -1)]);
    println!("p = {}\nq = {}", p, q);
    println!("p+q = {}\np-q = {}\np*q = {}", &p + &q, &p - &q, &p * &q);
    println!("-p = {} | p' = {} | p'' = {} | p''' = {}", -&p, p.derive(), p.derive().derive(), p.derive().derive().derive());
    println!("p(q(x)) = {}", p.compose(&q));
    println!("q(p(x)) = {}", q.compose(&p));
    for x in -2..=3 { print!("p({})={} ", x, p.eval(x)); }
    println!();
    match p.div_rem(&q) { Some((d, r)) => println!("p / q = {} rem {}", d, r), None => println!("not integral") }
    let cube = &(&q * &q) * &q;
    println!("(x-1)^3 = {}", cube);
    let big = Poly::new(&[(5, 3), (3, -2), (0, 7), (5, -3), (1, 4)]);
    println!("big = {} degree {:?}", big, big.degree());
    println!("big / (2x) = {:?}", big.div_rem(&Poly::new(&[(1, 2)])).map(|(a, b)| (a.to_string(), b.to_string())));
    println!("zero: [{:>6}] [{:<6}] deg={:?}", Poly::default(), &p - &p, (&p - &p).degree());
    println!("[{:^20}]", q);
    let sum: Poly = (1..=4).map(|k| Poly::new(&[(k, k as i64)])).fold(Poly::default(), |a, b| a + b);
    println!("sum = {} scaled = {}", sum, sum.clone() * -2);
    let mut cheb = vec![Poly::new(&[(0, 1)]), Poly::new(&[(1, 1)])];
    for n in 2..7 { let next = &(&Poly::new(&[(1, 2)]) * &cheb[n - 1]) - &cheb[n - 2]; cheb.push(next); }
    for (i, t) in cheb.iter().enumerate() { println!("T{} = {} ; T{}(1)={}", i, t, i, t.eval(1)); }
}
