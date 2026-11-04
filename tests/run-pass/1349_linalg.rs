// Const-generic matrices + a numeric trait hierarchy, rational numbers,
// Gaussian elimination, determinant, inverse, and a dynamic matrix type.
use std::fmt::{self, Debug, Display};
use std::iter::Sum;
use std::ops::{Add, AddAssign, Div, Index, IndexMut, Mul, Neg, Sub};

trait Zero {
    fn zero() -> Self;
    fn is_zero(&self) -> bool;
}
trait One {
    fn one() -> Self;
}
trait Field: Copy + PartialEq + Debug + Display + Zero + One + Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Div<Output = Self> + Neg<Output = Self> {
    fn abs_val(self) -> Self;
    fn gt(self, o: Self) -> bool;
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 {
        a.abs()
    } else {
        gcd(b, a % b)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Q {
    n: i64,
    d: i64,
}

impl Q {
    fn new(n: i64, d: i64) -> Q {
        assert!(d != 0, "zero denominator");
        let g = gcd(n, d).max(1);
        let s = if d < 0 { -1 } else { 1 };
        Q { n: s * n / g, d: s * d / g }
    }
}
impl Debug for Q {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.d == 1 {
            write!(f, "{}", self.n)
        } else {
            write!(f, "{}/{}", self.n, self.d)
        }
    }
}
impl Display for Q {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!("{:?}", self);
        f.pad(&s)
    }
}
impl From<i64> for Q {
    fn from(n: i64) -> Q {
        Q::new(n, 1)
    }
}
impl Zero for Q {
    fn zero() -> Q {
        Q::from(0)
    }
    fn is_zero(&self) -> bool {
        self.n == 0
    }
}
impl One for Q {
    fn one() -> Q {
        Q::from(1)
    }
}
impl Add for Q {
    type Output = Q;
    fn add(self, o: Q) -> Q {
        Q::new(self.n * o.d + o.n * self.d, self.d * o.d)
    }
}
impl Sub for Q {
    type Output = Q;
    fn sub(self, o: Q) -> Q {
        self + -o
    }
}
impl Mul for Q {
    type Output = Q;
    fn mul(self, o: Q) -> Q {
        Q::new(self.n * o.n, self.d * o.d)
    }
}
impl Div for Q {
    type Output = Q;
    fn div(self, o: Q) -> Q {
        Q::new(self.n * o.d, self.d * o.n)
    }
}
impl Neg for Q {
    type Output = Q;
    fn neg(self) -> Q {
        Q { n: -self.n, d: self.d }
    }
}
impl PartialOrd for Q {
    fn partial_cmp(&self, o: &Q) -> Option<std::cmp::Ordering> {
        (self.n as i128 * o.d as i128).partial_cmp(&(o.n as i128 * self.d as i128))
    }
}
impl Field for Q {
    fn abs_val(self) -> Q {
        Q { n: self.n.abs(), d: self.d }
    }
    fn gt(self, o: Q) -> bool {
        self > o
    }
}
impl Zero for f64 {
    fn zero() -> f64 {
        0.0
    }
    fn is_zero(&self) -> bool {
        self.abs() < 1e-12
    }
}
impl One for f64 {
    fn one() -> f64 {
        1.0
    }
}
impl Field for f64 {
    fn abs_val(self) -> f64 {
        self.abs()
    }
    fn gt(self, o: f64) -> bool {
        self > o
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Mat<T, const R: usize, const C: usize> {
    a: [[T; C]; R],
}

impl<T: Field, const R: usize, const C: usize> Mat<T, R, C> {
    fn zero() -> Self {
        Mat { a: [[T::zero(); C]; R] }
    }
    fn from_fn(f: impl Fn(usize, usize) -> T) -> Self {
        let mut m = Self::zero();
        for i in 0..R {
            for j in 0..C {
                m.a[i][j] = f(i, j);
            }
        }
        m
    }
    fn transpose(&self) -> Mat<T, C, R> {
        Mat::from_fn(|i, j| self.a[j][i])
    }
    const SHAPE: (usize, usize) = (R, C);
    fn rows(&self) -> impl Iterator<Item = &[T; C]> + '_ {
        self.a.iter()
    }
}

impl<T: Field, const N: usize> Mat<T, N, N> {
    fn identity() -> Self {
        Self::from_fn(|i, j| if i == j { T::one() } else { T::zero() })
    }
    fn trace(&self) -> T {
        (0..N).fold(T::zero(), |acc, i| acc + self.a[i][i])
    }
    /// Returns (determinant, inverse if it exists)
    fn det_inv(&self) -> (T, Option<Self>) {
        let mut a = self.a;
        let mut inv = Self::identity().a;
        let mut det = T::one();
        for col in 0..N {
            // partial pivoting
            let piv = (col..N).fold(col, |best, r| if a[r][col].abs_val().gt(a[best][col].abs_val()) { r } else { best });
            if a[piv][col].is_zero() {
                return (T::zero(), None);
            }
            if piv != col {
                a.swap(piv, col);
                inv.swap(piv, col);
                det = -det;
            }
            let p = a[col][col];
            det = det * p;
            for j in 0..N {
                a[col][j] = a[col][j] / p;
                inv[col][j] = inv[col][j] / p;
            }
            for r in 0..N {
                if r != col {
                    let f = a[r][col];
                    if !f.is_zero() {
                        for j in 0..N {
                            a[r][j] = a[r][j] - f * a[col][j];
                            inv[r][j] = inv[r][j] - f * inv[col][j];
                        }
                    }
                }
            }
        }
        (det, Some(Mat { a: inv }))
    }
    fn pow(&self, mut e: u32) -> Self {
        let mut base = *self;
        let mut acc = Self::identity();
        while e > 0 {
            if e & 1 == 1 {
                acc = acc * base;
            }
            base = base * base;
            e >>= 1;
        }
        acc
    }
}

impl<T: Field, const R: usize, const K: usize, const C: usize> Mul<Mat<T, K, C>> for Mat<T, R, K> {
    type Output = Mat<T, R, C>;
    fn mul(self, o: Mat<T, K, C>) -> Mat<T, R, C> {
        Mat::from_fn(|i, j| (0..K).fold(T::zero(), |s, k| s + self.a[i][k] * o.a[k][j]))
    }
}

impl<T: Field, const R: usize, const C: usize> Add for Mat<T, R, C> {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Mat::from_fn(|i, j| self.a[i][j] + o.a[i][j])
    }
}

impl<T: Field, const R: usize, const C: usize> Index<(usize, usize)> for Mat<T, R, C> {
    type Output = T;
    fn index(&self, (i, j): (usize, usize)) -> &T {
        &self.a[i][j]
    }
}
impl<T: Field, const R: usize, const C: usize> IndexMut<(usize, usize)> for Mat<T, R, C> {
    fn index_mut(&mut self, (i, j): (usize, usize)) -> &mut T {
        &mut self.a[i][j]
    }
}

impl<T: Field, const R: usize, const C: usize> Display for Mat<T, R, C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let cells: Vec<Vec<String>> = self.rows().map(|r| r.iter().map(|x| format!("{}", x)).collect()).collect();
        let w = cells.iter().flatten().map(|s| s.len()).max().unwrap_or(1);
        for (i, row) in cells.iter().enumerate() {
            let (l, r) = match (i, R) {
                (_, 1) => ('[', ']'),
                (0, _) => ('┌', '┐'),
                (i, _) if i == R - 1 => ('└', '┘'),
                _ => ('│', '│'),
            };
            write!(f, "{}", l)?;
            for (j, c) in row.iter().enumerate() {
                if j > 0 {
                    f.write_str(" ")?;
                }
                write!(f, "{:>w$}", c, w = w)?;
            }
            writeln!(f, "{}", r)?;
        }
        Ok(())
    }
}

// Dynamic-size matrix with slices of rows
#[derive(Clone, Debug, PartialEq)]
struct DMat {
    r: usize,
    c: usize,
    data: Vec<f64>,
}

impl DMat {
    fn new(r: usize, c: usize) -> Self {
        DMat { r, c, data: vec![0.0; r * c] }
    }
    fn row(&self, i: usize) -> &[f64] {
        &self.data[i * self.c..(i + 1) * self.c]
    }
    fn row_mut(&mut self, i: usize) -> &mut [f64] {
        let c = self.c;
        &mut self.data[i * c..(i + 1) * c]
    }
    fn matmul(&self, o: &DMat) -> Result<DMat, String> {
        if self.c != o.r {
            return Err(format!("shape mismatch {}x{} * {}x{}", self.r, self.c, o.r, o.c));
        }
        let mut out = DMat::new(self.r, o.c);
        for i in 0..self.r {
            for k in 0..self.c {
                let a = self.data[i * self.c + k];
                if a == 0.0 {
                    continue;
                }
                let orow = o.row(k);
                for (dst, &b) in out.row_mut(i).iter_mut().zip(orow) {
                    *dst += a * b;
                }
            }
        }
        Ok(out)
    }
    fn solve(mut self, mut b: Vec<f64>) -> Option<Vec<f64>> {
        let n = self.r;
        for col in 0..n {
            let piv = (col..n).max_by(|&x, &y| self.data[x * n + col].abs().total_cmp(&self.data[y * n + col].abs()))?;
            if self.data[piv * n + col].abs() < 1e-12 {
                return None;
            }
            if piv != col {
                for j in 0..n {
                    self.data.swap(piv * n + j, col * n + j);
                }
                b.swap(piv, col);
            }
            for r in col + 1..n {
                let f = self.data[r * n + col] / self.data[col * n + col];
                for j in col..n {
                    let v = self.data[col * n + j];
                    self.data[r * n + j] -= f * v;
                }
                b[r] -= f * b[col];
            }
        }
        let mut x = vec![0.0; n];
        for i in (0..n).rev() {
            let s: f64 = (i + 1..n).map(|j| self.data[i * n + j] * x[j]).sum();
            x[i] = (b[i] - s) / self.data[i * n + i];
        }
        Some(x)
    }
}

impl AddAssign<&DMat> for DMat {
    fn add_assign(&mut self, o: &DMat) {
        self.data.iter_mut().zip(&o.data).for_each(|(a, b)| *a += b);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Poly<const N: usize>([i64; N]);

impl<const N: usize> Poly<N> {
    fn eval(&self, x: i64) -> i64 {
        self.0.iter().rev().fold(0, |acc, &c| acc * x + c)
    }
    fn derive(&self) -> Poly<N> {
        let mut out = [0; N];
        for i in 1..N {
            out[i - 1] = self.0[i] * i as i64;
        }
        Poly(out)
    }
}

impl<const N: usize> Sum for Poly<N> {
    fn sum<I: Iterator<Item = Self>>(it: I) -> Self {
        it.fold(Poly([0; N]), |mut a, b| {
            for (x, y) in a.0.iter_mut().zip(b.0) {
                *x += y;
            }
            a
        })
    }
}

fn main() {
    let q = |n, d| Q::new(n, d);
    let m: Mat<Q, 3, 3> = Mat { a: [[q(2, 1), q(1, 1), q(1, 1)], [q(1, 1), q(3, 1), q(2, 1)], [q(1, 1), q(0, 1), q(0, 1)]] };
    print!("M =\n{}", m);
    let (det, inv) = m.det_inv();
    println!("det = {}, trace = {}", det, m.trace());
    let inv = inv.unwrap();
    print!("M^-1 =\n{}", inv);
    let id = m * inv;
    println!("M*M^-1 == I: {}", id == Mat::identity());
    print!("M^5 =\n{}", m.pow(5));
    let r: Mat<Q, 2, 3> = Mat::from_fn(|i, j| q((i * 3 + j) as i64, 2));
    print!("R =\n{}R^T =\n{}R*R^T =\n{}", r, r.transpose(), r * r.transpose());
    println!("shape {:?} {:?}", Mat::<Q, 2, 3>::SHAPE, Mat::<f64, 4, 1>::SHAPE);
    let sing: Mat<Q, 3, 3> = Mat::from_fn(|i, j| q((i + j) as i64, 1));
    println!("singular det = {}, inverse = {:?}", sing.det_inv().0, sing.det_inv().1.is_some());

    let hilbert: Mat<Q, 4, 4> = Mat::from_fn(|i, j| q(1, (i + j + 1) as i64));
    let (hd, hinv) = hilbert.det_inv();
    println!("hilbert4 det = {}", hd);
    print!("hilbert4^-1 =\n{}", hinv.unwrap());

    let mut fm: Mat<f64, 3, 3> = Mat::from_fn(|i, j| if i == j { 4.0 } else { 1.0 / (1.0 + i as f64 + j as f64) });
    fm[(0, 2)] = -0.5;
    let (fd, finv) = fm.det_inv();
    let check = fm * finv.unwrap();
    let err = (0..3).flat_map(|i| (0..3).map(move |j| (i, j))).map(|(i, j)| (check[(i, j)] - if i == j { 1.0 } else { 0.0 }).abs()).fold(0.0, f64::max);
    println!("float det = {:.6}, max err < 1e-12: {}", fd, err < 1e-12);
    let fib: Mat<f64, 2, 2> = Mat { a: [[1.0, 1.0], [1.0, 0.0]] };
    println!("fib via matrix: {}", fib.pow(40)[(0, 1)]);
    let fq: Mat<Q, 2, 2> = Mat { a: [[q(1, 1), q(1, 1)], [q(1, 1), q(0, 1)]] };
    println!("fib via Q matrix: {}", fq.pow(50)[(0, 1)]);

    // dynamic matrices
    let mut a = DMat::new(3, 4);
    for (k, x) in a.data.iter_mut().enumerate() {
        *x = (k as f64 * 0.5) - 2.0;
    }
    let mut b = DMat::new(4, 2);
    b.data.iter_mut().enumerate().for_each(|(k, x)| *x = (k % 3) as f64);
    let c = a.matmul(&b).unwrap();
    println!("{:?}", c);
    println!("{:?}", b.matmul(&b));
    let mut c2 = c.clone();
    c2 += &c;
    println!("rows: {:?} / {:?}", c2.row(0), c2.row(2));
    let sys = DMat { r: 3, c: 3, data: vec![2.0, 1.0, -1.0, -3.0, -1.0, 2.0, -2.0, 1.0, 2.0] };
    let x = sys.clone().solve(vec![8.0, -11.0, -3.0]).unwrap();
    println!("solution: {:?}", x.iter().map(|v| format!("{:.6}", v)).collect::<Vec<_>>());
    println!("singular solve: {:?}", DMat { r: 2, c: 2, data: vec![1.0, 2.0, 2.0, 4.0] }.solve(vec![1.0, 2.0]));

    let p = Poly([1, -3, 0, 2]);
    println!("p(2)={} p'(2)={} p''={:?}", p.eval(2), p.derive().eval(2), p.derive().derive());
    let s: Poly<4> = vec![p, p.derive(), Poly([0; 4])].into_iter().sum();
    println!("sum {:?} at -1 = {}", s, s.eval(-1));
    let mut qs = vec![q(1, 3), q(-1, 2), q(5, 7), q(2, 6), q(0, 1)];
    qs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    qs.dedup();
    println!("{:?} sum = {}", qs, qs.iter().copied().fold(Q::zero(), |a, b| a + b));
    println!("|{:>6}|{:<6}|{:^7}|", q(3, 4), q(-1, 2), q(10, 5));
}
