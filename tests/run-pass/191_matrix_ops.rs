use std::fmt;
use std::ops::{Add, Index, IndexMut, Mul, Neg, Sub};

#[derive(Debug, Clone, PartialEq)]
struct Matrix {
    rows: usize,
    cols: usize,
    data: Vec<f64>,
}

impl Matrix {
    fn zeros(rows: usize, cols: usize) -> Self {
        Matrix { rows, cols, data: vec![0.0; rows * cols] }
    }
    fn identity(n: usize) -> Self {
        let mut m = Matrix::zeros(n, n);
        for i in 0..n {
            m[(i, i)] = 1.0;
        }
        m
    }
    fn from_rows(rows: &[&[f64]]) -> Self {
        let r = rows.len();
        let c = rows[0].len();
        Matrix { rows: r, cols: c, data: rows.iter().flat_map(|row| row.iter().copied()).collect() }
    }
    fn transpose(&self) -> Matrix {
        let mut t = Matrix::zeros(self.cols, self.rows);
        for i in 0..self.rows {
            for j in 0..self.cols {
                t[(j, i)] = self[(i, j)];
            }
        }
        t
    }
    fn determinant(&self) -> f64 {
        assert_eq!(self.rows, self.cols);
        let n = self.rows;
        let mut m = self.clone();
        let mut det = 1.0;
        for col in 0..n {
            let pivot = (col..n).max_by(|&a, &b| m[(a, col)].abs().total_cmp(&m[(b, col)].abs())).unwrap();
            if m[(pivot, col)].abs() < 1e-12 {
                return 0.0;
            }
            if pivot != col {
                for k in 0..n {
                    let tmp = m[(col, k)];
                    m[(col, k)] = m[(pivot, k)];
                    m[(pivot, k)] = tmp;
                }
                det = -det;
            }
            det *= m[(col, col)];
            for r in col + 1..n {
                let factor = m[(r, col)] / m[(col, col)];
                for k in col..n {
                    let v = m[(col, k)];
                    m[(r, k)] -= factor * v;
                }
            }
        }
        det
    }
    fn pow(&self, mut e: u32) -> Matrix {
        let mut base = self.clone();
        let mut acc = Matrix::identity(self.rows);
        while e > 0 {
            if e & 1 == 1 {
                acc = &acc * &base;
            }
            base = &base * &base;
            e >>= 1;
        }
        acc
    }
}

impl Index<(usize, usize)> for Matrix {
    type Output = f64;
    fn index(&self, (r, c): (usize, usize)) -> &f64 {
        &self.data[r * self.cols + c]
    }
}
impl IndexMut<(usize, usize)> for Matrix {
    fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut f64 {
        &mut self.data[r * self.cols + c]
    }
}

impl<'a> Add for &'a Matrix {
    type Output = Matrix;
    fn add(self, o: &Matrix) -> Matrix {
        assert_eq!((self.rows, self.cols), (o.rows, o.cols));
        Matrix { rows: self.rows, cols: self.cols, data: self.data.iter().zip(&o.data).map(|(a, b)| a + b).collect() }
    }
}
impl<'a> Sub for &'a Matrix {
    type Output = Matrix;
    fn sub(self, o: &Matrix) -> Matrix {
        self + &(-o)
    }
}
impl<'a> Neg for &'a Matrix {
    type Output = Matrix;
    fn neg(self) -> Matrix {
        Matrix { rows: self.rows, cols: self.cols, data: self.data.iter().map(|x| -x).collect() }
    }
}
impl<'a> Mul for &'a Matrix {
    type Output = Matrix;
    fn mul(self, o: &Matrix) -> Matrix {
        assert_eq!(self.cols, o.rows);
        let mut out = Matrix::zeros(self.rows, o.cols);
        for i in 0..self.rows {
            for j in 0..o.cols {
                out[(i, j)] = (0..self.cols).map(|k| self[(i, k)] * o[(k, j)]).sum();
            }
        }
        out
    }
}
impl Mul<f64> for Matrix {
    type Output = Matrix;
    fn mul(mut self, k: f64) -> Matrix {
        self.data.iter_mut().for_each(|x| *x *= k);
        self
    }
}

impl fmt::Display for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let prec = f.precision().unwrap_or(1);
        for r in 0..self.rows {
            write!(f, "[")?;
            for c in 0..self.cols {
                if c > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{:>8.*}", prec, self[(r, c)])?;
            }
            writeln!(f, "]")?;
        }
        Ok(())
    }
}

fn main() {
    let a = Matrix::from_rows(&[&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]]);
    let b = Matrix::from_rows(&[&[7.0, 8.0], &[9.0, 10.0], &[11.0, 12.0]]);
    println!("A =\n{}", a);
    println!("A^T =\n{}", a.transpose());
    let ab = &a * &b;
    println!("A*B =\n{}", ab);
    println!("B*A =\n{:.0}", &b * &a);
    let i2 = Matrix::identity(2);
    println!("AB + I =\n{}", &ab + &i2);
    println!("AB - AB == 0: {}", (&ab - &ab).data.iter().all(|&x| x == 0.0));
    println!("2.5 * I =\n{:.3}", i2.clone() * 2.5);
    let fib = Matrix::from_rows(&[&[1.0, 1.0], &[1.0, 0.0]]);
    for n in [1, 5, 10, 30] {
        println!("fib^{} -> F = {}", n, fib.pow(n)[(0, 1)]);
    }
    let m3 = Matrix::from_rows(&[&[2.0, -3.0, 1.0], &[2.0, 0.0, -1.0], &[1.0, 4.0, 5.0]]);
    println!("det(m3) = {:.3}", m3.determinant());
    println!("det(AB) = {:.3}", ab.determinant());
    let singular = Matrix::from_rows(&[&[1.0, 2.0], &[2.0, 4.0]]);
    println!("det(singular) = {}", singular.determinant());
    let rot = Matrix::from_rows(&[&[0.0, -1.0], &[1.0, 0.0]]);
    println!("rot^4 == I: {}", rot.pow(4) == Matrix::identity(2));
    println!("{:?}", Matrix::identity(2));
}
