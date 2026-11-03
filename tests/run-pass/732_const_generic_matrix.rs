// Const-generic matrices: Matrix<R, C> with Mul producing Matrix<R, P>,
// transpose, identity via const fn, determinant for small sizes through a
// trait, arrays built with from_fn, and const evaluation of tables.
use std::fmt;
use std::ops::{Add, Index, IndexMut, Mul};

#[derive(Clone, Copy, PartialEq, Debug)]
struct Matrix<const R: usize, const C: usize> { d: [[i64; C]; R] }

impl<const R: usize, const C: usize> Matrix<R, C> {
    const ROWS: usize = R;
    const COLS: usize = C;
    fn zero() -> Self { Matrix { d: [[0; C]; R] } }
    fn from_fn(f: impl Fn(usize, usize) -> i64) -> Self { Matrix { d: std::array::from_fn(|r| std::array::from_fn(|c| f(r, c))) } }
    fn transpose(&self) -> Matrix<C, R> { Matrix::from_fn(|r, c| self.d[c][r]) }
    fn row(&self, r: usize) -> [i64; C] { self.d[r] }
    fn col(&self, c: usize) -> [i64; R] { self.d.map(|row| row[c]) }
    fn trace(&self) -> i64 where Self: Square { (0..R.min(C)).map(|i| self.d[i][i]).sum() }
}

trait Square { fn det(&self) -> i64; }
impl Square for Matrix<1, 1> { fn det(&self) -> i64 { self.d[0][0] } }
impl Square for Matrix<2, 2> { fn det(&self) -> i64 { self.d[0][0] * self.d[1][1] - self.d[0][1] * self.d[1][0] } }
impl Square for Matrix<3, 3> {
    fn det(&self) -> i64 {
        (0..3).map(|c| {
            let minor = Matrix::<2, 2>::from_fn(|r, k| self.d[r + 1][if k < c { k } else { k + 1 }]);
            let sign = if c % 2 == 0 { 1 } else { -1 };
            sign * self.d[0][c] * minor.det()
        }).sum()
    }
}

impl<const N: usize> Matrix<N, N> {
    fn identity() -> Self { Matrix::from_fn(|r, c| (r == c) as i64) }
    fn pow(self, mut e: u32) -> Self {
        let mut base = self;
        let mut acc = Self::identity();
        while e > 0 {
            if e & 1 == 1 { acc = acc * base; }
            base = base * base;
            e >>= 1;
        }
        acc
    }
}

impl<const R: usize, const C: usize, const P: usize> Mul<Matrix<C, P>> for Matrix<R, C> {
    type Output = Matrix<R, P>;
    fn mul(self, o: Matrix<C, P>) -> Matrix<R, P> {
        let mut out = Matrix::<R, P>::zero();
        for r in 0..R { for p in 0..P { out.d[r][p] = (0..C).map(|k| self.d[r][k] * o.d[k][p]).sum(); } }
        out
    }
}
impl<const R: usize, const C: usize> Add for Matrix<R, C> {
    type Output = Self;
    fn add(self, o: Self) -> Self { Matrix::from_fn(|r, c| self.d[r][c] + o.d[r][c]) }
}
impl<const R: usize, const C: usize> Index<(usize, usize)> for Matrix<R, C> {
    type Output = i64;
    fn index(&self, (r, c): (usize, usize)) -> &i64 { &self.d[r][c] }
}
impl<const R: usize, const C: usize> IndexMut<(usize, usize)> for Matrix<R, C> {
    fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut i64 { &mut self.d[r][c] }
}
impl<const R: usize, const C: usize> fmt::Display for Matrix<R, C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let w = self.d.iter().flatten().map(|x| x.to_string().len()).max().unwrap_or(1);
        for row in &self.d {
            let cells: Vec<String> = row.iter().map(|x| format!("{:>w$}", x, w = w)).collect();
            writeln!(f, "[{}]", cells.join(" "))?;
        }
        Ok(())
    }
}

const fn binomials<const N: usize>() -> [[u64; N]; N] {
    let mut t = [[0u64; N]; N];
    let mut n = 0;
    while n < N {
        t[n][0] = 1;
        let mut k = 1;
        while k <= n { t[n][k] = t[n - 1][k - 1] + if k < n { t[n - 1][k] } else { 0 }; k += 1; }
        n += 1;
    }
    t
}
const PASCAL: [[u64; 8]; 8] = binomials::<8>();
const SUM_ROW7: u64 = { let mut s = 0; let mut i = 0; while i < 8 { s += PASCAL[7][i]; i += 1; } s };

fn concat<const A: usize, const B: usize>(a: [u8; A], b: [u8; B]) -> Vec<u8> { a.iter().chain(b.iter()).copied().collect() }
fn split_last_n<const N: usize>(v: &[u8]) -> Option<&[u8; N]> { v.last_chunk::<N>() }

fn main() {
    let a = Matrix::<2, 3>::from_fn(|r, c| (r * 3 + c) as i64 + 1);
    let b = a.transpose();
    println!("A ({}x{}):\n{}B = A^T ({}x{}):\n{}", Matrix::<2, 3>::ROWS, Matrix::<2, 3>::COLS, a, Matrix::<3, 2>::ROWS, Matrix::<3, 2>::COLS, b);
    let ab = a * b;
    let ba = b * a;
    println!("A*B:\n{}B*A:\n{}", ab, ba);
    println!("det(AB)={} det(BA)={} trace(AB)={} trace(BA)={}", ab.det(), ba.det(), ab.trace(), ba.trace());
    let fib = Matrix::<2, 2> { d: [[1, 1], [1, 0]] };
    println!("fib^10 = {:?}, fib(50) = {}", fib.pow(10).d, fib.pow(50)[(0, 1)]);
    let mut m3 = Matrix::<3, 3>::from_fn(|r, c| ((r + 2) * (c + 3) % 7) as i64 - 3);
    m3[(1, 1)] = 10;
    println!("m3:\n{}det={} row1={:?} col2={:?}", m3, m3.det(), m3.row(1), m3.col(2));
    println!("I3 + m3 det = {}", (Matrix::<3, 3>::identity() + m3).det());
    let v = Matrix::<3, 1>::from_fn(|r, _| r as i64 - 1);
    println!("m3 * v = {:?}", (m3 * v).d);
    println!("1x1 det {}", Matrix::<1, 1> { d: [[42]] }.det());
    println!("pascal row 7: {:?} sum={}", PASCAL[7], SUM_ROW7);
    println!("concat: {:?}", concat([1, 2], [3, 4, 5]));
    println!("last 2: {:?} last 9: {:?}", split_last_n::<2>(&[9, 8, 7]), split_last_n::<9>(&[1]));
    let squares: [u32; 6] = std::array::from_fn(|i| (i * i) as u32);
    let strs = squares.map(|x| format!("<{}>", x));
    println!("{:?} {:?}", squares, strs);
    let [first, .., last] = squares;
    println!("first={} last={} zipped={:?}", first, last, squares.iter().zip(strs.iter()).nth(2));
}
