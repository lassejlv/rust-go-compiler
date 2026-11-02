// Const generics: arrays parameterized by N, const params in impls and methods, const expressions
// with {N * 2} via fixed helper consts, default const params, const generic matrices, associated
// consts derived from const params, and [T; N] trait impls (Default, Debug, map, zip).
use std::ops::{Add, Mul};

#[derive(Debug, Clone, Copy, PartialEq)]
struct Vector<const N: usize>([i64; N]);

impl<const N: usize> Vector<N> {
    const DIM: usize = N;
    const BYTES: usize = N * 8;
    fn zero() -> Self {
        Vector([0; N])
    }
    fn dot(&self, o: &Self) -> i64 {
        self.0.iter().zip(o.0.iter()).map(|(a, b)| a * b).sum()
    }
    fn extend_one(&self, v: i64) -> Vec<i64> {
        let mut out = self.0.to_vec();
        out.push(v);
        out
    }
}

impl<const N: usize> Add for Vector<N> {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        let mut r = [0; N];
        for i in 0..N {
            r[i] = self.0[i] + o.0[i];
        }
        Vector(r)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Matrix<const R: usize, const C: usize>([[i32; C]; R]);

impl<const R: usize, const C: usize> Matrix<R, C> {
    fn transpose(&self) -> Matrix<C, R> {
        let mut t = [[0; R]; C];
        for i in 0..R {
            for j in 0..C {
                t[j][i] = self.0[i][j];
            }
        }
        Matrix(t)
    }
    fn shape(&self) -> (usize, usize) {
        (R, C)
    }
}

impl<const R: usize, const K: usize, const C: usize> Mul<Matrix<K, C>> for Matrix<R, K> {
    type Output = Matrix<R, C>;
    fn mul(self, o: Matrix<K, C>) -> Matrix<R, C> {
        let mut m = [[0; C]; R];
        for i in 0..R {
            for j in 0..C {
                for k in 0..K {
                    m[i][j] += self.0[i][k] * o.0[k][j];
                }
            }
        }
        Matrix(m)
    }
}

struct RingBuf<T: Copy + Default, const CAP: usize = 4> {
    data: [T; CAP],
    head: usize,
    len: usize,
}
impl<T: Copy + Default + std::fmt::Debug, const CAP: usize> RingBuf<T, CAP> {
    fn new() -> Self {
        RingBuf { data: [T::default(); CAP], head: 0, len: 0 }
    }
    fn push(&mut self, v: T) -> Option<T> {
        let idx = (self.head + self.len) % CAP;
        if self.len == CAP {
            let old = self.data[self.head];
            self.data[self.head] = v;
            self.head = (self.head + 1) % CAP;
            Some(old)
        } else {
            self.data[idx] = v;
            self.len += 1;
            None
        }
    }
    fn to_vec(&self) -> Vec<T> {
        (0..self.len).map(|i| self.data[(self.head + i) % CAP]).collect()
    }
}

fn sum_array<const N: usize>(a: [u32; N]) -> u32 {
    a.iter().sum()
}

fn first_last<T: Copy, const N: usize>(a: &[T; N]) -> Option<(T, T)> {
    if N == 0 { None } else { Some((a[0], a[N - 1])) }
}

fn split_pairs<const N: usize>(a: [u8; N]) -> Vec<[u8; 2]> {
    a.chunks_exact(2).map(|c| [c[0], c[1]]).collect()
}

fn main() {
    let a = Vector([1, 2, 3]);
    let b = Vector([4, 5, 6]);
    println!("{:?} dot={} dim={} bytes={}", a + b, a.dot(&b), Vector::<3>::DIM, Vector::<3>::BYTES);
    println!("{:?} {:?}", Vector::<5>::zero(), a.extend_one(9));

    let m = Matrix([[1, 2, 3], [4, 5, 6]]);
    let t = m.transpose();
    println!("{:?} shape {:?}", t, t.shape());
    let p = m * t;
    println!("{:?} shape {:?}", p, p.shape());
    let q = t * m;
    println!("{:?}", q);
    let id = Matrix([[1, 0], [0, 1]]);
    println!("{}", p * id == p);

    let mut r: RingBuf<char> = RingBuf::new();
    for c in "abcdef".chars() {
        if let Some(ev) = r.push(c) {
            print!("evict {} ", ev);
        }
    }
    println!("-> {:?}", r.to_vec());
    let mut r2: RingBuf<u8, 2> = RingBuf::new();
    r2.push(1);
    println!("{:?} {:?} {:?}", r2.push(2), r2.push(3), r2.to_vec());

    println!("{} {} {}", sum_array([1, 2, 3]), sum_array([10; 10]), sum_array([]));
    println!("{:?} {:?}", first_last(&[7, 8, 9]), first_last::<i32, 0>(&[]));
    println!("{:?}", split_pairs([1, 2, 3, 4, 5]));

    let squares: [u64; 6] = std::array::from_fn(|i| (i as u64 + 1).pow(2));
    let doubled = squares.map(|x| x * 2);
    println!("{:?} {:?}", squares, doubled);
    let names = ["a", "b", "c"];
    let zipped: [(char, &str); 3] = std::array::from_fn(|i| ((b'x' + i as u8) as char, names[i]));
    println!("{:?}", zipped);
    let def: [Option<u8>; 3] = Default::default();
    println!("{:?} {}", def, [[0u8; 3]; 4].len() * [[0u8; 3]; 4][0].len());
    let arr2: [[i32; 2]; 3] = [[1, 2], [3, 4], [5, 6]];
    let flat: Vec<i32> = arr2.iter().flatten().copied().collect();
    println!("{:?} {:?}", flat, arr2.as_flattened());
    let conv: [u8; 4] = vec![9, 8, 7, 6].try_into().unwrap();
    let bad: Result<[u8; 2], _> = vec![1, 2, 3].try_into();
    println!("{:?} {:?}", conv, bad);
}
