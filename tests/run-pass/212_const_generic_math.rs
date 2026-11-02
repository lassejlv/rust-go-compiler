use std::fmt;
use std::ops::{Add, Index, IndexMut, Mul, Sub};

#[derive(Clone, Copy, PartialEq, Debug)]
struct Vector<const N: usize>([f64; N]);

impl<const N: usize> Vector<N> {
    const ZERO: Self = Vector([0.0; N]);
    const DIM: usize = N;

    fn dot(&self, o: &Self) -> f64 {
        let mut s = 0.0;
        for i in 0..N {
            s += self.0[i] * o.0[i];
        }
        s
    }
    fn norm_sq(&self) -> f64 {
        self.dot(self)
    }
    fn map(self, f: impl Fn(f64) -> f64) -> Self {
        Vector(self.0.map(f))
    }
}

impl<const N: usize> Add for Vector<N> {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        let mut r = self;
        for i in 0..N {
            r.0[i] += o.0[i];
        }
        r
    }
}

impl<const N: usize> Sub for Vector<N> {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        let mut r = self;
        for (a, b) in r.0.iter_mut().zip(o.0.iter()) {
            *a -= b;
        }
        r
    }
}

impl<const N: usize> Mul<f64> for Vector<N> {
    type Output = Self;
    fn mul(self, k: f64) -> Self {
        self.map(|x| x * k)
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Mat<const R: usize, const C: usize>([[i64; C]; R]);

impl<const R: usize, const C: usize> Mat<R, C> {
    fn zero() -> Self {
        Mat([[0; C]; R])
    }
    fn transpose(&self) -> Mat<C, R> {
        let mut t = Mat::<C, R>::zero();
        for i in 0..R {
            for j in 0..C {
                t.0[j][i] = self.0[i][j];
            }
        }
        t
    }
    fn mul<const K: usize>(&self, o: &Mat<C, K>) -> Mat<R, K> {
        let mut out = Mat::<R, K>::zero();
        for i in 0..R {
            for j in 0..K {
                let mut s = 0;
                for k in 0..C {
                    s += self.0[i][k] * o.0[k][j];
                }
                out.0[i][j] = s;
            }
        }
        out
    }
}

impl<const N: usize> Mat<N, N> {
    fn identity() -> Self {
        let mut m = Self::zero();
        for i in 0..N {
            m.0[i][i] = 1;
        }
        m
    }
    fn pow(&self, mut e: u32) -> Self {
        let mut base = *self;
        let mut acc = Self::identity();
        while e > 0 {
            if e & 1 == 1 {
                acc = acc.mul(&base);
            }
            base = base.mul(&base);
            e >>= 1;
        }
        acc
    }
    fn trace(&self) -> i64 {
        (0..N).map(|i| self.0[i][i]).sum()
    }
}

impl<const R: usize, const C: usize> Index<(usize, usize)> for Mat<R, C> {
    type Output = i64;
    fn index(&self, (r, c): (usize, usize)) -> &i64 {
        &self.0[r][c]
    }
}
impl<const R: usize, const C: usize> IndexMut<(usize, usize)> for Mat<R, C> {
    fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut i64 {
        &mut self.0[r][c]
    }
}

impl<const R: usize, const C: usize> fmt::Display for Mat<R, C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Mat<{}x{}>", R, C)?;
        for row in &self.0 {
            let cells: Vec<String> = row.iter().map(|x| format!("{:>4}", x)).collect();
            writeln!(f, "  [{}]", cells.join(""))?;
        }
        Ok(())
    }
}

fn concat<const A: usize, const B: usize>(a: [u8; A], b: [u8; B]) -> Vec<u8> {
    let mut v = Vec::with_capacity(A + B);
    v.extend_from_slice(&a);
    v.extend_from_slice(&b);
    v
}

fn split_first_n<const N: usize>(s: &[i32]) -> Option<([i32; N], &[i32])> {
    if s.len() < N {
        return None;
    }
    let (head, tail) = s.split_at(N);
    Some((head.try_into().unwrap(), tail))
}

fn main() {
    let a = Vector([1.0, 2.0, 3.0]);
    let b = Vector([4.0, -5.0, 6.0]);
    println!("{:?}", a + b);
    println!("{:?}", b - a);
    println!("{:?}", a * 2.5);
    println!("dot={} norm_sq={} dim={}", a.dot(&b), b.norm_sq(), Vector::<3>::DIM);
    println!("{:?} {}", Vector::<4>::ZERO, Vector::<2>::ZERO == Vector([0.0, 0.0]));

    let m = Mat([[1, 2, 3], [4, 5, 6]]);
    print!("{}", m);
    print!("{}", m.transpose());
    print!("{}", m.mul(&m.transpose()));
    let fibm = Mat([[1, 1], [1, 0]]);
    let p = fibm.pow(30);
    println!("fib(30)={} trace={}", p[(0, 1)], p.trace());
    let mut i3 = Mat::<3, 3>::identity();
    i3[(0, 2)] = 7;
    print!("{}", i3.pow(3));

    println!("{:?}", concat([1, 2], [3, 4, 5]));
    let data = [9, 8, 7, 6, 5];
    if let Some((h, t)) = split_first_n::<2>(&data) {
        println!("{:?} {:?}", h, t);
    }
    println!("{:?}", split_first_n::<9>(&data).is_none());
}
