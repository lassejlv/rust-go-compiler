use std::fmt::{Debug, Display};
use std::ops::{Add, Div, Mul, Rem, Sub};

pub trait Zero: Sized + Add<Self, Output = Self> {
    fn zero() -> Self;
    fn is_zero(&self) -> bool;
}
pub trait One: Sized + Mul<Self, Output = Self> {
    fn one() -> Self;
}
pub trait NumOps<Rhs = Self, Output = Self>:
    Add<Rhs, Output = Output> + Sub<Rhs, Output = Output> + Mul<Rhs, Output = Output> + Div<Rhs, Output = Output> + Rem<Rhs, Output = Output>
{
}
impl<T, Rhs, Output> NumOps<Rhs, Output> for T where
    T: Add<Rhs, Output = Output> + Sub<Rhs, Output = Output> + Mul<Rhs, Output = Output> + Div<Rhs, Output = Output> + Rem<Rhs, Output = Output>
{
}
pub trait Num: PartialEq + Zero + One + NumOps {
    type FromStrRadixErr: Debug;
    fn from_str_radix(s: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr>;
}
pub trait Bounded { fn min_value() -> Self; fn max_value() -> Self; }
pub trait CheckedAdd: Sized + Add<Self, Output = Self> { fn checked_add(&self, v: &Self) -> Option<Self>; }
pub trait PrimInt: Num + Bounded + Copy + Ord + CheckedAdd + Display {
    const BITS: u32;
    fn count_ones(self) -> u32;
}

macro_rules! impl_int {
    ($($t:ty),*) => {$(
        impl Zero for $t { fn zero() -> $t { 0 } fn is_zero(&self) -> bool { *self == 0 } }
        impl One for $t { fn one() -> $t { 1 } }
        impl Num for $t {
            type FromStrRadixErr = std::num::ParseIntError;
            fn from_str_radix(s: &str, r: u32) -> Result<$t, Self::FromStrRadixErr> { <$t>::from_str_radix(s, r) }
        }
        impl Bounded for $t { fn min_value() -> $t { <$t>::MIN } fn max_value() -> $t { <$t>::MAX } }
        impl CheckedAdd for $t { fn checked_add(&self, v: &$t) -> Option<$t> { <$t>::checked_add(*self, *v) } }
        impl PrimInt for $t {
            const BITS: u32 = <$t>::BITS;
            fn count_ones(self) -> u32 { <$t>::count_ones(self) }
        }
    )*};
}
impl_int!(u8, i16, u32, i64, u128);

impl Zero for f64 { fn zero() -> f64 { 0.0 } fn is_zero(&self) -> bool { *self == 0.0 } }
impl One for f64 { fn one() -> f64 { 1.0 } }
impl Num for f64 {
    type FromStrRadixErr = String;
    fn from_str_radix(s: &str, r: u32) -> Result<f64, String> {
        if r != 10 { return Err(format!("radix {} unsupported", r)); }
        s.parse().map_err(|_| "bad float".to_string())
    }
}

fn gcd<T: PrimInt>(mut a: T, mut b: T) -> T {
    while !b.is_zero() {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

fn pow<T: Num + Copy>(mut base: T, mut exp: u32) -> T {
    let mut acc = T::one();
    while exp > 0 {
        if exp & 1 == 1 { acc = acc * base; }
        base = base * base;
        exp >>= 1;
    }
    acc
}

fn sum_checked<T: PrimInt>(xs: &[T]) -> Option<T> {
    xs.iter().try_fold(T::zero(), |a, b| a.checked_add(b))
}

fn parse_all<T: Num>(xs: &[&str], radix: u32) -> Vec<Result<T, T::FromStrRadixErr>> {
    xs.iter().map(|s| T::from_str_radix(s, radix)).collect()
}

fn describe<T: PrimInt>() -> String {
    format!("bits={} min={} max={} ones(max)={}", T::BITS, T::min_value(), T::max_value(), T::max_value().count_ones())
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Complex<T> { re: T, im: T }
impl<T: Num + Copy> Add for Complex<T> { type Output = Self; fn add(self, o: Self) -> Self { Complex { re: self.re + o.re, im: self.im + o.im } } }
impl<T: Num + Copy> Sub for Complex<T> { type Output = Self; fn sub(self, o: Self) -> Self { Complex { re: self.re - o.re, im: self.im - o.im } } }
impl<T: Num + Copy> Mul for Complex<T> {
    type Output = Self;
    fn mul(self, o: Self) -> Self { Complex { re: self.re * o.re - self.im * o.im, im: self.re * o.im + self.im * o.re } }
}
impl<T: Num + Copy> Div for Complex<T> {
    type Output = Self;
    fn div(self, o: Self) -> Self {
        let d = o.re * o.re + o.im * o.im;
        Complex { re: (self.re * o.re + self.im * o.im) / d, im: (self.im * o.re - self.re * o.im) / d }
    }
}
impl<T: Num + Copy> Rem for Complex<T> { type Output = Self; fn rem(self, _o: Self) -> Self { Complex { re: T::zero(), im: T::zero() } } }
impl<T: Num + Copy> Zero for Complex<T> {
    fn zero() -> Self { Complex { re: T::zero(), im: T::zero() } }
    fn is_zero(&self) -> bool { self.re.is_zero() && self.im.is_zero() }
}
impl<T: Num + Copy> One for Complex<T> { fn one() -> Self { Complex { re: T::one(), im: T::zero() } } }
impl<T: Num + Copy> Num for Complex<T> {
    type FromStrRadixErr = Option<T::FromStrRadixErr>;
    fn from_str_radix(s: &str, r: u32) -> Result<Self, Self::FromStrRadixErr> {
        let (a, b) = s.split_once(',').ok_or(None)?;
        Ok(Complex { re: T::from_str_radix(a, r).map_err(Some)?, im: T::from_str_radix(b, r).map_err(Some)? })
    }
}

fn main() {
    println!("{} {} {}", gcd(48u32, 18), gcd(-48i64, 18), gcd(255u8, 15));
    println!("{} {} {}", pow(3u128, 40), pow(2.0f64, 10), pow(7i16, 3));
    println!("{:?} {:?}", sum_checked(&[100u8, 100, 50]), sum_checked(&[100u8, 100, 60]));
    println!("{:?}", parse_all::<i64>(&["ff", "-10", "zz"], 16));
    println!("{:?}", parse_all::<f64>(&["1.5", "x"], 10));
    println!("{:?}", parse_all::<f64>(&["1"], 2));
    println!("{}", describe::<u8>());
    println!("{}", describe::<i16>());
    println!("{}", describe::<u128>());
    let i = Complex { re: 0i64, im: 1 };
    println!("{:?}", pow(i, 2));
    println!("{:?}", pow(Complex { re: 1i64, im: 1 }, 8));
    let z = Complex { re: 3.0, im: 4.0 } / Complex { re: 1.0, im: 2.0 };
    println!("{:?}", z);
    println!("{:?}", parse_all::<Complex<i64>>(&["1,2", "3", "a,1"], 10));
    let cc: Complex<Complex<i64>> = Complex { re: Complex::one(), im: Complex::zero() };
    println!("{:?}", pow(cc, 3));
}
