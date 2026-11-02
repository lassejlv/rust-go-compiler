// Operator overloading on generic types: Add/Sub/Mul/Neg/AddAssign/MulAssign/Index/IndexMut,
// ops on references (&a + &b), mixed-type ops (Poly * scalar, scalar * Poly via concrete impls),
// a generic Complex<T> with Num-like trait bound, PartialEq/PartialOrd implementations, and Sum.
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub};

trait Num: Copy + Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Neg<Output = Self> + PartialEq + Default + fmt::Display {}
impl<T> Num for T where T: Copy + Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Neg<Output = T> + PartialEq + Default + fmt::Display {}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Complex<T> {
    re: T,
    im: T,
}

impl<T: Num> Add for Complex<T> {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Complex { re: self.re + o.re, im: self.im + o.im }
    }
}
impl<T: Num> Mul for Complex<T> {
    type Output = Self;
    fn mul(self, o: Self) -> Self {
        Complex { re: self.re * o.re - self.im * o.im, im: self.re * o.im + self.im * o.re }
    }
}
impl<T: Num> Neg for Complex<T> {
    type Output = Self;
    fn neg(self) -> Self {
        Complex { re: -self.re, im: -self.im }
    }
}
impl<'a, T: Num> Add<&'a Complex<T>> for &'a Complex<T> {
    type Output = Complex<T>;
    fn add(self, o: &Complex<T>) -> Complex<T> {
        *self + *o
    }
}
impl<T: Num> AddAssign for Complex<T> {
    fn add_assign(&mut self, o: Self) {
        *self = *self + o;
    }
}
impl<T: Num> Sum for Complex<T> {
    fn sum<I: Iterator<Item = Self>>(it: I) -> Self {
        it.fold(Complex::default(), |a, b| a + b)
    }
}
impl<T: Num + PartialOrd> fmt::Display for Complex<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.im < T::default() {
            write!(f, "{}-{}i", self.re, -self.im)
        } else {
            write!(f, "{}+{}i", self.re, self.im)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Poly(Vec<i64>); // coefficients, lowest first

impl Index<usize> for Poly {
    type Output = i64;
    fn index(&self, i: usize) -> &i64 {
        self.0.get(i).unwrap_or(&0)
    }
}
impl IndexMut<usize> for Poly {
    fn index_mut(&mut self, i: usize) -> &mut i64 {
        if i >= self.0.len() {
            self.0.resize(i + 1, 0);
        }
        &mut self.0[i]
    }
}
impl Mul for &Poly {
    type Output = Poly;
    fn mul(self, o: &Poly) -> Poly {
        let mut r = Poly(vec![]);
        for (i, a) in self.0.iter().enumerate() {
            for (j, b) in o.0.iter().enumerate() {
                r[i + j] += a * b;
            }
        }
        r
    }
}
impl Mul<i64> for Poly {
    type Output = Poly;
    fn mul(self, k: i64) -> Poly {
        Poly(self.0.into_iter().map(|c| c * k).collect())
    }
}
impl Mul<Poly> for i64 {
    type Output = Poly;
    fn mul(self, p: Poly) -> Poly {
        p * self
    }
}
impl MulAssign<i64> for Poly {
    fn mul_assign(&mut self, k: i64) {
        for c in self.0.iter_mut() {
            *c *= k;
        }
    }
}
impl Poly {
    fn eval(&self, x: i64) -> i64 {
        self.0.iter().rev().fold(0, |acc, c| acc * x + c)
    }
}
impl fmt::Display for Poly {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let terms: Vec<String> = self.0.iter().enumerate().rev().filter(|(_, c)| **c != 0).map(|(i, c)| match i {
            0 => format!("{}", c),
            1 => format!("{}x", c),
            _ => format!("{}x^{}", c, i),
        }).collect();
        if terms.is_empty() { write!(f, "0") } else { write!(f, "{}", terms.join(" + ")) }
    }
}

fn main() {
    let a = Complex { re: 1, im: 2 };
    let b = Complex { re: 3, im: -1 };
    println!("{} {} {} {}", a + b, a * b, -a, &a + &b);
    let mut c = a;
    c += b;
    c += Complex { re: -10, im: 0 };
    println!("{}", c);
    let fs = vec![Complex { re: 0.5, im: 0.25 }, Complex { re: 1.5, im: -0.75 }];
    let s: Complex<f64> = fs.into_iter().sum();
    println!("{} {:?}", s, s);
    let mut z = Complex { re: 0i64, im: 0 };
    let cpt = Complex { re: -1, im: 1 };
    for _ in 0..4 {
        z = z * z + cpt;
        print!("{} ", z);
    }
    println!();

    let p = Poly(vec![1, 2]); // 1 + 2x
    let q = Poly(vec![-1, 0, 3]); // -1 + 3x^2
    let pq = &p * &q;
    println!("({}) * ({}) = {}", p, q, pq);
    println!("eval at 2: {} {} {}", p.eval(2), q.eval(2), pq.eval(2));
    let mut r = 3 * pq.clone();
    r *= -1;
    r[6] = 1;
    println!("{} | r[1]={} r[10]={}", r, r[1], r[10]);
    println!("{}", Poly(vec![0, 0]));
    println!("{}", &Poly(vec![1, 1]) * &(&Poly(vec![1, 1]) * &Poly(vec![1, 1])));
}
