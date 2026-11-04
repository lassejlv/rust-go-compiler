use std::fmt::Debug;
use std::collections::BTreeMap;

fn first_item<I>(i: I) -> Option<<<I as IntoIterator>::IntoIter as Iterator>::Item>
where
    I: IntoIterator,
{
    i.into_iter().next()
}

fn collect_pairs<I>(i: I) -> Vec<(usize, <I::IntoIter as Iterator>::Item)>
where
    I: IntoIterator,
    <I as IntoIterator>::Item: Clone,
{
    i.into_iter().enumerate().collect()
}

trait Shape {
    const SIDES: u32;
    const NAME: &'static str;
    fn area(&self) -> f64;
    fn describe(&self) -> String {
        format!("{} with {} sides, area {:.2}", Self::NAME, Self::SIDES, self.area())
    }
}

struct Sq(f64);
struct Tri(f64, f64);
impl Shape for Sq {
    const SIDES: u32 = 4;
    const NAME: &'static str = "square";
    fn area(&self) -> f64 { self.0 * self.0 }
}
impl Shape for Tri {
    const SIDES: u32 = 3;
    const NAME: &'static str = "triangle";
    fn area(&self) -> f64 { 0.5 * self.0 * self.1 }
}

fn total_sides<A: Shape, B: Shape>() -> u32 {
    A::SIDES + B::SIDES
}

struct SidesArr<S: Shape> {
    _s: std::marker::PhantomData<S>,
}
impl<S: Shape> SidesArr<S> {
    const DOUBLE: u32 = S::SIDES * 2;
    fn get() -> u32 { Self::DOUBLE }
}

trait Container {
    type Item;
    type Iter<'a>: Iterator<Item = &'a Self::Item> where Self: 'a;
    fn items<'a>(&'a self) -> Self::Iter<'a>;
    fn first(&self) -> Option<&Self::Item> {
        self.items().next()
    }
}

struct Stack<T> { v: Vec<T> }
impl<T> Container for Stack<T> {
    type Item = T;
    type Iter<'a> = std::iter::Rev<std::slice::Iter<'a, T>> where T: 'a;
    fn items<'a>(&'a self) -> Self::Iter<'a> { self.v.iter().rev() }
}

fn show_container<C>(c: &C) -> String
where
    C: Container,
    C::Item: Debug,
{
    c.items().map(|x| format!("{:?}", x)).collect::<Vec<_>>().join(",")
}

trait Graph {
    type Node: Ord + Copy + Debug;
    type Edges: IntoIterator<Item = Self::Node>;
    fn neighbors(&self, n: Self::Node) -> Self::Edges;
}

struct Adj(BTreeMap<u32, Vec<u32>>);
impl Graph for Adj {
    type Node = u32;
    type Edges = Vec<u32>;
    fn neighbors(&self, n: u32) -> Vec<u32> {
        self.0.get(&n).cloned().unwrap_or_default()
    }
}

fn bfs<G: Graph>(g: &G, start: G::Node) -> Vec<G::Node>
where
    <G::Edges as IntoIterator>::IntoIter: Clone,
{
    let mut seen = std::collections::BTreeSet::new();
    let mut q = std::collections::VecDeque::new();
    let mut out = vec![];
    q.push_back(start);
    seen.insert(start);
    while let Some(n) = q.pop_front() {
        out.push(n);
        for m in g.neighbors(n) {
            if seen.insert(m) {
                q.push_back(m);
            }
        }
    }
    out
}

trait Parser {
    type Output;
    type Error: Debug + Default;
    fn parse(&self, s: &str) -> Result<Self::Output, Self::Error>;
}
struct IntP;
impl Parser for IntP {
    type Output = i64;
    type Error = String;
    fn parse(&self, s: &str) -> Result<i64, String> {
        s.trim().parse().map_err(|e: std::num::ParseIntError| e.to_string())
    }
}
fn parse_all<P: Parser>(p: &P, xs: &[&str]) -> Result<Vec<P::Output>, P::Error>
where
    P::Output: Clone,
{
    xs.iter().map(|x| p.parse(x)).collect()
}

fn main() {
    println!("{:?}", first_item(vec![3, 4]));
    println!("{:?}", first_item("ab".chars()));
    println!("{:?}", first_item(&[1u8, 2][..]));
    println!("{:?}", collect_pairs(vec!["a", "b"]));
    println!("{}", Sq(2.0).describe());
    println!("{}", Tri(3.0, 4.0).describe());
    println!("{}", total_sides::<Sq, Tri>());
    println!("{}", SidesArr::<Tri>::get());
    let s = Stack { v: vec![1, 2, 3] };
    println!("{} first={:?}", show_container(&s), s.first());
    let mut m = BTreeMap::new();
    m.insert(1, vec![2, 3]);
    m.insert(2, vec![4]);
    m.insert(3, vec![4, 1]);
    m.insert(4, vec![]);
    println!("{:?}", bfs(&Adj(m), 1));
    println!("{:?}", parse_all(&IntP, &["1", " 22 ", "3"]));
    println!("{:?}", parse_all(&IntP, &["1", "x"]));
    println!("{:?}", <IntP as Parser>::Error::default());
}
