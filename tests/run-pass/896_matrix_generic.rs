use std::ops::{Add, Mul, Index, IndexMut};
use std::fmt;

trait Zero { fn zero() -> Self; }
trait One { fn one() -> Self; }
impl Zero for i64 { fn zero() -> Self { 0 } }
impl One for i64 { fn one() -> Self { 1 } }
impl Zero for f64 { fn zero() -> Self { 0.0 } }
impl One for f64 { fn one() -> Self { 1.0 } }

#[derive(Clone, PartialEq, Debug)]
struct Matrix<T> { r: usize, c: usize, d: Vec<T> }

impl<T: Copy + Zero> Matrix<T> {
    fn zeros(r: usize, c: usize) -> Self { Matrix { r, c, d: vec![T::zero(); r * c] } }
    fn from_rows(rows: &[&[T]]) -> Self {
        let r = rows.len();
        let c = rows[0].len();
        let d = rows.iter().flat_map(|row| row.iter().copied()).collect();
        Matrix { r, c, d }
    }
    fn transpose(&self) -> Self {
        let mut m = Matrix::zeros(self.c, self.r);
        for i in 0..self.r { for j in 0..self.c { m[(j, i)] = self[(i, j)]; } }
        m
    }
    fn map<U: Copy + Zero>(&self, f: impl Fn(T) -> U) -> Matrix<U> {
        Matrix { r: self.r, c: self.c, d: self.d.iter().map(|&x| f(x)).collect() }
    }
}
impl<T: Copy + Zero + One> Matrix<T> {
    fn identity(n: usize) -> Self {
        let mut m = Self::zeros(n, n);
        for i in 0..n { m[(i, i)] = T::one(); }
        m
    }
}
impl<T> Index<(usize, usize)> for Matrix<T> {
    type Output = T;
    fn index(&self, (i, j): (usize, usize)) -> &T { &self.d[i * self.c + j] }
}
impl<T> IndexMut<(usize, usize)> for Matrix<T> {
    fn index_mut(&mut self, (i, j): (usize, usize)) -> &mut T { &mut self.d[i * self.c + j] }
}
impl<'a, T: Copy + Zero + Add<Output = T> + Mul<Output = T>> Mul for &'a Matrix<T> {
    type Output = Matrix<T>;
    fn mul(self, o: &'a Matrix<T>) -> Matrix<T> {
        assert_eq!(self.c, o.r);
        let mut m = Matrix::zeros(self.r, o.c);
        for i in 0..self.r { for j in 0..o.c {
            let mut s = T::zero();
            for k in 0..self.c { s = s + self[(i, k)] * o[(k, j)]; }
            m[(i, j)] = s;
        } }
        m
    }
}
impl<T: Copy + Add<Output = T>> Add for Matrix<T> {
    type Output = Matrix<T>;
    fn add(self, o: Self) -> Self { Matrix { r: self.r, c: self.c, d: self.d.iter().zip(&o.d).map(|(&a, &b)| a + b).collect() } }
}
impl<T: fmt::Display> fmt::Display for Matrix<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for i in 0..self.r {
            let row: Vec<String> = (0..self.c).map(|j| format!("{:>6}", self.d[i * self.c + j].to_string())).collect();
            writeln!(f, "[{}]", row.join(""))?;
        }
        Ok(())
    }
}

fn mat_pow<T: Copy + Zero + One + Add<Output = T> + Mul<Output = T>>(m: &Matrix<T>, mut e: u32) -> Matrix<T> {
    let mut result = Matrix::identity(m.r);
    let mut base = m.clone();
    while e > 0 {
        if e & 1 == 1 { result = &result * &base; }
        base = &base * &base;
        e >>= 1;
    }
    result
}

fn main() {
    let a = Matrix::from_rows(&[&[1i64, 2, 3], &[4, 5, 6]]);
    let b = a.transpose();
    print!("{}", a);
    print!("{}", b);
    print!("{}", &a * &b);
    print!("{}", &b * &a);
    let fib = Matrix::from_rows(&[&[1i64, 1], &[1, 0]]);
    println!("fib50 = {}", mat_pow(&fib, 50)[(0, 1)]);
    let rot = Matrix::from_rows(&[&[0.0, -1.0], &[1.0, 0.0]]);
    print!("{}", mat_pow(&rot, 2));
    print!("{}", mat_pow(&rot, 4) + Matrix::identity(2));
    let half = a.map(|x| x as f64 / 2.0);
    print!("{}", half);
    println!("{}", Matrix::<i64>::identity(3) == mat_pow(&Matrix::identity(3), 7));
    println!("{:?}", a.map(|x| x * x).d);
}
