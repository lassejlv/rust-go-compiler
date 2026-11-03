// generic matrix with operator overloading on values and references, Index/IndexMut, Display forwarding precision
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Index, IndexMut, Mul, Neg, Sub};

#[derive(Debug, Clone, PartialEq)]
struct Matrix<T> {
    r: usize,
    c: usize,
    d: Vec<T>,
}

trait Zero: Copy {
    fn zero() -> Self;
    fn one() -> Self;
}
impl Zero for i64 {
    fn zero() -> Self { 0 }
    fn one() -> Self { 1 }
}
impl Zero for f64 {
    fn zero() -> Self { 0.0 }
    fn one() -> Self { 1.0 }
}

impl<T: Zero> Matrix<T> {
    fn new(r: usize, c: usize) -> Self {
        Matrix { r, c, d: vec![T::zero(); r * c] }
    }
    fn identity(n: usize) -> Self {
        let mut m = Self::new(n, n);
        for i in 0..n {
            m[(i, i)] = T::one();
        }
        m
    }
    fn from_rows(rows: &[&[T]]) -> Self {
        let r = rows.len();
        let c = rows[0].len();
        Matrix { r, c, d: rows.iter().flat_map(|row| row.iter().copied()).collect() }
    }
    fn transpose(&self) -> Self {
        let mut t = Self::new(self.c, self.r);
        for i in 0..self.r {
            for j in 0..self.c {
                t[(j, i)] = self[(i, j)];
            }
        }
        t
    }
}

impl<T> Index<(usize, usize)> for Matrix<T> {
    type Output = T;
    fn index(&self, (i, j): (usize, usize)) -> &T {
        &self.d[i * self.c + j]
    }
}
impl<T> IndexMut<(usize, usize)> for Matrix<T> {
    fn index_mut(&mut self, (i, j): (usize, usize)) -> &mut T {
        &mut self.d[i * self.c + j]
    }
}
impl<T> Index<usize> for Matrix<T> {
    type Output = [T];
    fn index(&self, row: usize) -> &[T] {
        &self.d[row * self.c..(row + 1) * self.c]
    }
}

impl<'a, T: Zero + Add<Output = T> + Mul<Output = T>> Mul for &'a Matrix<T> {
    type Output = Matrix<T>;
    fn mul(self, o: &'a Matrix<T>) -> Matrix<T> {
        assert_eq!(self.c, o.r);
        let mut m = Matrix::new(self.r, o.c);
        for i in 0..self.r {
            for j in 0..o.c {
                let mut s = T::zero();
                for k in 0..self.c {
                    s = s + self[(i, k)] * o[(k, j)];
                }
                m[(i, j)] = s;
            }
        }
        m
    }
}
impl<T: Zero + Add<Output = T> + Mul<Output = T>> Mul for Matrix<T> {
    type Output = Matrix<T>;
    fn mul(self, o: Matrix<T>) -> Matrix<T> {
        &self * &o
    }
}
impl<T: Zero + Mul<Output = T>> Mul<T> for &Matrix<T> {
    type Output = Matrix<T>;
    fn mul(self, k: T) -> Matrix<T> {
        Matrix { r: self.r, c: self.c, d: self.d.iter().map(|&x| x * k).collect() }
    }
}
impl Mul<&Matrix<f64>> for f64 {
    type Output = Matrix<f64>;
    fn mul(self, m: &Matrix<f64>) -> Matrix<f64> {
        m * self
    }
}
impl<T: Copy + Add<Output = T>> Add for &Matrix<T> {
    type Output = Matrix<T>;
    fn add(self, o: Self) -> Matrix<T> {
        Matrix { r: self.r, c: self.c, d: self.d.iter().zip(&o.d).map(|(&a, &b)| a + b).collect() }
    }
}
impl<T: Copy + Sub<Output = T>> Sub for Matrix<T> {
    type Output = Matrix<T>;
    fn sub(self, o: Self) -> Matrix<T> {
        Matrix { r: self.r, c: self.c, d: self.d.into_iter().zip(o.d).map(|(a, b)| a - b).collect() }
    }
}
impl<T: Copy + AddAssign> AddAssign<&Matrix<T>> for Matrix<T> {
    fn add_assign(&mut self, o: &Matrix<T>) {
        for (a, &b) in self.d.iter_mut().zip(&o.d) {
            *a += b;
        }
    }
}
impl<T: Copy + Neg<Output = T>> Neg for Matrix<T> {
    type Output = Matrix<T>;
    fn neg(self) -> Matrix<T> {
        Matrix { r: self.r, c: self.c, d: self.d.into_iter().map(|x| -x).collect() }
    }
}
impl<'a, T: Zero + Copy + Add<Output = T> + 'a> Sum<&'a Matrix<T>> for Matrix<T> {
    fn sum<I: Iterator<Item = &'a Matrix<T>>>(mut it: I) -> Matrix<T> {
        let first = it.next().unwrap().clone();
        it.fold(first, |acc, m| &acc + m)
    }
}
impl<T: fmt::Display> fmt::Display for Matrix<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for i in 0..self.r {
            write!(f, "[")?;
            for j in 0..self.c {
                if j > 0 {
                    write!(f, " ")?;
                }
                let v = &self.d[i * self.c + j];
                match (f.width(), f.precision()) {
                    (Some(w), Some(p)) => write!(f, "{:>w$.p$}", v, w = w, p = p)?,
                    (Some(w), None) => write!(f, "{:>w$}", v, w = w)?,
                    (None, Some(p)) => write!(f, "{:.p$}", v, p = p)?,
                    (None, None) => write!(f, "{}", v)?,
                }
            }
            writeln!(f, "]")?;
        }
        Ok(())
    }
}

fn mat_pow(m: &Matrix<i64>, mut e: u32) -> Matrix<i64> {
    let mut base = m.clone();
    let mut acc = Matrix::identity(m.r);
    while e > 0 {
        if e & 1 == 1 {
            acc = &acc * &base;
        }
        e >>= 1;
        if e > 0 {
            base = &base * &base;
        }
    }
    acc
}

fn main() {
    let a = Matrix::from_rows(&[&[1i64, 2, 3], &[4, 5, 6]]);
    let b = a.transpose();
    print!("{}", &a * &b);
    print!("{:4}", &b * &a);
    println!("{:?}", &a[1]);
    let fib = Matrix::from_rows(&[&[1i64, 1], &[1, 0]]);
    println!("fib(90) = {}", mat_pow(&fib, 90)[(0, 1)]);
    let f = Matrix::from_rows(&[&[1.0, 2.5], &[-0.125, 4.0]]);
    print!("{:.2}", 0.5 * &f);
    print!("{:8.3}", &f * &f);
    let mut acc = Matrix::<f64>::new(2, 2);
    acc += &f;
    acc += &Matrix::identity(2);
    print!("{}", -(acc.clone() - f.clone()));
    let total: Matrix<f64> = [f.clone(), acc.clone(), Matrix::identity(2)].iter().sum();
    print!("{:6.1}", total);
    println!("{}", (f.clone() * Matrix::identity(2)) == f);
    let id3: Matrix<i64> = Matrix::identity(3);
    println!("{:?}", id3.d);
}
