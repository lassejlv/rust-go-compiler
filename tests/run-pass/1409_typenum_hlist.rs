use std::marker::PhantomData;
use std::ops::Add;

pub struct Z;
pub struct S<N>(PhantomData<N>);

pub trait Nat { const VAL: usize; fn name() -> String; }
impl Nat for Z { const VAL: usize = 0; fn name() -> String { "Z".into() } }
impl<N: Nat> Nat for S<N> { const VAL: usize = N::VAL + 1; fn name() -> String { format!("S({})", N::name()) } }

pub trait AddNat<B> { type Out: Nat; }
impl<B: Nat> AddNat<B> for Z { type Out = B; }
impl<A: Nat + AddNat<B>, B: Nat> AddNat<B> for S<A> { type Out = S<<A as AddNat<B>>::Out>; }

pub trait MulNat<B> { type Out: Nat; }
impl<B: Nat> MulNat<B> for Z { type Out = Z; }
impl<A, B> MulNat<B> for S<A>
where
    A: MulNat<B>,
    B: Nat + AddNat<<A as MulNat<B>>::Out>,
{
    type Out = <B as AddNat<<A as MulNat<B>>::Out>>::Out;
}

type N1 = S<Z>;
type N2 = S<N1>;
type N3 = S<N2>;
type N4 = <N2 as AddNat<N2>>::Out;
type N12 = <N3 as MulNat<N4>>::Out;
type Sum<A, B> = <A as AddNat<B>>::Out;
type Prod<A, B> = <A as MulNat<B>>::Out;

// type-level bool / compare
pub struct True;
pub struct False;
pub trait Bool { const B: bool; }
impl Bool for True { const B: bool = true; }
impl Bool for False { const B: bool = false; }
pub trait IsZero { type Out: Bool; }
impl IsZero for Z { type Out = True; }
impl<N> IsZero for S<N> { type Out = False; }

// heterogeneous list
pub struct HNil;
pub struct HCons<H, T>(H, T);
pub trait HList { type Len: Nat; fn describe(&self) -> String; }
impl HList for HNil { type Len = Z; fn describe(&self) -> String { "[]".into() } }
impl<H: std::fmt::Debug, T: HList> HList for HCons<H, T> {
    type Len = S<T::Len>;
    fn describe(&self) -> String { format!("{:?} :: {}", self.0, self.1.describe()) }
}
fn hlen<L: HList>(_: &L) -> usize { <L::Len as Nat>::VAL }

pub trait Append<R> { type Out; fn append(self, r: R) -> Self::Out; }
impl<R> Append<R> for HNil { type Out = R; fn append(self, r: R) -> R { r } }
impl<H, T: Append<R>, R> Append<R> for HCons<H, T> {
    type Out = HCons<H, T::Out>;
    fn append(self, r: R) -> Self::Out { HCons(self.0, self.1.append(r)) }
}

// sized vector indexed by type-level length
struct Vect<T, N: Nat> { v: Vec<T>, _n: PhantomData<N> }
impl<T> Vect<T, Z> { fn nil() -> Self { Vect { v: vec![], _n: PhantomData } } }
impl<T, N: Nat> Vect<T, N> {
    fn cons(mut self, x: T) -> Vect<T, S<N>> { self.v.push(x); Vect { v: self.v, _n: PhantomData } }
    fn len(&self) -> usize { N::VAL }
    fn concat<M: Nat>(mut self, o: Vect<T, M>) -> Vect<T, Sum<N, M>> where N: AddNat<M> {
        self.v.extend(o.v);
        Vect { v: self.v, _n: PhantomData }
    }
}
impl<T: Copy, N: Nat> Vect<T, S<N>> {
    fn head(&self) -> T { *self.v.last().unwrap() }
}
impl<T: Add<Output = T> + Copy, N: Nat> Add for Vect<T, N> {
    type Output = Vect<T, N>;
    fn add(self, o: Self) -> Self { Vect { v: self.v.iter().zip(&o.v).map(|(a, b)| *a + *b).collect(), _n: PhantomData } }
}

fn main() {
    println!("{} {} {}", N3::VAL, N4::VAL, N12::VAL);
    println!("{}", <Prod<N2, N3> as Nat>::name());
    println!("{}", <Sum<N12, Prod<N4, N4>> as Nat>::VAL);
    println!("{} {}", <<Z as IsZero>::Out as Bool>::B, <<N3 as IsZero>::Out as Bool>::B);
    let l = HCons(1u8, HCons("two", HCons(3.0f32, HNil)));
    println!("{} len={}", l.describe(), hlen(&l));
    let l2 = l.append(HCons('x', HCons(Some(5), HNil)));
    println!("{} len={}", l2.describe(), hlen(&l2));
    let a = Vect::nil().cons(1).cons(2);
    let b = Vect::nil().cons(10).cons(20).cons(30);
    println!("{} {}", a.len(), b.head());
    let c = a.concat(b);
    println!("{} {:?}", c.len(), c.v);
    let d = Vect::nil().cons(1).cons(1).cons(1).cons(1).cons(1);
    let e = c + d;
    println!("{} {:?} head={}", e.len(), e.v, e.head());
}
