use std::fmt;
use std::ops::{Index, IndexMut, Mul, Sub};

#[derive(Clone, PartialEq, Debug)]
struct Matrix { r: usize, c: usize, d: Vec<f64> }

impl Matrix {
    fn zeros(r: usize, c: usize) -> Self { Matrix { r, c, d: vec![0.0; r * c] } }
    fn identity(n: usize) -> Self { let mut m = Self::zeros(n, n); for i in 0..n { m[(i, i)] = 1.0; } m }
    fn from(rows: &[&[f64]]) -> Self { let r = rows.len(); let c = rows[0].len(); Matrix { r, c, d: rows.iter().flat_map(|row| row.iter().copied()).collect() } }
    fn transpose(&self) -> Self { let mut t = Self::zeros(self.c, self.r); for i in 0..self.r { for j in 0..self.c { t[(j, i)] = self[(i, j)]; } } t }
    fn lu(&self) -> Option<(Matrix, Matrix, Vec<usize>, i32)> {
        let n = self.r;
        let mut a = self.clone();
        let mut perm: Vec<usize> = (0..n).collect();
        let mut sign = 1;
        for k in 0..n {
            let p = (k..n).max_by(|&i, &j| a[(i, k)].abs().total_cmp(&a[(j, k)].abs()))?;
            if a[(p, k)].abs() < 1e-12 { return None; }
            if p != k { for j in 0..n { let t = a[(k, j)]; a[(k, j)] = a[(p, j)]; a[(p, j)] = t; } perm.swap(k, p); sign = -sign; }
            for i in k + 1..n {
                a[(i, k)] /= a[(k, k)];
                for j in k + 1..n { let f = a[(i, k)] * a[(k, j)]; a[(i, j)] -= f; }
            }
        }
        let mut l = Matrix::identity(n); let mut u = Matrix::zeros(n, n);
        for i in 0..n { for j in 0..n { if j < i { l[(i, j)] = a[(i, j)]; } else { u[(i, j)] = a[(i, j)]; } } }
        Some((l, u, perm, sign))
    }
    fn det(&self) -> f64 { self.lu().map_or(0.0, |(_, u, _, s)| s as f64 * (0..self.r).map(|i| u[(i, i)]).product::<f64>()) }
    fn permute_rows(&self, p: &[usize]) -> Matrix { let mut m = Self::zeros(self.r, self.c); for (i, &pi) in p.iter().enumerate() { for j in 0..self.c { m[(i, j)] = self[(pi, j)]; } } m }
    fn max_abs(&self) -> f64 { self.d.iter().fold(0.0, |a, &b| a.max(b.abs())) }
}
impl Index<(usize, usize)> for Matrix { type Output = f64; fn index(&self, (i, j): (usize, usize)) -> &f64 { &self.d[i * self.c + j] } }
impl IndexMut<(usize, usize)> for Matrix { fn index_mut(&mut self, (i, j): (usize, usize)) -> &mut f64 { &mut self.d[i * self.c + j] } }
impl<'a> Mul<&'a Matrix> for &'a Matrix {
    type Output = Matrix;
    fn mul(self, o: &Matrix) -> Matrix {
        assert_eq!(self.c, o.r, "dimension mismatch");
        let mut m = Matrix::zeros(self.r, o.c);
        for i in 0..self.r { for k in 0..self.c { let a = self[(i, k)]; for j in 0..o.c { m[(i, j)] += a * o[(k, j)]; } } }
        m
    }
}
impl<'a> Sub for &'a Matrix { type Output = Matrix; fn sub(self, o: &Matrix) -> Matrix { Matrix { r: self.r, c: self.c, d: self.d.iter().zip(&o.d).map(|(a, b)| a - b).collect() } } }
impl fmt::Display for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let prec = f.precision().unwrap_or(2);
        let width = f.width().unwrap_or(prec + 5);
        let plus = f.sign_plus();
        for i in 0..self.r {
            write!(f, "{}", if i == 0 { "⎡" } else if i + 1 == self.r { "⎣" } else { "⎢" })?;
            for j in 0..self.c {
                let v = self[(i, j)];
                let v = if v == 0.0 { 0.0 } else { v };
                if plus { write!(f, "{:>+w$.p$}", v, w = width, p = prec)?; } else { write!(f, "{:>w$.p$}", v, w = width, p = prec)?; }
            }
            writeln!(f, "{}", if i == 0 { "⎤" } else if i + 1 == self.r { "⎦" } else { "⎥" })?;
        }
        Ok(())
    }
}

fn main() {
    let a = Matrix::from(&[&[2.0, -1.0, 0.0, 3.0], &[4.0, 1.0, -2.0, 0.5], &[-6.0, 3.0, 1.0, 1.0], &[1.0, 1.0, 1.0, 1.0]]);
    print!("{}", a);
    print!("{:.4}", a.transpose());
    print!("{:+9.1}", &a * &a.transpose());
    let (l, u, p, s) = a.lu().unwrap();
    print!("L=\n{:.3}U=\n{:.3}", l, u);
    println!("perm {:?} sign {} det {:.6}", p, s, a.det());
    let recon = &l * &u;
    let err = (&recon - &a.permute_rows(&p)).max_abs();
    println!("reconstruction error < 1e-12: {}", err < 1e-12);
    let sing = Matrix::from(&[&[1.0, 2.0], &[2.0, 4.0]]);
    println!("singular lu {:?} det {}", sing.lu().map(|x| x.3), sing.det());
    let hilbert = Matrix { r: 5, c: 5, d: (0..25).map(|k| 1.0 / ((k / 5 + k % 5 + 1) as f64)).collect() };
    println!("hilbert5 det {:e}", hilbert.det());
    print!("{:10.6}", hilbert);
    let r = std::panic::catch_unwind(|| { let x = Matrix::zeros(2, 3); let y = Matrix::zeros(2, 3); let _ = &x * &y; });
    println!("mismatch panics: {}", r.is_err());
}
