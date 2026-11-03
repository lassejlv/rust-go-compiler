// const generic params in arrays, functions, impl blocks
use std::fmt::Debug;

fn sum_arr<const N: usize>(a: [i32; N]) -> i32 {
    a.iter().sum()
}

fn first_last<T: Copy + Default, const N: usize>(a: &[T; N]) -> (T, T) {
    if N == 0 {
        (T::default(), T::default())
    } else {
        (a[0], a[N - 1])
    }
}

fn chunks_of<const K: usize>(v: &[u8]) -> Vec<[u8; K]> {
    v.chunks_exact(K).map(|c| {
        let mut arr = [0u8; K];
        arr.copy_from_slice(c);
        arr
    }).collect()
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Vector<const D: usize>([f64; D]);

impl<const D: usize> Vector<D> {
    fn zero() -> Self {
        Vector([0.0; D])
    }
    fn dot(&self, o: &Self) -> f64 {
        self.0.iter().zip(o.0.iter()).map(|(a, b)| a * b).sum()
    }
    fn add(&self, o: &Self) -> Self {
        let mut r = [0.0; D];
        for i in 0..D {
            r[i] = self.0[i] + o.0[i];
        }
        Vector(r)
    }
    fn dim(&self) -> usize {
        D
    }
}

struct RingBuf<T, const N: usize> {
    buf: [Option<T>; N],
    head: usize,
    len: usize,
}
impl<T: Copy + Debug, const N: usize> RingBuf<T, N> {
    fn new() -> Self {
        RingBuf { buf: [None; N], head: 0, len: 0 }
    }
    fn push(&mut self, v: T) -> Option<T> {
        let idx = (self.head + self.len) % N;
        if self.len == N {
            let old = self.buf[self.head].replace(v);
            self.head = (self.head + 1) % N;
            old
        } else {
            self.buf[idx] = Some(v);
            self.len += 1;
            None
        }
    }
    fn items(&self) -> Vec<T> {
        (0..self.len).map(|i| self.buf[(self.head + i) % N].unwrap()).collect()
    }
}

trait Arr {
    const LEN: usize;
}
impl<T, const N: usize> Arr for [T; N] {
    const LEN: usize = N;
}

fn zip_arrays<A: Copy, B: Copy, const N: usize>(a: [A; N], b: [B; N]) -> [(A, B); N] {
    std::array::from_fn(|i| (a[i], b[i]))
}

fn main() {
    println!("{}", sum_arr([1, 2, 3]));
    println!("{}", sum_arr([10; 7]));
    println!("{}", sum_arr::<0>([]));
    println!("{:?}", first_last(&[5, 6, 7, 8]));
    println!("{:?}", first_last::<char, 0>(&[]));
    println!("{:?}", first_last(&["a", "z"]));
    println!("{:?}", chunks_of::<3>(&[1, 2, 3, 4, 5, 6, 7]));
    println!("{:?}", chunks_of::<2>(b"abcd"));
    let a = Vector([1.0, 2.0, 3.0]);
    let b = Vector([0.5, -1.0, 2.0]);
    println!("{} {:?} {}", a.dot(&b), a.add(&b), a.dim());
    println!("{:?} {}", Vector::<4>::zero(), Vector::<4>::zero() == Vector([0.0; 4]));
    let mut r: RingBuf<u32, 3> = RingBuf::new();
    for i in 1..=5 {
        let ev = r.push(i * 11);
        println!("push {} evicted {:?} -> {:?}", i * 11, ev, r.items());
    }
    println!("{} {}", <[u8; 12] as Arr>::LEN, <[String; 0] as Arr>::LEN);
    let z = zip_arrays([1, 2, 3], ['a', 'b', 'c']);
    println!("{:?}", z);
    let squares: [u64; 8] = std::array::from_fn(|i| (i as u64).pow(2));
    println!("{:?}", squares);
    let mapped = squares.map(|x| x % 5);
    println!("{:?}", mapped);
    let nested = [[0u8; 3]; 2];
    println!("{:?} {}", nested, std::mem::size_of_val(&nested));
    let arr2: [String; 3] = std::array::from_fn(|i| "ab".repeat(i));
    println!("{:?}", arr2);
    let [x, y, z] = arr2;
    println!("{}|{}|{}", x, y, z);
    let tw: Result<[i32; 3], _> = vec![1, 2, 3].try_into();
    println!("{:?}", tw);
    let bad: Result<[i32; 2], Vec<i32>> = vec![1, 2, 3].try_into();
    println!("{:?}", bad);
    let sl: &[u8] = &[9, 8, 7, 6];
    let fixed: &[u8; 4] = sl.try_into().unwrap();
    println!("{:?}", fixed);
}
