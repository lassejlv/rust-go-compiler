use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

trait Num: Copy + PartialEq + fmt::Display + Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Div<Output = Self> + Neg<Output = Self> {
    fn zero() -> Self;
    fn one() -> Self;
    fn is_zero(&self) -> bool { *self == Self::zero() }
    fn abs_gt(&self, other: &Self) -> bool;
}

impl Num for f64 {
    fn zero() -> Self { 0.0 }
    fn one() -> Self { 1.0 }
    fn is_zero(&self) -> bool { self.abs() < 1e-12 }
    fn abs_gt(&self, o: &Self) -> bool { self.abs() > o.abs() }
}

fn gcd(a: i64, b: i64) -> i64 { if b == 0 { a.abs() } else { gcd(b, a % b) } }

#[derive(Clone, Copy, PartialEq, Debug)]
struct Q { n: i64, d: i64 }
impl Q {
    fn new(n: i64, d: i64) -> Q {
        assert!(d != 0, "zero denominator");
        let g = gcd(n, d).max(1);
        let s = if d < 0 { -1 } else { 1 };
        Q { n: s * n / g, d: s * d / g }
    }
}
impl fmt::Display for Q {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = if self.d == 1 { format!("{}", self.n) } else { format!("{}/{}", self.n, self.d) };
        f.pad(&s)
    }
}
impl Add for Q { type Output = Q; fn add(self, o: Q) -> Q { Q::new(self.n * o.d + o.n * self.d, self.d * o.d) } }
impl Sub for Q { type Output = Q; fn sub(self, o: Q) -> Q { self + (-o) } }
impl Mul for Q { type Output = Q; fn mul(self, o: Q) -> Q { Q::new(self.n * o.n, self.d * o.d) } }
impl Div for Q { type Output = Q; fn div(self, o: Q) -> Q { Q::new(self.n * o.d, self.d * o.n) } }
impl Neg for Q { type Output = Q; fn neg(self) -> Q { Q { n: -self.n, d: self.d } } }
impl Num for Q {
    fn zero() -> Self { Q::new(0, 1) }
    fn one() -> Self { Q::new(1, 1) }
    fn abs_gt(&self, o: &Self) -> bool { (self.n.abs() * o.d) > (o.n.abs() * self.d) }
}
impl From<i64> for Q { fn from(n: i64) -> Q { Q::new(n, 1) } }

#[derive(Clone)]
struct Mat<T> { r: usize, c: usize, a: Vec<T> }
impl<T: Num> Mat<T> {
    fn from_rows(rows: Vec<Vec<T>>) -> Self { let r = rows.len(); let c = rows[0].len(); Mat { r, c, a: rows.into_iter().flatten().collect() } }
    fn at(&self, i: usize, j: usize) -> T { self.a[i * self.c + j] }
    fn set(&mut self, i: usize, j: usize, v: T) { self.a[i * self.c + j] = v; }
    fn swap_rows(&mut self, i: usize, k: usize) { if i != k { for j in 0..self.c { self.a.swap(i * self.c + j, k * self.c + j); } } }
    fn identity(n: usize) -> Self { let mut m = Mat { r: n, c: n, a: vec![T::zero(); n * n] }; for i in 0..n { m.set(i, i, T::one()); } m }
    fn mul(&self, o: &Mat<T>) -> Mat<T> {
        let mut m = Mat { r: self.r, c: o.c, a: vec![T::zero(); self.r * o.c] };
        for i in 0..self.r { for j in 0..o.c { let mut s = T::zero(); for k in 0..self.c { s = s + self.at(i, k) * o.at(k, j); } m.set(i, j, s); } }
        m
    }
    /// returns (determinant, inverse) via Gauss-Jordan with partial pivoting
    fn det_inv(&self) -> (T, Option<Mat<T>>) {
        let n = self.r;
        let mut a = self.clone();
        let mut inv = Mat::identity(n);
        let mut det = T::one();
        for col in 0..n {
            let mut piv = col;
            for r in col + 1..n { if a.at(r, col).abs_gt(&a.at(piv, col)) { piv = r; } }
            if a.at(piv, col).is_zero() { return (T::zero(), None); }
            if piv != col { a.swap_rows(piv, col); inv.swap_rows(piv, col); det = -det; }
            let p = a.at(col, col);
            det = det * p;
            for j in 0..n { a.set(col, j, a.at(col, j) / p); inv.set(col, j, inv.at(col, j) / p); }
            for r in 0..n {
                if r == col { continue; }
                let f = a.at(r, col);
                if f.is_zero() { continue; }
                for j in 0..n {
                    a.set(r, j, a.at(r, j) - f * a.at(col, j));
                    inv.set(r, j, inv.at(r, j) - f * inv.at(col, j));
                }
            }
        }
        (det, Some(inv))
    }
}
impl<T: Num> fmt::Display for Mat<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for i in 0..self.r {
            write!(f, "[")?;
            for j in 0..self.c { if j > 0 { write!(f, " ")?; } write!(f, "{:>7}", self.at(i, j))?; }
            writeln!(f, "]")?;
        }
        Ok(())
    }
}

fn solve<T: Num>(m: &Mat<T>, b: &[T]) -> Option<Vec<T>> {
    let (_, inv) = m.det_inv();
    let inv = inv?;
    Some((0..m.r).map(|i| (0..m.c).fold(T::zero(), |s, j| s + inv.at(i, j) * b[j])).collect())
}

fn main() {
    let q = |n: i64| Q::from(n);
    let m = Mat::from_rows(vec![vec![q(2), q(1), q(-1)], vec![q(-3), q(-1), q(2)], vec![q(-2), q(1), q(2)]]);
    let (det, inv) = m.det_inv();
    println!("det = {}", det);
    let inv = inv.unwrap();
    print!("inverse:\n{}", inv);
    print!("check:\n{}", m.mul(&inv));
    let x = solve(&m, &[q(8), q(-11), q(-3)]).unwrap();
    println!("solution: {}", x.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "));
    let h = Mat::from_rows((1..=4).map(|i| (1..=4).map(|j| Q::new(1, i + j - 1)).collect()).collect());
    let (hd, hinv) = h.det_inv();
    println!("hilbert4 det = {}", hd);
    print!("{}", hinv.unwrap());
    let sing = Mat::from_rows(vec![vec![q(1), q(2)], vec![q(2), q(4)]]);
    println!("singular: {:?}", sing.det_inv().1.is_none());
    let mf = Mat::from_rows(vec![vec![4.0, 3.0], vec![6.0, 3.0]]);
    let (d, i) = mf.det_inv();
    println!("float det {:.3}", d);
    print!("{}", i.unwrap());
    let sol = solve(&Mat::from_rows(vec![vec![1.0, 1.0, 1.0], vec![0.0, 2.0, 5.0], vec![2.0, 5.0, -1.0]]), &[6.0, -4.0, 27.0]).unwrap();
    println!("{:?}", sol.iter().map(|v| format!("{:.4}", v)).collect::<Vec<_>>());
    println!("{} {} {} {}", Q::new(6, -8), Q::new(1, 3) + Q::new(1, 6), Q::new(3, 4) * Q::new(-8, 9) / Q::new(1, 2), Q::new(0, -5));
    println!("[{:>6}] [{:<6}] [{:^7}]", Q::new(1, 2), Q::new(-3, 1), Q::new(22, 7));
}
