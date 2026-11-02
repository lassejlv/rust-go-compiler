use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a.abs() } else { gcd(b, a % b) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Frac {
    n: i64,
    d: i64,
}

impl Frac {
    fn new(n: i64, d: i64) -> Frac {
        assert!(d != 0, "zero denominator");
        let g = gcd(n, d);
        let s = if d < 0 { -1 } else { 1 };
        Frac { n: s * n / g, d: s * d / g }
    }
    fn to_f64(self) -> f64 {
        self.n as f64 / self.d as f64
    }
}

impl fmt::Display for Frac {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.d == 1 {
            write!(f, "{}", self.n)
        } else {
            write!(f, "{}/{}", self.n, self.d)
        }
    }
}

impl Add for Frac {
    type Output = Frac;
    fn add(self, o: Frac) -> Frac {
        Frac::new(self.n * o.d + o.n * self.d, self.d * o.d)
    }
}

impl Sub for Frac {
    type Output = Frac;
    fn sub(self, o: Frac) -> Frac {
        self + (-o)
    }
}

impl Mul for Frac {
    type Output = Frac;
    fn mul(self, o: Frac) -> Frac {
        Frac::new(self.n * o.n, self.d * o.d)
    }
}

impl Div for Frac {
    type Output = Frac;
    fn div(self, o: Frac) -> Frac {
        Frac::new(self.n * o.d, self.d * o.n)
    }
}

impl Neg for Frac {
    type Output = Frac;
    fn neg(self) -> Frac {
        Frac { n: -self.n, d: self.d }
    }
}

impl PartialOrd for Frac {
    fn partial_cmp(&self, o: &Frac) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl Ord for Frac {
    fn cmp(&self, o: &Frac) -> Ordering {
        (self.n * o.d).cmp(&(o.n * self.d))
    }
}

impl From<i64> for Frac {
    fn from(n: i64) -> Frac {
        Frac::new(n, 1)
    }
}

fn main() {
    let a = Frac::new(1, 2);
    let b = Frac::new(3, 4);
    println!("{} + {} = {}", a, b, a + b);
    println!("{} - {} = {}", a, b, a - b);
    println!("{} * {} = {}", a, b, a * b);
    println!("{} / {} = {}", a, b, a / b);
    println!("-{} = {}", a, -a);
    println!("normalize 6/-8 = {}", Frac::new(6, -8));
    println!("4/2 = {}", Frac::new(4, 2));
    println!("a < b: {}", a < b);
    let mut h = Frac::from(0);
    for k in 1..=10 {
        h = h + Frac::new(1, k);
    }
    println!("H(10) = {} ~ {:.6}", h, h.to_f64());
    let mut fs = vec![Frac::new(2, 3), Frac::new(-1, 5), Frac::new(5, 8), Frac::new(1, 3), Frac::from(1)];
    fs.sort();
    let strs: Vec<String> = fs.iter().map(|f| f.to_string()).collect();
    println!("sorted: {}", strs.join(" < "));
    println!("max: {}", fs.iter().max().unwrap());
    let mut x = Frac::from(1);
    for _ in 0..6 {
        x = Frac::from(1) + Frac::from(1) / x;
    }
    println!("continued fraction for phi: {} ~ {:.5}", x, x.to_f64());
    println!("eq: {}", Frac::new(2, 4) == Frac::new(1, 2));
}
