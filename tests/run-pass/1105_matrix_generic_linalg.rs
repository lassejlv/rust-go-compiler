// Generic matrix / linear algebra with operator overloading, const generics,
// a numeric trait hierarchy, rationals, and Gaussian elimination.
use std::fmt;
use std::ops::{Add, AddAssign, Div, Index, IndexMut, Mul, Neg, Sub};

trait Zero { fn zero() -> Self; fn is_zero(&self) -> bool; }
trait One { fn one() -> Self; }
trait Field:
    Copy + Zero + One + PartialEq + PartialOrd + fmt::Display
    + Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Div<Output = Self> + Neg<Output = Self>
{
    fn abs_val(self) -> Self { if self < Self::zero() { -self } else { self } }
}

impl Zero for f64 { fn zero() -> Self { 0.0 } fn is_zero(&self) -> bool { self.abs() < 1e-12 } }
impl One for f64 { fn one() -> Self { 1.0 } }
impl Field for f64 {}
impl Zero for i64 { fn zero() -> Self { 0 } fn is_zero(&self) -> bool { *self == 0 } }
impl One for i64 { fn one() -> Self { 1 } }

fn gcd(a: i64, b: i64) -> i64 { if b == 0 { a.abs() } else { gcd(b, a % b) } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
impl PartialOrd for Q {
    fn partial_cmp(&self, o: &Q) -> Option<std::cmp::Ordering> { (self.n * o.d).partial_cmp(&(o.n * self.d)) }
}
impl Add for Q { type Output = Q; fn add(self, o: Q) -> Q { Q::new(self.n * o.d + o.n * self.d, self.d * o.d) } }
impl Sub for Q { type Output = Q; fn sub(self, o: Q) -> Q { Q::new(self.n * o.d - o.n * self.d, self.d * o.d) } }
impl Mul for Q { type Output = Q; fn mul(self, o: Q) -> Q { Q::new(self.n * o.n, self.d * o.d) } }
impl Div for Q { type Output = Q; fn div(self, o: Q) -> Q { Q::new(self.n * o.d, self.d * o.n) } }
impl Neg for Q { type Output = Q; fn neg(self) -> Q { Q { n: -self.n, d: self.d } } }
impl Zero for Q { fn zero() -> Q { Q { n: 0, d: 1 } } fn is_zero(&self) -> bool { self.n == 0 } }
impl One for Q { fn one() -> Q { Q { n: 1, d: 1 } } }
impl Field for Q {}
impl From<i64> for Q { fn from(n: i64) -> Q { Q::new(n, 1) } }

#[derive(Clone, PartialEq, Debug)]
struct Matrix<T> { rows: usize, cols: usize, data: Vec<T> }

impl<T: Copy + Zero + One> Matrix<T> {
    fn zeros(rows: usize, cols: usize) -> Self { Matrix { rows, cols, data: vec![T::zero(); rows * cols] } }
    fn identity(n: usize) -> Self {
        let mut m = Self::zeros(n, n);
        for i in 0..n { m[(i, i)] = T::one(); }
        m
    }
    fn from_rows<const C: usize>(rows: &[[T; C]]) -> Self {
        let data = rows.iter().flat_map(|r| r.iter().copied()).collect();
        Matrix { rows: rows.len(), cols: C, data }
    }
    fn transpose(&self) -> Self {
        let mut t = Self::zeros(self.cols, self.rows);
        for r in 0..self.rows { for c in 0..self.cols { t[(c, r)] = self[(r, c)]; } }
        t
    }
    fn map<U: Copy + Zero + One>(&self, f: impl Fn(T) -> U) -> Matrix<U> {
        Matrix { rows: self.rows, cols: self.cols, data: self.data.iter().map(|&x| f(x)).collect() }
    }
    fn row(&self, r: usize) -> &[T] { &self.data[r * self.cols..(r + 1) * self.cols] }
}

impl<T> Index<(usize, usize)> for Matrix<T> {
    type Output = T;
    fn index(&self, (r, c): (usize, usize)) -> &T {
        assert!(r < self.rows && c < self.cols, "index ({}, {}) out of bounds", r, c);
        &self.data[r * self.cols + c]
    }
}
impl<T> IndexMut<(usize, usize)> for Matrix<T> {
    fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut T { &mut self.data[r * self.cols + c] }
}

impl<'a, T: Copy + Zero + One + Add<Output = T> + Mul<Output = T>> Mul for &'a Matrix<T> {
    type Output = Matrix<T>;
    fn mul(self, o: &'a Matrix<T>) -> Matrix<T> {
        assert_eq!(self.cols, o.rows, "dimension mismatch");
        let mut m = Matrix::zeros(self.rows, o.cols);
        for i in 0..self.rows {
            for k in 0..self.cols {
                let a = self[(i, k)];
                for j in 0..o.cols {
                    m[(i, j)] = m[(i, j)] + a * o[(k, j)];
                }
            }
        }
        m
    }
}
impl<T: Copy + Add<Output = T>> Add for Matrix<T> {
    type Output = Matrix<T>;
    fn add(mut self, o: Matrix<T>) -> Matrix<T> {
        for (a, b) in self.data.iter_mut().zip(o.data) { *a = *a + b; }
        self
    }
}
impl<T: Copy + Add<Output = T>> AddAssign<&Matrix<T>> for Matrix<T> {
    fn add_assign(&mut self, o: &Matrix<T>) {
        for (a, b) in self.data.iter_mut().zip(&o.data) { *a = *a + *b; }
    }
}
impl<T: Copy + Mul<Output = T>> Mul<T> for Matrix<T> {
    type Output = Matrix<T>;
    fn mul(self, k: T) -> Matrix<T> { Matrix { data: self.data.into_iter().map(|x| x * k).collect(), ..self } }
}

impl<T: fmt::Display> fmt::Display for Matrix<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let cells: Vec<String> = self.data.iter().map(|x| match f.precision() {
            Some(p) => format!("{:.*}", p, x),
            None => format!("{}", x),
        }).collect();
        let w = cells.iter().map(|s| s.chars().count()).max().unwrap_or(1);
        for r in 0..self.rows {
            write!(f, "[")?;
            for c in 0..self.cols {
                if c > 0 { write!(f, " ")?; }
                write!(f, "{:>w$}", cells[r * self.cols + c], w = w)?;
            }
            writeln!(f, "]")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
enum SolveError { Singular { col: usize }, Shape(usize, usize) }

impl<T: Field> Matrix<T> {
    /// Returns (rank, determinant, reduced row echelon form)
    fn rref(&self) -> (usize, T, Matrix<T>) {
        let mut m = self.clone();
        let mut det = T::one();
        let mut rank = 0;
        for c in 0..m.cols {
            if rank == m.rows { break; }
            // partial pivot: largest absolute value
            let mut best = rank;
            for r in rank + 1..m.rows {
                if m[(r, c)].abs_val() > m[(best, c)].abs_val() { best = r; }
            }
            if m[(best, c)].is_zero() { det = T::zero(); continue; }
            if best != rank {
                for k in 0..m.cols { let t = m[(best, k)]; m[(best, k)] = m[(rank, k)]; m[(rank, k)] = t; }
                det = -det;
            }
            let p = m[(rank, c)];
            det = det * p;
            for k in 0..m.cols { m[(rank, k)] = m[(rank, k)] / p; }
            for r in 0..m.rows {
                if r != rank && !m[(r, c)].is_zero() {
                    let f = m[(r, c)];
                    for k in 0..m.cols { m[(r, k)] = m[(r, k)] - f * m[(rank, k)]; }
                }
            }
            rank += 1;
        }
        if rank < m.rows.min(m.cols) || m.rows != m.cols { det = T::zero(); }
        (rank, det, m)
    }

    fn solve(&self, b: &[T]) -> Result<Vec<T>, SolveError> {
        if self.rows != self.cols || b.len() != self.rows { return Err(SolveError::Shape(self.rows, self.cols)); }
        let n = self.rows;
        let mut aug = Matrix::zeros(n, n + 1);
        for r in 0..n { for c in 0..n { aug[(r, c)] = self[(r, c)]; } aug[(r, n)] = b[r]; }
        let (rank, _, red) = aug.rref();
        for c in 0..n {
            if red[(c.min(n - 1), c)].is_zero() || rank < n { return Err(SolveError::Singular { col: c }); }
        }
        Ok((0..n).map(|r| red[(r, n)]).collect())
    }

    fn inverse(&self) -> Option<Matrix<T>> {
        let n = self.rows;
        let mut aug = Matrix::zeros(n, 2 * n);
        for r in 0..n { for c in 0..n { aug[(r, c)] = self[(r, c)]; } aug[(r, n + r)] = T::one(); }
        let (_, _, red) = aug.rref();
        for i in 0..n { if red[(i, i)] != T::one() { return None; } }
        let mut inv = Matrix::zeros(n, n);
        for r in 0..n { for c in 0..n { inv[(r, c)] = red[(r, n + c)]; } }
        Some(inv)
    }
}

fn pow<T: Copy + Zero + One + Add<Output = T> + Mul<Output = T>>(m: &Matrix<T>, mut e: u32) -> Matrix<T> {
    let mut base = m.clone();
    let mut acc = Matrix::identity(m.rows);
    while e > 0 {
        if e & 1 == 1 { acc = &acc * &base; }
        base = &base * &base;
        e >>= 1;
    }
    acc
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
struct V3 { x: f64, y: f64, z: f64 }
impl Add for V3 { type Output = V3; fn add(self, o: V3) -> V3 { V3 { x: self.x + o.x, y: self.y + o.y, z: self.z + o.z } } }
impl Sub for V3 { type Output = V3; fn sub(self, o: V3) -> V3 { V3 { x: self.x - o.x, y: self.y - o.y, z: self.z - o.z } } }
impl Mul<f64> for V3 { type Output = V3; fn mul(self, k: f64) -> V3 { V3 { x: self.x * k, y: self.y * k, z: self.z * k } } }
impl Mul<V3> for f64 { type Output = V3; fn mul(self, v: V3) -> V3 { v * self } }
impl V3 {
    fn dot(self, o: V3) -> f64 { self.x * o.x + self.y * o.y + self.z * o.z }
    fn cross(self, o: V3) -> V3 { V3 { x: self.y * o.z - self.z * o.y, y: self.z * o.x - self.x * o.z, z: self.x * o.y - self.y * o.x } }
    fn norm(self) -> f64 { self.dot(self).sqrt() }
}

fn gram_schmidt(vs: &[V3]) -> Vec<V3> {
    let mut out: Vec<V3> = Vec::new();
    for &v in vs {
        let mut w = v;
        for &u in &out { w = w - u * w.dot(u); }
        let n = w.norm();
        if n > 1e-9 { out.push(w * (1.0 / n)); }
    }
    out
}

fn main() {
    let a = Matrix::from_rows(&[[2.0, 1.0, -1.0], [-3.0, -1.0, 2.0], [-2.0, 1.0, 2.0]]);
    let b = [8.0, -11.0, -3.0];
    println!("A =\n{}", a);
    match a.solve(&b) {
        Ok(x) => println!("solution: {:?}", x.iter().map(|v| format!("{:.6}", v)).collect::<Vec<_>>()),
        Err(e) => println!("error: {:?}", e),
    }
    let (rank, det, _) = a.rref();
    println!("rank {} det {:.4}", rank, det);
    let inv = a.inverse().unwrap();
    println!("A^-1 =\n{:.4}", inv);
    let id = &a * &inv;
    println!("A*A^-1 rounded =\n{}", id.map(|x| (x * 1e9).round() / 1e9 + 0.0));

    // exact rational arithmetic
    let q = Matrix::from_rows(&[
        [Q::from(1), Q::new(1, 2), Q::new(1, 3)],
        [Q::new(1, 2), Q::new(1, 3), Q::new(1, 4)],
        [Q::new(1, 3), Q::new(1, 4), Q::new(1, 5)],
    ]);
    println!("Hilbert(3) =\n{}", q);
    let (r, d, _) = q.rref();
    println!("rank {} det {}", r, d);
    let qi = q.inverse().unwrap();
    println!("inverse =\n{}", qi);
    println!("H * H^-1 == I: {}", &q * &qi == Matrix::identity(3));
    let sol = q.solve(&[Q::from(1), Q::from(0), Q::from(0)]).unwrap();
    println!("H x = e1 -> x = [{}]", sol.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "));

    let sing = Matrix::from_rows(&[[1.0, 2.0, 3.0], [2.0, 4.0, 6.0], [1.0, 0.0, 1.0]]);
    println!("singular solve: {:?}", sing.solve(&[1.0, 2.0, 3.0]).map(|_| ()));
    println!("singular inverse: {:?}", sing.inverse().map(|m| m.rows));
    let (r, d, red) = sing.rref();
    println!("rank {} det {}\nrref:\n{:.2}", r, d, red);
    let rect = Matrix::from_rows(&[[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]]);
    println!("rect solve: {:?}", rect.solve(&[1.0, 2.0]).err());
    println!("rect^T * rect =\n{}", &rect.transpose() * &rect);

    // integer matrices: fibonacci via fast exponentiation
    let fib = Matrix::from_rows(&[[1i64, 1], [1, 0]]);
    for n in [1u32, 2, 10, 50, 63] {
        let p = pow(&fib, n);
        println!("F({}) = {}", n, p[(0, 1)]);
    }
    let mut acc = Matrix::<i64>::zeros(2, 2);
    for k in 1..=4 { acc += &pow(&fib, k); }
    print!("sum F^1..F^4 =\n{}", acc);
    let scaled = acc.clone() * 3 + Matrix::identity(2);
    print!("3*sum + I =\n{}", scaled);
    println!("row 1: {:?}", scaled.row(1));

    let basis = gram_schmidt(&[
        V3 { x: 1.0, y: 1.0, z: 0.0 },
        V3 { x: 2.0, y: 2.0, z: 0.0 },
        V3 { x: 1.0, y: 0.0, z: 1.0 },
        V3 { x: 0.0, y: 1.0, z: 1.0 },
    ]);
    println!("orthonormal basis size {}", basis.len());
    for (i, u) in basis.iter().enumerate() {
        println!("  u{} = ({:+.5}, {:+.5}, {:+.5}) |u|={:.3}", i, u.x, u.y, u.z, u.norm());
    }
    for i in 0..basis.len() {
        for j in i + 1..basis.len() {
            println!("  u{}.u{} = {:.1e}", i, j, basis[i].dot(basis[j]).abs());
        }
    }
    let c = basis[0].cross(basis[1]);
    println!("u0 x u1 = {:?} (dot u2 = {:.6})", (c.x * 1e6).round() / 1e6, c.dot(basis[2]).abs());
    println!("2 * v = {:?}", 2.0 * V3 { x: 1.0, y: -0.5, z: 0.25 });
    let r = std::panic::catch_unwind(|| {
        let m = Matrix::<i64>::identity(2);
        m[(2, 0)]
    });
    println!("out of bounds caught: {}", r.is_err());
}
