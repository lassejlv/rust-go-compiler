// Type-level computation: Peano numbers with Add/Mul via associated types,
// a type-level list with length/sum, and dimensioned quantities whose
// exponents are tracked in the type system (typenum-style integers).
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Add, Div, Mul};

struct Z;
struct S<N>(PhantomData<N>);

trait Nat { const VAL: u32; fn name() -> String; }
impl Nat for Z { const VAL: u32 = 0; fn name() -> String { "Z".into() } }
impl<N: Nat> Nat for S<N> { const VAL: u32 = N::VAL + 1; fn name() -> String { format!("S({})", N::name()) } }

trait PAdd<R> { type Out: Nat; }
impl<R: Nat> PAdd<R> for Z { type Out = R; }
impl<N: Nat + PAdd<R>, R: Nat> PAdd<R> for S<N> { type Out = S<<N as PAdd<R>>::Out>; }

trait PMul<R> { type Out: Nat; }
impl<R: Nat> PMul<R> for Z { type Out = Z; }
impl<N, R> PMul<R> for S<N> where N: PMul<R>, R: Nat + PAdd<<N as PMul<R>>::Out> {
    type Out = <R as PAdd<<N as PMul<R>>::Out>>::Out;
}

type N1 = S<Z>;
type N2 = S<N1>;
type N3 = S<N2>;
type Sum<A, B> = <A as PAdd<B>>::Out;
type Prod<A, B> = <A as PMul<B>>::Out;

fn show<N: Nat>() -> String { format!("{}={}", N::name(), N::VAL) }

// type-level heterogeneous list
struct HNil;
struct HCons<H, T>(H, T);
trait HList { const LEN: usize; fn describe(&self) -> Vec<String>; }
impl HList for HNil { const LEN: usize = 0; fn describe(&self) -> Vec<String> { vec![] } }
impl<H: fmt::Debug, T: HList> HList for HCons<H, T> {
    const LEN: usize = 1 + T::LEN;
    fn describe(&self) -> Vec<String> {
        let mut v = vec![format!("{:?}", self.0)];
        v.extend(self.1.describe());
        v
    }
}
macro_rules! hlist {
    () => { HNil };
    ($h:expr $(, $t:expr)*) => { HCons($h, hlist!($($t),*)) };
}

// signed type-level integers for dimensions: represented as (Pos, Neg) Peano pair normalized lazily
trait Int { const V: i32; }
struct I<P, N>(PhantomData<(P, N)>);
impl<P: Nat, N: Nat> Int for I<P, N> { const V: i32 = P::VAL as i32 - N::VAL as i32; }
trait IAdd<R> { type Out: Int; }
impl<P1: Nat + PAdd<P2>, N1: Nat + PAdd<N2>, P2: Nat, N2: Nat> IAdd<I<P2, N2>> for I<P1, N1> {
    type Out = I<Sum<P1, P2>, Sum<N1, N2>>;
}
trait INeg { type Out: Int; }
impl<P: Nat, N: Nat> INeg for I<P, N> { type Out = I<N, P>; }

struct Q<L, T> { v: f64, _d: PhantomData<(L, T)> }
impl<L: Int, T: Int> Q<L, T> { fn new(v: f64) -> Self { Q { v, _d: PhantomData } } }
impl<L: Int, T: Int> fmt::Display for Q<L, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.v)?;
        for (sym, e) in [("m", L::V), ("s", T::V)] {
            match e { 0 => {}, 1 => write!(f, " {}", sym)?, _ => write!(f, " {}^{}", sym, e)? }
        }
        Ok(())
    }
}
impl<L, T> Add for Q<L, T> { type Output = Q<L, T>; fn add(self, o: Self) -> Self { Q { v: self.v + o.v, _d: PhantomData } } }
impl<L1: IAdd<L2>, T1: IAdd<T2>, L2, T2> Mul<Q<L2, T2>> for Q<L1, T1> {
    type Output = Q<L1::Out, T1::Out>;
    fn mul(self, o: Q<L2, T2>) -> Self::Output { Q { v: self.v * o.v, _d: PhantomData } }
}
impl<L1, T1, L2: INeg, T2: INeg> Div<Q<L2, T2>> for Q<L1, T1> where L1: IAdd<L2::Out>, T1: IAdd<T2::Out> {
    type Output = Q<<L1 as IAdd<L2::Out>>::Out, <T1 as IAdd<T2::Out>>::Out>;
    fn div(self, o: Q<L2, T2>) -> Self::Output { Q { v: self.v / o.v, _d: PhantomData } }
}

type Zero = I<Z, Z>;
type One = I<N1, Z>;
type Meters = Q<One, Zero>;
type Seconds = Q<Zero, One>;

fn main() {
    println!("{}", show::<N3>());
    println!("2+3 = {}", show::<Sum<N2, N3>>());
    println!("2*3 = {}", <Prod<N2, N3> as Nat>::VAL);
    println!("3*3+1 = {}", <S<Prod<N3, N3>> as Nat>::VAL);
    println!("0*3 = {}  3*0 = {}", <Prod<Z, N3> as Nat>::VAL, <Prod<N3, Z> as Nat>::VAL);
    type N6 = Prod<N2, N3>;
    println!("6*6 = {}", <Prod<N6, N6> as Nat>::VAL);

    let l = hlist!(1u8, "two", 3.5f32, Some('c'), vec![5]);
    println!("hlist len {} items {:?}", <HCons<u8, HCons<&str, HCons<f32, HCons<Option<char>, HCons<Vec<i32>, HNil>>>>> as HList>::LEN, l.describe());
    fn len_of<L: HList>(_: &L) -> usize { L::LEN }
    println!("len_of {}", len_of(&hlist!((), ((), ()))));

    let d: Meters = Q::new(100.0);
    let t: Seconds = Q::new(9.58);
    let speed = Q::<One, Zero>::new(100.0) / t;
    println!("distance {} time {} speed {}", d, Q::<Zero, One>::new(9.58), speed);
    let area = Q::<One, Zero>::new(3.0) * Q::<One, Zero>::new(4.0);
    println!("area {}", area);
    let vol = area * Q::<One, Zero>::new(2.0);
    println!("volume {}", vol);
    let accel = Q::<One, Zero>::new(10.0) / (Q::<Zero, One>::new(2.0) * Q::<Zero, One>::new(5.0));
    println!("accel {}", accel);
    let ratio = Q::<One, Zero>::new(10.0) / Q::<One, Zero>::new(4.0);
    println!("dimensionless {}", ratio);
    let sum = Q::<One, Zero>::new(1.5) + d;
    println!("sum {}", sum);
    println!("exponents: {} {} {}", <I<N3, N1> as Int>::V, <I<Z, N2> as Int>::V, <<I<N1, Z> as IAdd<I<Z, N3>>>::Out as Int>::V);
}
