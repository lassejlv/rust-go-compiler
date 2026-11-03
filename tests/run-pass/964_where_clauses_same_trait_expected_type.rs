use std::ops::{Add, Mul};

fn widen<T: Copy>(t: T) -> (u32, u64)
where
    T: Into<u32> + Into<u64>,
{
    let x: u64 = t.into();
    let y: u32 = t.into();
    (y, x)
}

fn widen2<T>(t: T, u: T) -> u64
where
    T: Into<u32> + Into<u64>,
{
    let a: u32 = u.into();
    let b: u64 = t.into();
    b + a as u64
}

fn pass_u64(v: u64) -> u64 {
    v * 2
}

fn arg_expected<T: Copy + Into<u32> + Into<u64>>(t: T) -> u64 {
    pass_u64(t.into())
}

#[derive(Clone, Copy, Debug)]
struct V(f64, f64);
#[derive(Clone, Copy, Debug)]
struct S(f64);

impl Mul<V> for S {
    type Output = V;
    fn mul(self, v: V) -> V {
        V(self.0 * v.0, self.0 * v.1)
    }
}
impl Mul<S> for S {
    type Output = S;
    fn mul(self, o: S) -> S {
        S(self.0 * o.0)
    }
}
impl Add for V {
    type Output = V;
    fn add(self, o: V) -> V {
        V(self.0 + o.0, self.1 + o.1)
    }
}

fn scale<X, A, VV>(x: X, a: A, v: VV) -> (<X as Mul<A>>::Output, <X as Mul<VV>>::Output)
where
    X: Mul<A> + Mul<VV> + Copy,
{
    (x * a, x * v)
}

fn scale_m<X, A, VV>(x: X, a: A, v: VV) -> (<X as Mul<A>>::Output, <X as Mul<VV>>::Output)
where
    X: Mul<A> + Mul<VV> + Copy,
{
    (x.mul(a), x.mul(v))
}

fn scale_v<X: Copy>(x: X, s: S, v: V) -> V
where
    X: Mul<S, Output = S> + Mul<V, Output = V>,
{
    let a: S = x * s;
    let b: V = x * v;
    V(b.0 + a.0, b.1)
}

fn from_both<T>(a: u8) -> (T, T)
where
    T: From<u8> + From<u16>,
{
    let x: T = T::from(a);
    let y: T = T::from(300u16);
    (x, y)
}

fn main() {
    println!("{:?}", widen(7u8));
    println!("{}", widen2(3u8, 4u8));
    println!("{}", arg_expected(5u16));
    println!("{:?}", scale_v(S(2.0), S(3.0), V(1.0, 2.0)));
    let (p, q) = scale(S(2.0), S(5.0), V(1.0, 1.5));
    println!("{:?} {:?}", p, q);
    let (p, q) = scale_m(S(2.0), S(5.0), V(1.0, 1.5));
    println!("{:?} {:?}", p, q);
    println!("{:?}", from_both::<u32>(9));
}
