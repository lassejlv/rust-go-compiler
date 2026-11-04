use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Buf<const N: usize> { data: [u8; N], len: usize }

impl<const N: usize> Default for Buf<N> {
    fn default() -> Self { Buf { data: [0; N], len: 0 } }
}
impl<const N: usize> Buf<N> {
    const CAP: usize = N;
    fn push(&mut self, b: u8) -> Result<(), u8> {
        if self.len == N { return Err(b); }
        self.data[self.len] = b;
        self.len += 1;
        Ok(())
    }
    fn as_slice(&self) -> &[u8] { &self.data[..self.len] }
    fn resize<const M: usize>(&self) -> Buf<M> {
        let mut b = Buf::<M>::default();
        for &x in self.as_slice() { let _ = b.push(x); }
        b
    }
}

fn sum_arr<T: Copy + std::iter::Sum<T>, const N: usize>(a: [T; N]) -> T {
    a.iter().copied().sum()
}

fn transpose<T: Copy + Default, const R: usize, const C: usize>(m: [[T; C]; R]) -> [[T; R]; C] {
    let mut out = [[T::default(); R]; C];
    for r in 0..R {
        for c in 0..C {
            out[c][r] = m[r][c];
        }
    }
    out
}

fn matmul<const A: usize, const B: usize, const C: usize>(x: &[[i64; B]; A], y: &[[i64; C]; B]) -> [[i64; C]; A] {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..B).map(|k| x[i][k] * y[k][j]).sum()))
}

struct Matrix<const R: usize, const C: usize>([[f64; C]; R]);
impl<const R: usize, const C: usize> Matrix<R, C> {
    fn identity() -> Self where Self: Sized {
        Matrix(std::array::from_fn(|i| std::array::from_fn(|j| if i == j { 1.0 } else { 0.0 })))
    }
    fn dims(&self) -> (usize, usize) { (R, C) }
    fn scale(self, k: f64) -> Self { Matrix(self.0.map(|row| row.map(|v| v * k))) }
}

trait Chunk { const SIZE: usize; fn chunks(v: &[u8]) -> usize { (v.len() + Self::SIZE - 1) / Self::SIZE } }
struct C4; struct C16;
impl Chunk for C4 { const SIZE: usize = 4; }
impl Chunk for C16 { const SIZE: usize = 16; }

fn first_n<const N: usize>(v: &[i32]) -> Option<[i32; N]> {
    v.get(..N)?.try_into().ok()
}

fn pairwise<T: Debug, const N: usize>(a: &[T; N]) -> Vec<String> {
    a.each_ref().iter().zip(a.iter().skip(1)).map(|(x, y)| format!("{:?}-{:?}", x, y)).collect()
}

struct Ring<T, const N: usize> { buf: [Option<T>; N], head: usize, len: usize }
impl<T: Clone + Debug, const N: usize> Ring<T, N> {
    fn new() -> Self { Ring { buf: std::array::from_fn(|_| None), head: 0, len: 0 } }
    fn push(&mut self, x: T) {
        let idx = (self.head + self.len) % N;
        self.buf[idx] = Some(x);
        if self.len < N { self.len += 1 } else { self.head = (self.head + 1) % N }
    }
    fn to_vec(&self) -> Vec<T> {
        (0..self.len).map(|i| self.buf[(self.head + i) % N].clone().unwrap()).collect()
    }
}

fn main() {
    let mut b: Buf<4> = Default::default();
    for i in 1..=6 {
        if let Err(x) = b.push(i * 10) { println!("full, dropped {}", x); }
    }
    println!("{:?} cap={}", b.as_slice(), Buf::<4>::CAP);
    let small: Buf<2> = b.resize();
    let big = b.resize::<8>();
    println!("{:?} {:?} {}", small.as_slice(), big.as_slice(), big.data.len());
    println!("{} {}", sum_arr([1, 2, 3, 4]), sum_arr([0.5, 0.25]));
    let m = [[1, 2, 3], [4, 5, 6]];
    println!("{:?}", transpose(m));
    let x = [[1, 2], [3, 4], [5, 6]];
    let y = [[7, 8, 9], [10, 11, 12]];
    println!("{:?}", matmul(&x, &y));
    let id = Matrix::<3, 3>::identity().scale(2.5);
    println!("{:?} {:?}", id.dims(), id.0);
    let r: Matrix<2, 4> = Matrix::identity();
    println!("{:?} {:?}", r.dims(), r.0[1]);
    println!("{} {}", C4::chunks(&[0; 9]), C16::chunks(&[0; 33]));
    println!("{:?} {:?}", first_n::<2>(&[9, 8, 7]), first_n::<5>(&[1]));
    let arr3: [[[u8; 2]; 2]; 2] = std::array::from_fn(|i| std::array::from_fn(|j| std::array::from_fn(|k| (i * 4 + j * 2 + k) as u8)));
    println!("{:?}", arr3);
    let flat: Vec<u8> = arr3.iter().flatten().flatten().copied().collect();
    println!("{:?}", flat);
    println!("{:?}", pairwise(&["a", "b", "c"]));
    let strs = ["x".to_string(), "yy".to_string()];
    let lens = strs.each_ref().map(|s| s.len());
    println!("{:?} {:?}", lens, strs);
    let mut ring: Ring<String, 3> = Ring::new();
    for w in "a b c d e".split(' ') { ring.push(w.to_string()); }
    println!("{:?}", ring.to_vec());
    let squares: [u64; 6] = std::array::from_fn(|i| (i * i) as u64);
    let [a, bb, rest @ ..] = squares;
    println!("{} {} {:?}", a, bb, rest);
    println!("{:?}", squares.map(|v| v % 3 == 0));
}
