use std::fmt::{Debug, Display};
use std::any::Any;

trait Store { type Key: Default + Debug + Clone; type Val; const CAP: usize; fn make() -> Self; }
struct S1;
impl Store for S1 { type Key = String; type Val = Vec<u8>; const CAP: usize = 3; fn make() -> S1 { S1 } }

fn defaults<T: Store>() -> (T::Key, <T as Store>::Key, usize, usize)
where T::Val: Default + Debug {
    let a = <T as Store>::Key::default();
    let b = T::Key::default();
    let v = <T as Store>::Val::default();
    println!("{:?}", v);
    (a, b, <T as Store>::CAP, T::CAP)
}

trait Outer { type In: Store; }
struct O;
impl Outer for O { type In = S1; }
fn deep<T: Outer>() -> <<T as Outer>::In as Store>::Key {
    let _s = <T::In as Store>::make();
    let _s2 = <<T as Outer>::In as Store>::make();
    let k = <<T as Outer>::In as Store>::Key::default();
    println!("cap {}", <<T as Outer>::In as Store>::CAP);
    k
}

fn show_all(it: impl Iterator<Item: Display>) -> String {
    it.map(|x| x.to_string()).collect::<Vec<_>>().join("/")
}
fn nested_bound<I>(it: I) -> usize where I: IntoIterator<Item: IntoIterator<Item: Debug>> {
    it.into_iter().map(|inner| inner.into_iter().map(|x| format!("{:?}", x).len()).sum::<usize>()).sum()
}

trait A { fn a(&self) -> i32; }
trait B: A { fn b(&self) -> i32 { self.a() + 1 } }
trait C: B { fn c(&self) -> i32 { self.b() * 2 } }
struct X(i32);
impl A for X { fn a(&self) -> i32 { self.0 } }
impl B for X {}
impl C for X {}

trait Make { fn make() -> Self where Self: Sized; fn name(&self) -> String; fn gen<T: Debug>(&self, t: T) -> String where Self: Sized { format!("{}:{:?}", self.name(), t) } }
impl Make for X { fn make() -> X { X(0) } fn name(&self) -> String { format!("X{}", self.0) } }

trait Pretty { fn pretty(&self) -> String; }
impl<T: Debug> Pretty for T { fn pretty(&self) -> String { format!("~{:?}~", self) } }

struct W<T = i32, U = String> { t: T, u: U }
impl W { fn mk() -> Self { W { t: 1, u: "d".into() } } }

#[derive(Debug, PartialEq)]
struct Cents(i64);
impl PartialEq<i64> for Cents { fn eq(&self, o: &i64) -> bool { self.0 == *o } }
impl PartialEq<Cents> for i64 { fn eq(&self, o: &Cents) -> bool { *self == o.0 } }
impl PartialOrd<i64> for Cents { fn partial_cmp(&self, o: &i64) -> Option<std::cmp::Ordering> { self.0.partial_cmp(o) } }

fn main() {
    println!("{:?}", defaults::<S1>());
    println!("{:?}", deep::<O>());
    println!("{}", show_all([1.5, 2.0].iter()));
    println!("{}", nested_bound(vec![vec!["a", "bb"], vec![]]));
    let x = X(3);
    let c: &dyn C = &x;
    let b: &dyn B = c;
    let a: &dyn A = b;
    let a2: &dyn A = c;
    println!("{} {} {} {}", c.c(), b.b(), a.a(), a2.a());
    let bx: Box<dyn C> = Box::new(X(10));
    let ba: Box<dyn A> = bx;
    println!("{}", ba.a());
    let m: &dyn Make = &x;
    println!("{} {}", m.name(), x.gen("q"));
    let made = X::make();
    println!("{}", made.name());
    let p: &dyn Pretty = &5u8;
    let ps: Vec<Box<dyn Pretty>> = vec![Box::new("s"), Box::new(vec![1]), Box::new(p.pretty())];
    for q in &ps { println!("{}", q.pretty()); }
    let w = W::mk();
    let w2: W<u8, bool> = W { t: 2, u: true };
    println!("{} {} {} {}", w.t, w.u, w2.t, w2.u);
    let c = Cents(5);
    println!("{} {} {} {}", c == 5, 5 == c, c > 3, c == Cents(5));
    let anyv: Vec<Box<dyn Any>> = vec![Box::new(X(1)), Box::new(2)];
    println!("{}", anyv.iter().filter(|a| a.is::<X>()).count());
}
