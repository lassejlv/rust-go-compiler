use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Matrix<const R: usize, const C: usize> {
    d: [[i64; C]; R],
}

impl<const R: usize, const C: usize> Matrix<R, C> {
    const CELLS: usize = R * C;
    fn zero() -> Self {
        Matrix { d: [[0; C]; R] }
    }
    fn from_fn(f: impl Fn(usize, usize) -> i64) -> Self {
        let mut m = Self::zero();
        for r in 0..R {
            for c in 0..C {
                m.d[r][c] = f(r, c);
            }
        }
        m
    }
    fn transpose(&self) -> Matrix<C, R> {
        Matrix::<C, R>::from_fn(|r, c| self.d[c][r])
    }
    fn mul<const K: usize>(&self, o: &Matrix<C, K>) -> Matrix<R, K> {
        Matrix::from_fn(|r, k| (0..C).map(|i| self.d[r][i] * o.d[i][k]).sum())
    }
}

impl<const N: usize> Matrix<N, N> {
    fn identity() -> Self {
        Self::from_fn(|r, c| (r == c) as i64)
    }
    fn trace(&self) -> i64 {
        (0..N).map(|i| self.d[i][i]).sum()
    }
}

struct Buf<T, const N: usize> {
    items: [Option<T>; N],
    len: usize,
}

impl<T: Debug, const N: usize> Buf<T, N> {
    fn new() -> Self {
        Buf { items: std::array::from_fn(|_| None), len: 0 }
    }
    fn push(&mut self, t: T) -> Result<(), T> {
        if self.len == N {
            return Err(t);
        }
        self.items[self.len] = Some(t);
        self.len += 1;
        Ok(())
    }
    fn capacity(&self) -> usize {
        N
    }
}

fn first_n<const N: usize>(v: &[u8]) -> Option<[u8; N]> {
    v.get(..N)?.try_into().ok()
}

fn sum_arr<const N: usize>(a: [u32; N]) -> u32 {
    a.iter().sum()
}

fn concat<const A: usize, const B: usize>(a: [i32; A], b: [i32; B]) -> Vec<i32> {
    let mut v = Vec::with_capacity(A + B);
    v.extend_from_slice(&a);
    v.extend_from_slice(&b);
    v
}

trait HasSize {
    const SIZE: usize;
    const NAME: &'static str = "anon";
    fn describe() -> String {
        format!("{}:{}", Self::NAME, Self::SIZE)
    }
}

struct A;
struct B;
impl HasSize for A {
    const SIZE: usize = 4;
}
impl HasSize for B {
    const SIZE: usize = A::SIZE * 2 + 1;
    const NAME: &'static str = "bee";
}

fn total<X: HasSize, Y: HasSize>() -> usize {
    X::SIZE + Y::SIZE
}

const fn fib(n: u32) -> u64 {
    let mut a = 0;
    let mut b = 1;
    let mut i = 0;
    while i < n {
        let t = a + b;
        a = b;
        b = t;
        i += 1;
    }
    a
}

const FIB20: u64 = fib(20);
const TABLE: [u64; 8] = {
    let mut t = [0u64; 8];
    let mut i = 0;
    while i < 8 {
        t[i] = fib(i as u32 * 3);
        i += 1;
    }
    t
};
static GREETING: &str = "hi";
const MASKS: [u8; 4] = [1 << 0, 1 << 2, 1 << 4, 1 << 6];

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Id<const TAG: char>(u32);

fn main() {
    let m = Matrix::<2, 3>::from_fn(|r, c| (r * 3 + c) as i64);
    let t = m.transpose();
    println!("{:?}", t);
    let p = m.mul(&t);
    println!("{:?} trace={}", p, p.trace());
    println!("{} {}", Matrix::<2, 3>::CELLS, Matrix::<4, 4>::CELLS);
    let i3 = Matrix::<3, 3>::identity();
    println!("{}", t.mul(&Matrix::<2, 2>::identity()) == t);
    println!("{:?}", i3.mul(&t).d);

    let mut b: Buf<String, 2> = Buf::new();
    println!("{:?}", b.push("a".into()));
    println!("{:?}", b.push("b".into()));
    println!("{:?}", b.push("c".into()));
    println!("{} {} {:?}", b.capacity(), b.len, b.items);

    let data = [1u8, 2, 3, 4, 5];
    println!("{:?} {:?}", first_n::<3>(&data), first_n::<6>(&data));
    let x: Option<[u8; 2]> = first_n(&data[3..]);
    println!("{:?}", x);
    println!("{}", sum_arr([1, 2, 3]) + sum_arr([10; 4]));
    println!("{:?}", concat([1, 2], [3, 4, 5]));

    println!("{} {} {}", A::describe(), B::describe(), total::<A, B>());
    println!("{} {:?} {} {:?}", FIB20, TABLE, GREETING, MASKS);

    let arr: [usize; 5] = std::array::from_fn(|i| i * i);
    println!("{:?}", arr);
    let strs = arr.map(|x| format!("<{}>", x));
    println!("{:?}", strs);
    let [a0, .., a4] = arr;
    println!("{} {}", a0, a4);
    let arr2: [[u8; 2]; 3] = [[1, 2], [3, 4], [5, 6]];
    let flat: Vec<u8> = arr2.iter().flatten().copied().collect();
    println!("{:?} {:?}", flat, arr2.as_flattened());
    let z = [0u8; 0];
    println!("{:?} {}", z, z.is_empty());
    let id1: Id<'u'> = Id(5);
    let id2 = Id::<'u'>::default();
    println!("{:?} {:?} {}", id1, id2, id1 > id2);
    let tup_arr = [(1, 'a'); 3];
    println!("{:?}", tup_arr);
    let v: Vec<[i32; 2]> = vec![[1, 2], [3, 4]];
    let sumv: [i32; 2] = v.iter().fold([0, 0], |acc, x| [acc[0] + x[0], acc[1] + x[1]]);
    println!("{:?}", sumv);
    let r: Result<[i32; 3], _> = vec![1, 2, 3].try_into();
    println!("{:?}", r);
    let r: Result<[i32; 2], Vec<i32>> = vec![1, 2, 3].try_into();
    println!("{:?}", r);
    let arr_eq = [1, 2, 3] == [1, 2, 3][..];
    println!("{}", arr_eq);
    let mut sorted = [3, 1, 2];
    sorted.sort();
    println!("{:?} {}", sorted, sorted.len());
    let each: [String; 2] = ["a".to_string(), "b".to_string()];
    for s in each {
        print!("{} ", s);
    }
    println!();
    println!("{:?}", [1u8, 2].iter().rev().collect::<Vec<_>>());
    println!("{}", std::mem::size_of::<[u16; 7]>());
    println!("{}", std::mem::size_of::<Matrix<3, 2>>());
}
