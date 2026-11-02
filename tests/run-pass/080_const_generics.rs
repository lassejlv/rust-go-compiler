#[derive(Debug)]
struct Buffer<const N: usize> {
    data: [u8; N],
    len: usize,
}

impl<const N: usize> Buffer<N> {
    fn new() -> Self {
        Buffer { data: [0; N], len: 0 }
    }
    fn push(&mut self, b: u8) -> bool {
        if self.len == N {
            return false;
        }
        self.data[self.len] = b;
        self.len += 1;
        true
    }
    fn capacity(&self) -> usize {
        N
    }
    fn contents(&self) -> &[u8] {
        &self.data[..self.len]
    }
}

fn sum_array<const N: usize>(arr: [i32; N]) -> i32 {
    let mut s = 0;
    for x in arr {
        s += x;
    }
    s
}

fn first_last<T: Copy, const N: usize>(arr: &[T; N]) -> (T, T) {
    (arr[0], arr[N - 1])
}

fn dot<const N: usize>(a: [f64; N], b: [f64; N]) -> f64 {
    let mut s = 0.0;
    for i in 0..N {
        s += a[i] * b[i];
    }
    s
}

#[derive(Debug, Clone, Copy)]
struct Mat<const R: usize, const C: usize> {
    m: [[i32; C]; R],
}

impl<const R: usize, const C: usize> Mat<R, C> {
    fn transpose(&self) -> Mat<C, R> {
        let mut t = [[0; R]; C];
        for i in 0..R {
            for j in 0..C {
                t[j][i] = self.m[i][j];
            }
        }
        Mat { m: t }
    }
    fn mul<const K: usize>(&self, other: &Mat<C, K>) -> Mat<R, K> {
        let mut out = [[0; K]; R];
        for i in 0..R {
            for j in 0..K {
                for k in 0..C {
                    out[i][j] += self.m[i][k] * other.m[k][j];
                }
            }
        }
        Mat { m: out }
    }
}

fn main() {
    let mut b: Buffer<4> = Buffer::new();
    for x in 10..16 {
        let ok = b.push(x);
        println!("push {} -> {}", x, ok);
    }
    println!("capacity {} contents {:?}", b.capacity(), b.contents());
    println!("sum3 = {}", sum_array([1, 2, 3]));
    println!("sum5 = {}", sum_array([10, 20, 30, 40, 50]));
    println!("first_last = {:?}", first_last(&['a', 'b', 'c', 'd']));
    println!("dot = {}", dot([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]));
    let a = Mat { m: [[1, 2, 3], [4, 5, 6]] };
    let t = a.transpose();
    println!("a = {:?}", a.m);
    println!("t = {:?}", t.m);
    let p = a.mul(&t);
    println!("a * t = {:?}", p.m);
    let q = t.mul(&a);
    println!("t * a = {:?}", q.m);
    let arr: [u8; 3] = core::array::from_fn(|i| (i * i) as u8);
    println!("from_fn = {:?}", arr);
}
