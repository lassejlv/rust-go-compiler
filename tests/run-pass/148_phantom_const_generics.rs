use std::fmt::Debug;
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy)]
struct Meters;
#[derive(Debug, Clone, Copy)]
struct Feet;

#[derive(Debug, Clone, Copy)]
struct Length<U> {
    value: f64,
    _unit: PhantomData<U>,
}

impl<U> Length<U> {
    fn new(value: f64) -> Self {
        Length { value, _unit: PhantomData }
    }
    fn add(self, other: Length<U>) -> Length<U> {
        Length::new(self.value + other.value)
    }
}

impl Length<Feet> {
    fn to_meters(self) -> Length<Meters> {
        Length::new(self.value * 0.3048)
    }
}

struct Id<T> {
    raw: u32,
    _t: PhantomData<fn() -> T>,
}
impl<T> Id<T> {
    fn new(raw: u32) -> Self {
        Id { raw, _t: PhantomData }
    }
}

struct Typed<T> {
    items: Vec<String>,
    _marker: PhantomData<T>,
}
impl<T: Debug + Default> Typed<T> {
    fn push(&mut self, v: T) {
        self.items.push(format!("{:?}", v));
    }
    fn default_repr() -> String {
        format!("{:?}", T::default())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Debug)]
struct Image<const W: usize, const H: usize> {
    pixels: [[Pixel; W]; H],
}

impl<const W: usize, const H: usize> Default for Image<W, H> {
    fn default() -> Self {
        Image { pixels: [[Pixel::default(); W]; H] }
    }
}

impl<const W: usize, const H: usize> Image<W, H> {
    fn fill_diag(&mut self, p: Pixel) {
        for i in 0..W.min(H) {
            self.pixels[i][i] = p;
        }
    }
    fn count_nonblack(&self) -> usize {
        self.pixels.iter().flatten().filter(|p| **p != Pixel::default()).count()
    }
    const AREA: usize = W * H;
}

struct RingBuf<T, const N: usize> {
    data: [Option<T>; N],
    head: usize,
    len: usize,
}

impl<T: Copy + Debug, const N: usize> RingBuf<T, N> {
    fn new() -> Self {
        RingBuf { data: [None; N], head: 0, len: 0 }
    }
    fn push(&mut self, v: T) {
        let idx = (self.head + self.len) % N;
        self.data[idx] = Some(v);
        if self.len < N {
            self.len += 1;
        } else {
            self.head = (self.head + 1) % N;
        }
    }
    fn to_vec(&self) -> Vec<T> {
        (0..self.len).map(|i| self.data[(self.head + i) % N].unwrap()).collect()
    }
}

fn sum_array<const N: usize>(a: [i32; N]) -> i32 {
    a.iter().sum()
}

fn first_n<const N: usize>(v: &[u8]) -> [u8; N] {
    let mut out = [0u8; N];
    out.copy_from_slice(&v[..N]);
    out
}

fn main() {
    let a: Length<Feet> = Length::new(10.0);
    let b = Length::<Feet>::new(5.0);
    let m = a.add(b).to_meters();
    println!("{:.4} m", m.value);
    let user: Id<String> = Id::new(7);
    let order: Id<Vec<u8>> = Id::new(7);
    println!("ids {} {} size {}", user.raw, order.raw, std::mem::size_of::<Id<String>>());

    let mut t: Typed<(i32, bool)> = Typed { items: vec![], _marker: PhantomData };
    t.push((1, true));
    t.push((-2, false));
    println!("{:?} default={}", t.items, Typed::<(i32, bool)>::default_repr());

    let mut img: Image<4, 3> = Image::default();
    img.fill_diag(Pixel { r: 255, g: 0, b: 10 });
    println!("nonblack {} area {}", img.count_nonblack(), Image::<4, 3>::AREA);
    println!("{:?}", img.pixels[1][1]);
    println!("{:?}", img.pixels[2][3]);

    let mut rb: RingBuf<i32, 3> = RingBuf::new();
    for i in 1..=5 {
        rb.push(i * 10);
        println!("{:?}", rb.to_vec());
    }
    println!("{} {}", sum_array([1, 2, 3]), sum_array([10; 5]));
    let arr: [u8; 2] = first_n(&[9, 8, 7]);
    let arr4 = first_n::<4>(b"rustacean");
    println!("{:?} {:?}", arr, arr4);
    let grid: [[char; 3]; 2] = [['a'; 3], ['b'; 3]];
    println!("{:?}", grid);
}
