use std::collections::HashMap;
use std::fmt::Debug;

trait Kind { type Repr: Default + Debug + Clone + From<u8>; const ID: u32; fn new() -> Self; }
trait Meta { const TAG: &'static str; }
#[derive(Debug, Default, Clone)]
struct Bytes(Vec<u8>);
impl From<u8> for Bytes { fn from(b: u8) -> Self { Bytes(vec![b]) } }
impl Meta for Bytes { const TAG: &'static str = "bytes"; }
impl Meta for u64 { const TAG: &'static str = "u64"; }
struct K1; struct K2;
impl Kind for K1 { type Repr = Bytes; const ID: u32 = 1; fn new() -> Self { K1 } }
impl Kind for K2 { type Repr = u64; const ID: u32 = 2; fn new() -> Self { K2 } }

trait Outer { type K: Kind; }
impl Outer for (K1, K2) { type K = K2; }

fn info<T: Kind>() -> String where T::Repr: Meta {
    let d = <T::Repr>::default();
    let d2 = <T as Kind>::Repr::from(7);
    let v: Vec<T::Repr> = (1..=2u8).map(T::Repr::from).collect();
    let v2: Vec<T::Repr> = (3..=4u8).map(<T::Repr as From<u8>>::from).collect();
    format!("{} {} {:?} {:?} {:?} {:?}", <T::Repr as Meta>::TAG, T::ID, d, d2, v, v2)
}
fn outer_info<O: Outer>() -> (u32, &'static str) where <O::K as Kind>::Repr: Meta {
    let _k = <O::K as Kind>::new();
    let _k2 = O::K::new();
    (<<O as Outer>::K as Kind>::ID, <<O::K as Kind>::Repr as Meta>::TAG)
}

struct Reg<T> { items: Vec<T> }
impl<T: Clone + Debug + Default> Reg<T> {
    fn helper(x: &T) -> String { format!("<{:?}>", x) }
    fn render(&self) -> Vec<String> { self.items.iter().map(Self::helper).collect() }
    fn render2(&self) -> Vec<String> { self.items.iter().map(Reg::<T>::helper).collect() }
    fn first_or_default(&self) -> T { self.items.first().cloned().unwrap_or_else(T::default) }
    fn make() -> Self { Self { items: vec![T::default(); 2] } }
}

enum Op { Add(i32), Neg }
impl Op {
    fn all() -> Vec<Self> { vec![Self::Add(1), Self::Neg, Op::Add(5)] }
    fn apply(&self, x: i32) -> i32 { match self { Self::Add(n) => x + n, Self::Neg => -x } }
}

type V = Vec<i32>;
type M<T> = HashMap<String, T>;
type O = Option<i32>;
type R<T> = Result<T, String>;

fn main() {
    println!("{}", info::<K1>());
    println!("{}", info::<K2>());
    println!("{:?}", outer_info::<(K1, K2)>());
    let r = Reg { items: vec![3u8, 4] };
    println!("{:?} {:?} {}", r.render(), r.render2(), r.first_or_default());
    println!("{:?}", Reg::<String>::make().render());
    println!("{}", Op::all().iter().fold(10, |a, o| o.apply(a)));
    let mut v = V::new();
    v.push(1);
    let mut m = M::<u8>::new();
    m.insert("a".into(), 1);
    let o = O::Some(3);
    let n = O::None;
    let r1: R<u8> = R::Ok(1);
    let r2 = R::<u8>::Err("e".into());
    println!("{:?} {:?} {:?} {:?} {:?} {:?}", v, m, o, n, r1, r2);
    println!("{:?} {:?} {:?}", Option::<i32>::None, None::<i32>, Result::<i32, String>::Ok(1));
    println!("{} {} {}", <[i32]>::len(&[1, 2]), <str>::len("abc"), <Vec<u8>>::with_capacity(4).capacity() >= 4);
    let s: String = Into::<String>::into("x");
    let s2 = <&str as Into<String>>::into("y");
    let n = <i64 as From<i32>>::from(5) + i64::from(3u8);
    println!("{} {} {}", s, s2, n);
    let parsed: Vec<u32> = ["1", "2"].iter().map(|s| s.parse()).collect::<Result<_, _>>().unwrap();
    let lens: Vec<usize> = ["ab", "c"].iter().copied().map(str::len).collect();
    let strs: Vec<String> = [1, 2].iter().map(ToString::to_string).collect();
    let up: Vec<String> = ["a"].iter().map(|s| s.to_uppercase()).collect();
    println!("{:?} {:?} {:?} {:?}", parsed, lens, strs, up);
    let somes: Vec<Option<i32>> = vec![1, 2].into_iter().map(Some).collect();
    let oks: Vec<Result<i32, ()>> = vec![1].into_iter().map(Ok).collect();
    let boxed: Vec<Box<str>> = vec!["q"].into_iter().map(Box::from).collect();
    println!("{:?} {:?} {:?}", somes, oks, boxed);
}
