// associated consts used in generic code, const defaults, consts in array lengths via const params
trait Shape {
    const SIDES: u32;
    const NAME: &'static str;
    const DOUBLE_SIDES: u32 = Self::SIDES * 2;
    fn describe() -> String {
        format!("{} has {} sides ({} doubled)", Self::NAME, Self::SIDES, Self::DOUBLE_SIDES)
    }
}
struct Triangle;
struct Square;
struct Hexagon;
impl Shape for Triangle {
    const SIDES: u32 = 3;
    const NAME: &'static str = "triangle";
}
impl Shape for Square {
    const SIDES: u32 = 4;
    const NAME: &'static str = "square";
    const DOUBLE_SIDES: u32 = 99;
}
impl Shape for Hexagon {
    const SIDES: u32 = 6;
    const NAME: &'static str = "hexagon";
}

fn total_sides<A: Shape, B: Shape>() -> u32 {
    A::SIDES + B::SIDES
}

trait Bounded {
    const MIN: Self;
    const MAX: Self;
    const BITS: u32;
}
macro_rules! bounded {
    ($($t:ty),*) => {$(
        impl Bounded for $t {
            const MIN: Self = <$t>::MIN;
            const MAX: Self = <$t>::MAX;
            const BITS: u32 = <$t>::BITS;
        }
    )*};
}
bounded!(u8, i8, u16, i32, u64, i128);

fn range_str<T: Bounded + std::fmt::Display>() -> String {
    format!("[{}, {}] bits={}", T::MIN, T::MAX, T::BITS)
}

struct Buffer<const N: usize> {
    data: [u8; N],
}
impl<const N: usize> Buffer<N> {
    const CAP: usize = N;
    const HALF: usize = N / 2;
    fn new() -> Self {
        Buffer { data: [0; N] }
    }
    fn fill_pattern(&mut self) {
        for i in 0..N {
            self.data[i] = (i * 7 % 256) as u8;
        }
    }
    fn halves(&self) -> (&[u8], &[u8]) {
        self.data.split_at(Self::HALF)
    }
}

trait HasId {
    const ID: u16;
}
struct A;
struct B;
impl HasId for A {
    const ID: u16 = 0x0A;
}
impl HasId for B {
    const ID: u16 = 0x0B;
}
struct Pair<X, Y>(X, Y);
impl<X: HasId, Y: HasId> HasId for Pair<X, Y> {
    const ID: u16 = (X::ID << 8) | Y::ID;
}

const fn fib(n: u32) -> u64 {
    let mut a = 0u64;
    let mut b = 1u64;
    let mut i = 0;
    while i < n {
        let t = a + b;
        a = b;
        b = t;
        i += 1;
    }
    a
}
const FIBS: [u64; 10] = {
    let mut arr = [0u64; 10];
    let mut i = 0;
    while i < 10 {
        arr[i] = fib(i as u32 * 3);
        i += 1;
    }
    arr
};

struct Matrix<const R: usize, const C: usize>([[i32; C]; R]);
impl<const R: usize, const C: usize> Matrix<R, C> {
    const CELLS: usize = R * C;
    fn transpose(&self) -> Matrix<C, R> {
        let mut out = [[0; R]; C];
        for r in 0..R {
            for c in 0..C {
                out[c][r] = self.0[r][c];
            }
        }
        Matrix(out)
    }
}

fn main() {
    println!("{}", Triangle::describe());
    println!("{}", Square::describe());
    println!("{}", Hexagon::describe());
    println!("{}", total_sides::<Triangle, Hexagon>());
    println!("{}", range_str::<u8>());
    println!("{}", range_str::<i8>());
    println!("{}", range_str::<u16>());
    println!("{}", range_str::<i32>());
    println!("{}", range_str::<u64>());
    println!("{}", range_str::<i128>());
    let mut b = Buffer::<10>::new();
    b.fill_pattern();
    let (l, r) = b.halves();
    println!("cap={} {:?} {:?}", Buffer::<10>::CAP, l, r);
    println!("{}", Buffer::<7>::HALF);
    println!("{:#06x}", Pair::<A, B>::ID);
    println!("{:#x}", Pair::<B, Pair<A, A>>::ID);
    println!("{:?}", FIBS);
    let m = Matrix([[1, 2, 3], [4, 5, 6]]);
    let t = m.transpose();
    println!("{:?} cells={} {}", t.0, Matrix::<2, 3>::CELLS, Matrix::<3, 2>::CELLS);
    const LEN: usize = Triangle::SIDES as usize + 1;
    let arr = [Square::SIDES; LEN];
    println!("{:?}", arr);
}
