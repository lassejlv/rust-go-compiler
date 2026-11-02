// Generic dense matrix over a numeric trait: Mul/Add impls on references, Index<(usize, usize)>,
// transpose, identity, power by squaring, Gaussian elimination with f64 partial pivoting for
// determinant/inverse, rational (fraction) matrices for exact results, and Display formatting.
use std::fmt;
use std::ops::{Add, Div, Index, IndexMut, Mul, Neg, Sub};

trait Field: Copy + PartialEq + fmt::Display + Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Div<Output = Self> + Neg<Output = Self> {
    fn zero() -> Self;
    fn one() -> Self;
    fn is_zero(&self) -> bool {
        *self == Self::zero()
    }
}
impl Field for f64 {
    fn zero() -> Self { 0.0 }
    fn one() -> Self { 1.0 }
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct Frac(i64, i64);
fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a.abs() } else { gcd(b, a % b) }
}
impl Frac {
    fn new(n: i64, d: i64) -> Frac {
        let g = gcd(n, d).max(1);
        let s = if d < 0 { -1 } else { 1 };
        Frac(s * n / g, s * d / g)
    }
}
impl Add for Frac { type Output = Frac; fn add(self, o: Frac) -> Frac { Frac::new(self.0 * o.1 + o.0 * self.1, self.1 * o.1) } }
impl Sub for Frac { type Output = Frac; fn sub(self, o: Frac) -> Frac { self + (-o) } }
impl Mul for Frac { type Output = Frac; fn mul(self, o: Frac) -> Frac { Frac::new(self.0 * o.0, self.1 * o.1) } }
impl Div for Frac { type Output = Frac; fn div(self, o: Frac) -> Frac { Frac::new(self.0 * o.1, self.1 * o.0) } }
impl Neg for Frac { type Output = Frac; fn neg(self) -> Frac { Frac(-self.0, self.1) } }
impl fmt::Display for Frac {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = if self.1 == 1 { format!("{}", self.0) } else { format!("{}/{}", self.0, self.1) };
        f.pad(&s)
    }
}
impl Field for Frac {
    fn zero() -> Self { Frac(0, 1) }
    fn one() -> Self { Frac(1, 1) }
}

#[derive(Clone, PartialEq, Debug)]
struct Mat<T> {
    r: usize,
    c: usize,
    d: Vec<T>,
}

impl<T: Field> Mat<T> {
    fn from(rows: Vec<Vec<T>>) -> Self {
        let r = rows.len();
        let c = rows[0].len();
        Mat { r, c, d: rows.into_iter().flatten().collect() }
    }
    fn identity(n: usize) -> Self {
        let mut m = Mat { r: n, c: n, d: vec![T::zero(); n * n] };
        for i in 0..n { m[(i, i)] = T::one(); }
        m
    }
    fn t(&self) -> Self {
        let mut m = Mat { r: self.c, c: self.r, d: vec![T::zero(); self.d.len()] };
        for i in 0..self.r { for j in 0..self.c { m[(j, i)] = self[(i, j)]; } }
        m
    }
    fn pow(&self, mut e: u32) -> Self {
        let (mut base, mut acc) = (self.clone(), Mat::identity(self.r));
        while e > 0 {
            if e & 1 == 1 { acc = &acc * &base; }
            base = &base * &base;
            e >>= 1;
        }
        acc
    }
    // returns (determinant, inverse if exists)
    fn det_inv(&self) -> (T, Option<Self>) {
        let n = self.r;
        let mut a = self.clone();
        let mut inv = Mat::identity(n);
        let mut det = T::one();
        for col in 0..n {
            let Some(p) = (col..n).find(|&r| !a[(r, col)].is_zero()) else { return (T::zero(), None) };
            if p != col {
                for j in 0..n {
                    a.d.swap(p * n + j, col * n + j);
                    inv.d.swap(p * n + j, col * n + j);
                }
                det = -det;
            }
            let pv = a[(col, col)];
            det = det * pv;
            for j in 0..n {
                a[(col, j)] = a[(col, j)] / pv;
                inv[(col, j)] = inv[(col, j)] / pv;
            }
            for r in (0..n).filter(|&r| r != col) {
                let f = a[(r, col)];
                for j in 0..n {
                    let (av, iv) = (a[(col, j)], inv[(col, j)]);
                    a[(r, j)] = a[(r, j)] - f * av;
                    inv[(r, j)] = inv[(r, j)] - f * iv;
                }
            }
        }
        (det, Some(inv))
    }
}

impl<T> Index<(usize, usize)> for Mat<T> {
    type Output = T;
    fn index(&self, (i, j): (usize, usize)) -> &T { &self.d[i * self.c + j] }
}
impl<T> IndexMut<(usize, usize)> for Mat<T> {
    fn index_mut(&mut self, (i, j): (usize, usize)) -> &mut T { &mut self.d[i * self.c + j] }
}
impl<'a, T: Field> Mul<&'a Mat<T>> for &'a Mat<T> {
    type Output = Mat<T>;
    fn mul(self, o: &Mat<T>) -> Mat<T> {
        assert_eq!(self.c, o.r);
        let mut m = Mat { r: self.r, c: o.c, d: vec![T::zero(); self.r * o.c] };
        for i in 0..self.r { for j in 0..o.c { for k in 0..self.c { m[(i, j)] = m[(i, j)] + self[(i, k)] * o[(k, j)]; } } }
        m
    }
}
impl<'a, T: Field> Add<&'a Mat<T>> for &'a Mat<T> {
    type Output = Mat<T>;
    fn add(self, o: &Mat<T>) -> Mat<T> {
        Mat { r: self.r, c: self.c, d: self.d.iter().zip(&o.d).map(|(a, b)| *a + *b).collect() }
    }
}
impl<T: Field> fmt::Display for Mat<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let w = f.width().unwrap_or(6);
        for i in 0..self.r {
            let row: Vec<String> = (0..self.c).map(|j| format!("{:>w$}", self[(i, j)], w = w)).collect();
            writeln!(f, "[{}]", row.join(" "))?;
        }
        Ok(())
    }
}

fn fr(v: &[&[i64]]) -> Mat<Frac> {
    Mat::from(v.iter().map(|r| r.iter().map(|&x| Frac(x, 1)).collect()).collect())
}

fn main() {
    let fib = Mat::from(vec![vec![1.0, 1.0], vec![1.0, 0.0]]);
    print!("{:8}", fib.pow(30));
    let a = fr(&[&[2, 1, 1], &[1, 3, 2], &[1, 0, 0]]);
    let (det, inv) = a.det_inv();
    println!("det = {}", det);
    let inv = inv.unwrap();
    print!("{:6}", inv);
    print!("{:4}", &a * &inv);
    println!("{}", &a * &inv == Mat::identity(3));
    let sing = fr(&[&[1, 2], &[2, 4]]);
    println!("singular: {:?}", sing.det_inv().0);
    let r = fr(&[&[1, 2, 3], &[4, 5, 6]]);
    print!("{:4}", &r * &r.t());
    print!("{:4}", &r.t() + &r.t());
    let h = Mat::from((0..3).map(|i| (0..3).map(|j| Frac::new(1, i + j + 1)).collect()).collect());
    let (hd, hi) = h.det_inv();
    println!("hilbert det = {}", hd);
    print!("{:6}", hi.unwrap());
    let m = Mat::from(vec![vec![4.0, 7.0], vec![2.0, 6.0]]);
    let (d, mi) = m.det_inv();
    println!("{:.3}", d);
    print!("{:8.3}", mi.unwrap().d.iter().map(|x| format!("{:.3}", x)).collect::<Vec<_>>().join(" "));
    println!();
}
