use std::ops::{Add, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq)]
struct Vec2 {
    x: f64,
    y: f64,
}

impl Vec2 {
    fn new(x: f64, y: f64) -> Self {
        Vec2 { x, y }
    }
    fn dot(self, o: Vec2) -> f64 {
        self.x * o.x + self.y * o.y
    }
    fn length(self) -> f64 {
        self.dot(self).sqrt()
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x + o.x, self.y + o.y)
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x - o.x, self.y - o.y)
    }
}

impl Mul<f64> for Vec2 {
    type Output = Vec2;
    fn mul(self, k: f64) -> Vec2 {
        Vec2::new(self.x * k, self.y * k)
    }
}

impl Mul<Vec2> for f64 {
    type Output = Vec2;
    fn mul(self, v: Vec2) -> Vec2 {
        v * self
    }
}

impl Neg for Vec2 {
    type Output = Vec2;
    fn neg(self) -> Vec2 {
        Vec2::new(-self.x, -self.y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Complex {
    re: i64,
    im: i64,
}

impl Add for Complex {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Complex { re: self.re + o.re, im: self.im + o.im }
    }
}

impl Mul for Complex {
    type Output = Self;
    fn mul(self, o: Self) -> Self {
        Complex { re: self.re * o.re - self.im * o.im, im: self.re * o.im + self.im * o.re }
    }
}

impl<'a> Add<&'a Complex> for &'a Complex {
    type Output = Complex;
    fn add(self, o: &Complex) -> Complex {
        *self + *o
    }
}

fn main() {
    let a = Vec2::new(1.0, 2.0);
    let b = Vec2::new(3.0, -1.0);
    println!("a + b = {:?}", a + b);
    println!("a - b = {:?}", a - b);
    println!("a * 3 = {:?}", a * 3.0);
    println!("2 * b = {:?}", 2.0 * b);
    println!("-a = {:?}", -a);
    println!("dot = {}", a.dot(b));
    println!("len = {}", Vec2::new(3.0, 4.0).length());
    println!("combo = {:?}", (a + b) * 0.5 - -a);
    println!("eq = {}", a + b == Vec2::new(4.0, 1.0));
    let i = Complex { re: 0, im: 1 };
    println!("i * i = {:?}", i * i);
    let z = Complex { re: 2, im: 3 };
    println!("z + i = {:?}", z + i);
    println!("&z + &i = {:?}", &z + &i);
    let mut p = Complex { re: 1, im: 0 };
    for k in 1..=4 {
        p = p * z;
        println!("z^{} = {} + {}i", k, p.re, p.im);
    }
    let sum = [a, b, Vec2::new(0.5, 0.5)].iter().fold(Vec2::new(0.0, 0.0), |acc, v| acc + *v);
    println!("sum = {:?}", sum);
}
