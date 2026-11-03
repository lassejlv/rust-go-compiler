use std::collections::HashMap;
use std::fmt::Debug;
use std::path::Path;
use std::rc::Rc;

trait Tr { fn m(&self) -> String; }
impl Tr for i32 { fn m(&self) -> String { format!("i32 {}", self) } }
impl Tr for str { fn m(&self) -> String { format!("str {}", self) } }
trait Dy { fn d(&self) -> u8; }
struct S1;
impl Dy for S1 { fn d(&self) -> u8 { 11 } }

fn rc<T: Clone + Debug>(x: Rc<T>) -> (Rc<T>, T) { (x.clone(), (*x).clone()) }
fn vecs<T: Debug>(x: &Vec<T>) -> usize { x.iter().count() + x.len() }
fn arr<T: Debug, const N: usize>(a: [T; N]) -> Vec<String> { a.iter().map(|x| format!("{:?}", x)).collect() }
fn dynm<T: ?Sized + Dy>(x: &T) -> u8 { x.d() }
fn unsz<T: ?Sized + Tr>(x: &T) -> String { x.m() }
fn asref<S: AsRef<str>>(s: S) -> usize { s.as_ref().len() }
fn asp<P: AsRef<Path>>(p: P) -> bool { p.as_ref().is_absolute() }
fn into2<T: Into<u64> + Copy>(x: T) -> u64 { x.into() + 1 }
fn mapit<K: std::hash::Hash + Eq + Debug, V: Debug>(m: &HashMap<K, V>, k: &K) -> String { format!("{:?}", m.get(k)) }
fn iters<I: Iterator<Item = u32>>(it: I) -> u32 { it.map(|x| x * 2).filter(|x| *x > 2).sum() }
fn by_mut<I: Iterator<Item = u8>>(it: &mut I) -> Option<u8> { it.next() }
fn opt<T: Clone>(o: &Option<T>) -> Option<T> { o.as_ref().cloned() }
fn strs<T: ToString>(v: &[T]) -> String { v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",") }
struct G<T> { v: Vec<T> }
impl<T: PartialOrd + Copy> G<T> {
    fn max(&self) -> Option<T> { let mut it = self.v.iter(); let mut m = *it.next()?; for x in it { if *x > m { m = *x; } } Some(m) }
    fn push(&mut self, x: T) { self.v.push(x); }
}
fn main() {
    println!("{:?}", rc(Rc::new(5)));
    println!("{}", vecs(&vec![1, 2]));
    println!("{:?}", arr([1u8, 2, 3]));
    println!("{}", dynm(&S1));
    let b: Box<dyn Dy> = Box::new(S1);
    println!("{}", dynm(&*b));
    println!("{} {}", unsz(&3), unsz("hi"));
    println!("{} {}", asref("abc"), asref(String::from("abcd")));
    println!("{}", asp("/x"));
    println!("{}", into2(3u32));
    let mut m = HashMap::new(); m.insert(1, "a");
    println!("{}", mapit(&m, &1));
    println!("{}", iters(vec![1u32, 2, 3].into_iter()));
    let mut it = vec![7u8].into_iter();
    println!("{:?}", by_mut(&mut it));
    println!("{:?}", opt(&Some(String::from("o"))));
    println!("{}", strs(&[1, 2]));
    let mut g = G { v: vec![1, 5, 2] }; g.push(9);
    println!("{:?}", g.max());
}
