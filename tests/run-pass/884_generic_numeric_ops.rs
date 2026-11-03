use std::ops::{Add, Mul, Neg, Sub, AddAssign, Index};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
struct V2<T> { x: T, y: T }

impl<T: Add<Output = T>> Add for V2<T> {
    type Output = V2<T>;
    fn add(self, o: Self) -> V2<T> { V2 { x: self.x + o.x, y: self.y + o.y } }
}
impl<T: Sub<Output = T>> Sub for V2<T> {
    type Output = Self;
    fn sub(self, o: Self) -> Self { V2 { x: self.x - o.x, y: self.y - o.y } }
}
// scalar multiplication with different RHS
impl<T: Mul<Output = T> + Copy> Mul<T> for V2<T> {
    type Output = V2<T>;
    fn mul(self, k: T) -> V2<T> { V2 { x: self.x * k, y: self.y * k } }
}
// dot product: V2 * V2 -> T
impl<T: Mul<Output = T> + Add<Output = T>> Mul<V2<T>> for V2<T> {
    type Output = T;
    fn mul(self, o: V2<T>) -> T { self.x * o.x + self.y * o.y }
}
impl<T: Neg<Output = T>> Neg for V2<T> {
    type Output = Self;
    fn neg(self) -> Self { V2 { x: -self.x, y: -self.y } }
}
impl<T: AddAssign> AddAssign for V2<T> {
    fn add_assign(&mut self, o: Self) { self.x += o.x; self.y += o.y; }
}
impl<T: fmt::Display> fmt::Display for V2<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "({}, {})", self.x, self.y) }
}
impl<T> Index<usize> for V2<T> {
    type Output = T;
    fn index(&self, i: usize) -> &T { if i == 0 { &self.x } else { &self.y } }
}

// Mixed: i32 * V2<i32>
impl Mul<V2<i32>> for i32 {
    type Output = V2<i32>;
    fn mul(self, v: V2<i32>) -> V2<i32> { V2 { x: self * v.x, y: self * v.y } }
}

fn sum_all<T: Add<Output = T> + Copy>(xs: &[T], zero: T) -> T {
    xs.iter().fold(zero, |a, &b| a + b)
}

fn lerp<T>(a: T, b: T, t: T) -> T
where
    T: Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Copy,
{
    a + (b - a) * t
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct Money(i64);
impl Add for Money { type Output = Money; fn add(self, o: Money) -> Money { Money(self.0 + o.0) } }
impl Mul<i64> for Money { type Output = Money; fn mul(self, k: i64) -> Money { Money(self.0 * k) } }

fn main() {
    let a = V2 { x: 1, y: 2 };
    let b = V2 { x: 10, y: 20 };
    println!("{} {} {}", a + b, b - a, a * 3);
    println!("{}", a * b);
    println!("{} {}", -a, 2 * b);
    let mut c = a;
    c += b;
    c += V2 { x: 100, y: 100 };
    println!("{} {} {}", c, c[0], c[1]);
    let f = V2 { x: 0.5, y: 1.5 };
    println!("{} {}", f * 2.0, f * f);
    println!("{}", sum_all(&[a, b, c], V2 { x: 0, y: 0 }));
    println!("{}", sum_all(&[1.5, 2.25], 0.0));
    println!("{}", lerp(0.0, 10.0, 0.25));
    println!("{}", lerp(2, 12, 3));
    println!("{:?}", lerp(1.0f32, 3.0, 0.5));
    let m = sum_all(&[Money(5), Money(7)], Money(0)) * 3;
    println!("{:?} {}", m, m > Money(30));
    let mv = V2 { x: Money(1), y: Money(2) } + V2 { x: Money(3), y: Money(4) };
    println!("{:?}", mv);
}
