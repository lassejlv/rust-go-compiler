use std::fmt::Debug;
use std::marker::PhantomData;

// HList
#[derive(Debug, Clone, Copy, PartialEq)] struct HNil;
#[derive(Debug, Clone, Copy, PartialEq)] struct HCons<H, T> { head: H, tail: T }
trait HList { const LEN: usize; fn describe(&self) -> String; }
impl HList for HNil { const LEN: usize = 0; fn describe(&self) -> String { "nil".into() } }
impl<H: Debug, T: HList> HList for HCons<H, T> { const LEN: usize = 1 + T::LEN; fn describe(&self) -> String { format!("{:?} :: {}", self.head, self.tail.describe()) } }
fn hcons<H, T: HList>(head: H, tail: T) -> HCons<H, T> { HCons { head, tail } }

trait SumAll { fn sum_all(&self) -> f64; }
impl SumAll for HNil { fn sum_all(&self) -> f64 { 0.0 } }
impl<H: Copy + Into<f64>, T: SumAll> SumAll for HCons<H, T> { fn sum_all(&self) -> f64 { self.head.into() + self.tail.sum_all() } }

trait Reverse<Acc> { type Out; fn rev_onto(self, acc: Acc) -> Self::Out; }
impl<Acc> Reverse<Acc> for HNil { type Out = Acc; fn rev_onto(self, acc: Acc) -> Acc { acc } }
impl<Acc, H, T: Reverse<HCons<H, Acc>>> Reverse<Acc> for HCons<H, T> { type Out = T::Out; fn rev_onto(self, acc: Acc) -> T::Out { self.tail.rev_onto(HCons { head: self.head, tail: acc }) } }

// Peano
struct Z; struct S<N>(PhantomData<N>);
trait Nat { const N: u64; }
impl Nat for Z { const N: u64 = 0; }
impl<N: Nat> Nat for S<N> { const N: u64 = N::N + 1; }
trait Add<B> { type Sum: Nat; }
impl<B: Nat> Add<B> for Z { type Sum = B; }
impl<A: Nat + Add<B>, B: Nat> Add<B> for S<A> { type Sum = S<<A as Add<B>>::Sum>; }
trait Mul<B> { type Prod: Nat; }
impl<B: Nat> Mul<B> for Z { type Prod = Z; }
impl<A: Mul<B>, B: Nat> Mul<B> for S<A> where B: Add<<A as Mul<B>>::Prod> { type Prod = <B as Add<<A as Mul<B>>::Prod>>::Sum; }
type Two = S<S<Z>>; type Three = S<Two>; type Five = <Two as Add<Three>>::Sum;
type Fifteen = <Three as Mul<Five>>::Prod;

// Visitor double dispatch
trait Visitor { type Out; fn num(&mut self, n: i64) -> Self::Out; fn add(&mut self, a: &Node, b: &Node) -> Self::Out; fn neg(&mut self, a: &Node) -> Self::Out; fn var(&mut self, name: &str) -> Self::Out; }
enum Node { Num(i64), Add(Box<Node>, Box<Node>), Neg(Box<Node>), Var(String) }
impl Node {
    fn accept<V: Visitor>(&self, v: &mut V) -> V::Out {
        match self { Node::Num(n) => v.num(*n), Node::Add(a, b) => v.add(a, b), Node::Neg(a) => v.neg(a), Node::Var(s) => v.var(s) }
    }
}
struct Eval<'a> { env: &'a [(&'a str, i64)], lookups: usize }
impl<'a> Visitor for Eval<'a> {
    type Out = Option<i64>;
    fn num(&mut self, n: i64) -> Option<i64> { Some(n) }
    fn add(&mut self, a: &Node, b: &Node) -> Option<i64> { Some(a.accept(self)? + b.accept(self)?) }
    fn neg(&mut self, a: &Node) -> Option<i64> { a.accept(self).map(|x| -x) }
    fn var(&mut self, name: &str) -> Option<i64> { self.lookups += 1; self.env.iter().find(|(n, _)| *n == name).map(|p| p.1) }
}
struct Printer;
impl Visitor for Printer {
    type Out = String;
    fn num(&mut self, n: i64) -> String { n.to_string() }
    fn add(&mut self, a: &Node, b: &Node) -> String { format!("({} + {})", a.accept(self), b.accept(self)) }
    fn neg(&mut self, a: &Node) -> String { format!("-{}", a.accept(self)) }
    fn var(&mut self, name: &str) -> String { name.to_uppercase() }
}
struct Depth;
impl Visitor for Depth {
    type Out = usize;
    fn num(&mut self, _: i64) -> usize { 1 }
    fn add(&mut self, a: &Node, b: &Node) -> usize { 1 + a.accept(self).max(b.accept(self)) }
    fn neg(&mut self, a: &Node) -> usize { 1 + a.accept(self) }
    fn var(&mut self, _: &str) -> usize { 1 }
}

fn main() {
    let l = hcons(1u8, hcons(2.5f32, hcons(-3i32, hcons(4u16, HNil))));
    println!("{} len={}", l.describe(), <HCons<u8, HCons<f32, HCons<i32, HCons<u16, HNil>>>> as HList>::LEN);
    println!("sum={}", l.sum_all());
    let r = l.rev_onto(HNil);
    println!("{}", r.describe());
    let mixed = hcons("str", hcons(Some(1), hcons(vec!['a'], HNil)));
    println!("{} {}", mixed.describe(), mixed.rev_onto(HNil).describe());
    println!("{} {} {} {}", Two::N, Five::N, Fifteen::N, <<Fifteen as Add<Fifteen>>::Sum as Nat>::N);
    println!("{}", <<Five as Mul<Five>>::Prod as Nat>::N);
    let tree = { use Node::*; Add(Box::new(Num(5)), Box::new(Neg(Box::new(Add(Box::new(Var("x".into())), Box::new(Neg(Box::new(Var("y".into()))))))))) };
    let env = [("x", 10), ("y", 3)];
    let mut ev = Eval { env: &env, lookups: 0 };
    println!("{} = {:?} (lookups {}) depth {}", tree.accept(&mut Printer), tree.accept(&mut ev), ev.lookups, tree.accept(&mut Depth));
    let mut ev2 = Eval { env: &env[..1], lookups: 0 };
    println!("{:?} lookups {}", tree.accept(&mut ev2), ev2.lookups);
}
