trait Shape {
    const SIDES: u32;
    fn new(size: f64) -> Self;
    fn name() -> &'static str;
    fn area(&self) -> f64;
}

struct Square {
    s: f64,
}
struct Triangle {
    s: f64,
}

impl Shape for Square {
    const SIDES: u32 = 4;
    fn new(size: f64) -> Self {
        Square { s: size }
    }
    fn name() -> &'static str {
        "square"
    }
    fn area(&self) -> f64 {
        self.s * self.s
    }
}

impl Shape for Triangle {
    const SIDES: u32 = 3;
    fn new(size: f64) -> Self {
        Triangle { s: size }
    }
    fn name() -> &'static str {
        "triangle"
    }
    fn area(&self) -> f64 {
        self.s * self.s * 3f64.sqrt() / 4.0
    }
}

fn build_and_report<T: Shape>(size: f64) -> f64 {
    let shape = T::new(size);
    let a = shape.area();
    println!("{} with {} sides, size {}: area {:.3}", T::name(), T::SIDES, size, a);
    a
}

trait Zero {
    fn zero() -> Self;
    fn is_zero(&self) -> bool;
}

impl Zero for i32 {
    fn zero() -> Self {
        0
    }
    fn is_zero(&self) -> bool {
        *self == 0
    }
}

impl Zero for f64 {
    fn zero() -> Self {
        0.0
    }
    fn is_zero(&self) -> bool {
        *self == 0.0
    }
}

fn count_zeros<T: Zero>(v: &[T]) -> usize {
    v.iter().filter(|x| x.is_zero()).count()
}

fn make_zeros<T: Zero + Clone>(n: usize) -> Vec<T> {
    vec![T::zero(); n]
}

fn main() {
    let a = build_and_report::<Square>(2.0);
    let b = build_and_report::<Triangle>(2.0);
    println!("total = {:.3}", a + b);
    println!("sides: {} {}", Square::SIDES, <Triangle as Shape>::SIDES);
    println!("zeros in ints: {}", count_zeros(&[0, 1, 0, 2]));
    println!("zeros in floats: {}", count_zeros(&[0.0, 0.5]));
    let z: Vec<i32> = make_zeros(3);
    let zf = make_zeros::<f64>(2);
    println!("{:?} {:?}", z, zf);
    println!("i32::zero() = {}", i32::zero());
    println!("from_str: {:?}", <i32 as std::str::FromStr>::from_str("17"));
}
