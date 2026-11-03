trait Bounded {
    const MIN: Self;
    const MAX: Self;
    const NAME: &'static str;
    const BITS: u32 = 0;
}

impl Bounded for u8 {
    const MIN: u8 = 0;
    const MAX: u8 = 255;
    const NAME: &'static str = "u8";
    const BITS: u32 = 8;
}
impl Bounded for i16 {
    const MIN: i16 = -32768;
    const MAX: i16 = 32767;
    const NAME: &'static str = "i16";
    const BITS: u32 = 16;
}
impl Bounded for char {
    const MIN: char = '\0';
    const MAX: char = 'z';
    const NAME: &'static str = "char";
}

fn describe<T: Bounded + std::fmt::Debug>() -> String {
    format!("{}: [{:?}, {:?}] bits={}", T::NAME, T::MIN, T::MAX, T::BITS)
}

fn clamp_to<T: Bounded + PartialOrd + Copy>(x: T, lo: Option<T>) -> T {
    let lo = lo.unwrap_or(T::MIN);
    if x < lo { lo } else if x > T::MAX { T::MAX } else { x }
}

trait Shape {
    const SIDES: u32;
    const IS_POLY: bool = Self::SIDES >= 3;
    fn sides(&self) -> u32 { Self::SIDES }
}
struct Tri;
struct Circle;
struct Sq;
impl Shape for Tri { const SIDES: u32 = 3; }
impl Shape for Circle { const SIDES: u32 = 0; }
impl Shape for Sq { const SIDES: u32 = 4; const IS_POLY: bool = true; }

fn total_sides<A: Shape, B: Shape>() -> u32 { A::SIDES + B::SIDES }

struct Buf<T> { _t: std::marker::PhantomData<T> }
impl<T> Buf<T> {
    const ELEM: usize = std::mem::size_of::<T>();
    const DOUBLE: usize = Self::ELEM * 2;
}

trait Id { const ID: u32; }
struct A; struct B;
impl Id for A { const ID: u32 = 7; }
impl Id for B { const ID: u32 = 11; }
struct Both<X, Y>(X, Y);
impl<X: Id, Y: Id> Id for Both<X, Y> { const ID: u32 = X::ID * 100 + Y::ID; }

fn main() {
    println!("{}", describe::<u8>());
    println!("{}", describe::<i16>());
    println!("{}", describe::<char>());
    println!("{} {} {}", clamp_to(300i16, Some(5)), clamp_to(-5i16, Some(0)), clamp_to(17u8, None));
    println!("{} {}", <u8 as Bounded>::MAX, u8::MAX);
    println!("{} {} {}", Tri::IS_POLY, Circle::IS_POLY, Sq::IS_POLY);
    println!("{} {}", Tri.sides(), Sq.sides());
    println!("{}", total_sides::<Tri, Sq>());
    println!("{} {} {}", Buf::<u64>::ELEM, Buf::<u16>::DOUBLE, Buf::<(u8, u32)>::ELEM);
    println!("{} {}", Both::<A, B>::ID, <Both<B, Both<A, A>> as Id>::ID);
}
