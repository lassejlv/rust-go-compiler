use std::marker::PhantomData;

pub struct Z;
pub struct S<N>(PhantomData<N>);
pub trait Nat { const VAL: u64; }
impl Nat for Z { const VAL: u64 = 0; }
impl<N: Nat> Nat for S<N> { const VAL: u64 = N::VAL + 1; }

pub trait Add<B> { type Out: Nat; }
impl<B: Nat> Add<B> for Z { type Out = B; }
impl<A: Add<B>, B: Nat> Add<B> for S<A> { type Out = S<A::Out>; }

pub trait Mul<B> { type Out: Nat; }
impl<B: Nat> Mul<B> for Z { type Out = Z; }
impl<A: Mul<B>, B: Nat + Add<A::Out>> Mul<B> for S<A> { type Out = <B as Add<A::Out>>::Out; }

pub trait Fact { type Out: Nat; }
impl Fact for Z { type Out = S<Z>; }
impl<N> Fact for S<N> where N: Fact, S<N>: Mul<N::Out> { type Out = <S<N> as Mul<N::Out>>::Out; }

pub trait Fib { type Out: Nat; }
impl Fib for Z { type Out = Z; }
impl Fib for S<Z> { type Out = S<Z>; }
impl<N> Fib for S<S<N>> where N: Fib, S<N>: Fib, <S<N> as Fib>::Out: Add<<N as Fib>::Out> {
    type Out = <<S<N> as Fib>::Out as Add<<N as Fib>::Out>>::Out;
}

pub trait Cmp<B> { const ORD: i8; }
impl Cmp<Z> for Z { const ORD: i8 = 0; }
impl<N> Cmp<S<N>> for Z { const ORD: i8 = -1; }
impl<N> Cmp<Z> for S<N> { const ORD: i8 = 1; }
impl<A: Cmp<B>, B> Cmp<S<B>> for S<A> { const ORD: i8 = A::ORD; }

pub trait Pred { type Out; }
impl<N> Pred for S<N> { type Out = N; }

type N1 = S<Z>;
type N2 = S<N1>;
type N3 = S<N2>;
type N4 = S<N3>;
type N5 = S<N4>;
type N6 = S<N5>;
type N8 = <N4 as Add<N4>>::Out;
type N10 = <N5 as Add<N5>>::Out;

fn val<N: Nat>() -> u64 { N::VAL }
fn cmp<A: Cmp<B>, B>() -> i8 { A::ORD }

fn main() {
    println!("{} {} {}", val::<<N3 as Fact>::Out>(), val::<<N4 as Fact>::Out>(), val::<<N5 as Fact>::Out>());
    println!("{} {} {}", val::<<N6 as Fib>::Out>(), val::<<N8 as Fib>::Out>(), val::<<N10 as Fib>::Out>());
    println!("{} {} {}", cmp::<N3, N5>(), cmp::<N8, <N4 as Mul<N2>>::Out>(), cmp::<N10, N2>());
    println!("{}", val::<<<N10 as Pred>::Out as Pred>::Out>());
}
