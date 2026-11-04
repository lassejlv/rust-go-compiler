use std::collections::{BTreeSet, HashMap};
use std::fmt::Debug;

trait Graph {
    type Node: Clone + Ord;
    type Edges<'a>: Iterator<Item = Self::Node> + 'a where Self: 'a;
    fn nodes(&self) -> Vec<Self::Node>;
    fn edges<'a>(&'a self, n: &Self::Node) -> Self::Edges<'a>;
    fn describe(&self) -> String where Self::Node: Debug {
        self.nodes().iter().map(|n| format!("{:?}->{:?}", n, self.edges(n).collect::<Vec<_>>())).collect::<Vec<_>>().join("; ")
    }
    fn reachable(&self, start: Self::Node) -> BTreeSet<Self::Node> {
        let mut seen = BTreeSet::new();
        let mut stack = vec![start];
        while let Some(n) = stack.pop() {
            if seen.insert(n.clone()) { stack.extend(self.edges(&n)); }
        }
        seen
    }
}

struct AdjList { m: HashMap<char, Vec<char>> }
impl Graph for AdjList {
    type Node = char;
    type Edges<'a> = std::iter::Copied<std::slice::Iter<'a, char>>;
    fn nodes(&self) -> Vec<char> { let mut v: Vec<char> = self.m.keys().copied().collect(); v.sort(); v }
    fn edges<'a>(&'a self, n: &char) -> Self::Edges<'a> { self.m.get(n).map(|v| v.as_slice()).unwrap_or(&[]).iter().copied() }
}

struct Ring(u32);
impl Graph for Ring {
    type Node = u32;
    type Edges<'a> = std::iter::Once<u32>;
    fn nodes(&self) -> Vec<u32> { (0..self.0).collect() }
    fn edges<'a>(&'a self, n: &u32) -> std::iter::Once<u32> { std::iter::once((n + 1) % self.0) }
}

fn count_edges<G: Graph>(g: &G) -> usize { g.nodes().iter().map(|n| g.edges(n).count()).sum() }

fn into_iter_of<I: Iterator<Item = u8>>(i: I) -> <I as IntoIterator>::IntoIter { i.into_iter() }
fn pass_through<I>(i: I) -> Vec<<I as IntoIterator>::Item> where I: Iterator, I::Item: Clone {
    let v: <I as IntoIterator>::IntoIter = i;
    v.collect()
}
fn param_default<T: Default + Debug>() -> String {
    let v: <Vec<T> as IntoIterator>::Item = <Vec<T> as IntoIterator>::Item::default();
    format!("{:?}", v)
}

trait Shape { type Unit; fn area(&self) -> Self::Unit; }
trait Describe { fn describe(&self) -> String; }
impl<S: Shape> Describe for S where S::Unit: Debug { fn describe(&self) -> String { format!("area={:?}", self.area()) } }
struct Sq(u32);
impl Shape for Sq { type Unit = u64; fn area(&self) -> u64 { (self.0 * self.0) as u64 } }
fn describe_all<T: Describe>(xs: &[T]) -> Vec<String> { xs.iter().map(Describe::describe).collect() }
fn unit_of<S: Shape>(s: &S) -> <S as Shape>::Unit where <S as Shape>::Unit: Clone { let u = s.area(); u.clone() }

fn main() {
    let mut m = HashMap::new();
    m.insert('a', vec!['b', 'c']);
    m.insert('b', vec!['d']);
    m.insert('c', vec!['a']);
    m.insert('d', vec![]);
    let g = AdjList { m };
    println!("{}", g.describe());
    println!("{:?} {}", g.reachable('b'), count_edges(&g));
    let r = Ring(4);
    println!("{} | {:?} {}", r.describe(), r.reachable(2), count_edges(&r));
    println!("{:?}", into_iter_of(vec![1u8, 2].into_iter()).collect::<Vec<_>>());
    println!("{:?}", pass_through("ab".chars()));
    println!("{} {}", param_default::<u8>(), param_default::<String>());
    println!("{:?} {}", describe_all(&[Sq(2), Sq(3)]), unit_of(&Sq(5)));
}
