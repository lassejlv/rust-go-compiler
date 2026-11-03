use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Vector<const N: usize> {
    v: [f64; N],
}

impl<const N: usize> Vector<N> {
    fn zero() -> Self { Vector { v: [0.0; N] } }
    fn from_fn(f: impl Fn(usize) -> f64) -> Self {
        let mut v = [0.0; N];
        for i in 0..N { v[i] = f(i); }
        Vector { v }
    }
    fn dot(&self, o: &Self) -> f64 { (0..N).map(|i| self.v[i] * o.v[i]).sum() }
    fn dim(&self) -> usize { N }
    fn add(&self, o: &Self) -> Self {
        let mut r = *self;
        for i in 0..N { r.v[i] += o.v[i]; }
        r
    }
}

impl Vector<3> {
    fn cross(&self, o: &Self) -> Self {
        let [a1, a2, a3] = self.v;
        let [b1, b2, b3] = o.v;
        Vector { v: [a2 * b3 - a3 * b2, a3 * b1 - a1 * b3, a1 * b2 - a2 * b1] }
    }
}
impl Vector<2> {
    fn perp(&self) -> Self { Vector { v: [-self.v[1], self.v[0]] } }
}

struct RingBuf<T, const CAP: usize> {
    data: [Option<T>; CAP],
    head: usize,
    len: usize,
}

impl<T: Copy + Debug, const CAP: usize> RingBuf<T, CAP> {
    fn new() -> Self { RingBuf { data: [None; CAP], head: 0, len: 0 } }
    fn push(&mut self, x: T) -> Option<T> {
        let idx = (self.head + self.len) % CAP;
        if self.len == CAP {
            let old = self.data[self.head].take();
            self.data[self.head] = Some(x);
            self.head = (self.head + 1) % CAP;
            old
        } else {
            self.data[idx] = Some(x);
            self.len += 1;
            None
        }
    }
    fn to_vec(&self) -> Vec<T> {
        (0..self.len).map(|i| self.data[(self.head + i) % CAP].unwrap()).collect()
    }
    const CAPACITY: usize = CAP;
}

fn sum_arr<const N: usize>(a: [i32; N]) -> i32 { a.iter().sum() }
fn first_last<T: Copy, const N: usize>(a: &[T; N]) -> Option<(T, T)> {
    if N == 0 { None } else { Some((a[0], a[N - 1])) }
}
fn chunks<const K: usize>(v: &[u8]) -> Vec<[u8; K]> {
    v.chunks_exact(K).map(|c| { let mut a = [0u8; K]; a.copy_from_slice(c); a }).collect()
}

fn main() {
    let a = Vector::<3>::from_fn(|i| i as f64 + 1.0);
    let b = Vector { v: [0.0, 1.0, 0.0] };
    println!("{:?} dot={} dim={}", a, a.dot(&b), a.dim());
    println!("{:?}", a.cross(&b));
    println!("{:?}", a.add(&b).add(&Vector::zero()));
    let p = Vector { v: [3.0, 4.0] };
    println!("{:?} {}", p.perp(), p.dot(&p).sqrt());
    let z: Vector<5> = Vector::zero();
    println!("{} {:?}", z.dim(), z.v);
    let mut r: RingBuf<i32, 3> = RingBuf::new();
    for i in 1..=5 {
        let ev = r.push(i * 10);
        println!("push {} evicted {:?} -> {:?}", i * 10, ev, r.to_vec());
    }
    println!("cap {}", RingBuf::<char, 7>::CAPACITY);
    println!("{} {}", sum_arr([1, 2, 3]), sum_arr([10; 8]));
    println!("{:?} {:?}", first_last(&[1, 2, 3, 4]), first_last::<u8, 0>(&[]));
    println!("{:?}", chunks::<2>(&[1, 2, 3, 4, 5]));
    println!("{:?}", chunks::<3>(&[9, 8, 7, 6, 5, 4, 3]));
}
