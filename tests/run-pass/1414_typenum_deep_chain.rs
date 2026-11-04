#![recursion_limit = "512"]
use std::marker::PhantomData;
pub struct Z;
pub struct S<N>(PhantomData<N>);
pub trait Nat { const VAL: u64; }
impl Nat for Z { const VAL: u64 = 0; }
impl<N: Nat> Nat for S<N> { const VAL: u64 = N::VAL + 1; }
pub trait AddNat<B> { type Out: Nat; }
impl<B: Nat> AddNat<B> for Z { type Out = B; }
impl<A: Nat + AddNat<B>, B: Nat> AddNat<B> for S<A> { type Out = S<<A as AddNat<B>>::Out>; }
pub trait MulNat<B> { type Out: Nat; }
impl<B: Nat> MulNat<B> for Z { type Out = Z; }
impl<A, B> MulNat<B> for S<A> where A: MulNat<B>, B: Nat + AddNat<<A as MulNat<B>>::Out> {
    type Out = <B as AddNat<<A as MulNat<B>>::Out>>::Out;
}
type N2 = S<S<Z>>;
type N4 = <N2 as MulNat<N2>>::Out;
type N16 = <N4 as MulNat<N4>>::Out;
type N32 = <N16 as AddNat<N16>>::Out;
type N64 = <N32 as AddNat<N32>>::Out;
type N8 = <N4 as AddNat<N4>>::Out;
type N128 = <N16 as MulNat<N8>>::Out;

fn chain(v: Vec<u64>) -> u64 {
    v.into_iter()
        .map(|x| x + 1).map(|x| x * 2).filter(|x| x % 3 != 0).map(|x| x + 1).map(|x| x ^ 5)
        .map(|x| x + 1).map(|x| x * 2).filter(|x| x % 3 != 0).map(|x| x + 1).map(|x| x ^ 5)
        .map(|x| x + 1).map(|x| x * 2).filter(|x| x % 3 != 0).map(|x| x + 1).map(|x| x ^ 5)
        .map(|x| x + 1).map(|x| x * 2).filter(|x| x % 3 != 0).map(|x| x + 1).map(|x| x ^ 5)
        .map(|x| x + 1).map(|x| x * 2).filter(|x| x % 3 != 0).map(|x| x + 1).map(|x| x ^ 5)
        .map(|x| x + 1).map(|x| x * 2).filter(|x| x % 3 != 0).map(|x| x + 1).map(|x| x ^ 5)
        .enumerate().map(|(i, x)| x + i as u64).zip(0..50).map(|(a, b)| a * b)
        .chain(std::iter::once(7)).skip(1).take(100).fold(0, |a, b| a.wrapping_add(b))
}

fn wrap<T>(t: T) -> Option<Option<Option<Option<T>>>> { Some(Some(Some(Some(t)))) }

fn main() {
    println!("{} {} {}", N64::VAL, N128::VAL, <<N64 as MulNat<N2>>::Out as Nat>::VAL);
    println!("{}", chain((0..50).collect()));
    let w = wrap(wrap(wrap(1u8)));
    println!("{:?}", w);
    let t = ((((1, 2), (3, 4)), ((5, 6), (7, 8))), (((9, 10), (11, 12)), ((13, 14), (15, 16))));
    println!("{:?}", t);
    let f = |x: i32| move |y: i32| move |z: i32| x + y + z;
    let g = |h: &dyn Fn(i32) -> i32| h(1) + h(2);
    println!("{} {}", f(1)(2)(3), g(&|x| f(x)(x)(x)));
}
