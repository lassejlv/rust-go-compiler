use std::fmt::Display;
use std::ops::{Add, Div, Mul, Sub, Rem};

trait Num: Copy + PartialOrd + Display + Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Div<Output = Self> {
    fn zero() -> Self;
    fn one() -> Self;
    fn from_i32(v: i32) -> Self;
    fn to_f64(self) -> f64;
    fn abs(self) -> Self { if self < Self::zero() { Self::zero() - self } else { self } }
}

macro_rules! impl_num_int {
    ($($t:ty),*) => { $(
        impl Num for $t {
            fn zero() -> Self { 0 }
            fn one() -> Self { 1 }
            fn from_i32(v: i32) -> Self { v as $t }
            fn to_f64(self) -> f64 { self as f64 }
        }
    )* };
}
macro_rules! impl_num_float {
    ($($t:ty),*) => { $(
        impl Num for $t {
            fn zero() -> Self { 0.0 }
            fn one() -> Self { 1.0 }
            fn from_i32(v: i32) -> Self { v as $t }
            fn to_f64(self) -> f64 { self as f64 }
        }
    )* };
}
impl_num_int!(i8, i32, i64, u16, u64, usize);
impl_num_float!(f32, f64);

fn sum<T: Num>(xs: &[T]) -> T { xs.iter().fold(T::zero(), |a, &b| a + b) }
fn product<T: Num>(xs: &[T]) -> T { xs.iter().fold(T::one(), |a, &b| a * b) }
fn mean<T: Num>(xs: &[T]) -> f64 { sum(xs).to_f64() / xs.len() as f64 }
fn max_of<T: Num>(xs: &[T]) -> Option<T> {
    let mut it = xs.iter().copied();
    let first = it.next()?;
    Some(it.fold(first, |m, x| if x > m { x } else { m }))
}
fn pow<T: Num>(base: T, mut e: u32) -> T {
    let mut r = T::one();
    let mut b = base;
    while e > 0 {
        if e & 1 == 1 { r = r * b; }
        e >>= 1;
        if e > 0 { b = b * b; }
    }
    r
}
fn gcd<T: Num + Rem<Output = T>>(a: T, b: T) -> T { if b == T::zero() { a } else { gcd(b, a % b) } }
fn poly_eval<T: Num>(coeffs: &[T], x: T) -> T { coeffs.iter().rev().fold(T::zero(), |acc, &c| acc * x + c) }
fn show<T: Num>(label: &str, xs: &[T]) {
    println!("{}: sum={} prod={} mean={:.3} max={} absfirst={}", label, sum(xs), product(xs), mean(xs),
        max_of(xs).map(|m| m.to_string()).unwrap_or("-".into()), xs[0].abs());
}
fn range_step<T: Num>(start: T, end: T, step: T) -> Vec<T> {
    let mut v = vec![]; let mut c = start;
    while c < end { v.push(c); c = c + step; }
    v
}

fn main() {
    show("i8", &[-3i8, 2, 1]);
    show("i32", &[4, -5, 6]);
    show("i64", &[100000i64, 3]);
    show("u16", &[7u16, 8, 9]);
    show("u64", &[1u64 << 20, 3]);
    show("usize", &[2usize, 2, 2]);
    show("f32", &[-1.5f32, 2.25]);
    show("f64", &[0.1, 0.2, 0.3]);
    println!("{} {} {} {}", pow(3i64, 13), pow(2u64, 40), pow(1.5f64, 3), pow(2.0f32, 10));
    println!("{} {} {}", gcd(48i32, 18), gcd(1071u64, 462), gcd(17usize, 5));
    println!("{} {}", poly_eval(&[1, 2, 3], 10), poly_eval(&[0.5, 0.0, 2.0], 1.5));
    println!("{:?}", range_step(0.0, 1.0, 0.25));
    println!("{:?}", range_step(T3::from_i32(1), 10, 3));
    println!("{}", <f64 as Num>::from_i32(-7).abs());
}
type T3 = i32;
