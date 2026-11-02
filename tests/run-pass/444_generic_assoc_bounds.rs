// Generic code over traits with associated types: bounds on associated types (T::Item: Display),
// associated type equality constraints (Iterator<Item = u8>), projections in struct fields,
// traits with associated types that themselves have bounds, and generic recursion over containers.
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::hash::Hash;

trait Container {
    type Item;
    type Iter<'a>: Iterator<Item = &'a Self::Item>
    where
        Self: 'a;
    fn items<'a>(&'a self) -> Self::Iter<'a>;
    fn first(&self) -> Option<&Self::Item> {
        self.items().next()
    }
}

struct Stack<T> {
    v: Vec<T>,
}
impl<T> Container for Stack<T> {
    type Item = T;
    type Iter<'a>
        = std::iter::Rev<std::slice::Iter<'a, T>>
    where
        T: 'a;
    fn items<'a>(&'a self) -> Self::Iter<'a> {
        self.v.iter().rev()
    }
}

struct Pairs<K, V> {
    v: Vec<(K, V)>,
}
impl<K, V> Container for Pairs<K, V> {
    type Item = (K, V);
    type Iter<'a>
        = std::slice::Iter<'a, (K, V)>
    where
        K: 'a,
        V: 'a;
    fn items<'a>(&'a self) -> Self::Iter<'a> {
        self.v.iter()
    }
}

fn join_all<C>(c: &C) -> String
where
    C: Container,
    C::Item: Debug,
{
    c.items().map(|x| format!("{:?}", x)).collect::<Vec<_>>().join(" ")
}

trait Graph {
    type Node: Copy + Eq + Hash + Debug + Ord;
    type Edge: Display;
    fn neighbors(&self, n: Self::Node) -> Vec<(Self::Node, Self::Edge)>;
}

struct Grid {
    w: i32,
    h: i32,
}
impl Graph for Grid {
    type Node = (i32, i32);
    type Edge = char;
    fn neighbors(&self, (x, y): (i32, i32)) -> Vec<((i32, i32), char)> {
        let mut v = vec![];
        for (dx, dy, c) in [(1, 0, 'R'), (0, 1, 'D'), (-1, 0, 'L'), (0, -1, 'U')] {
            let (nx, ny) = (x + dx, y + dy);
            if nx >= 0 && ny >= 0 && nx < self.w && ny < self.h {
                v.push(((nx, ny), c));
            }
        }
        v
    }
}

fn bfs<G: Graph>(g: &G, start: G::Node) -> Vec<(G::Node, String)> {
    let mut seen: HashMap<G::Node, String> = HashMap::new();
    let mut q = std::collections::VecDeque::new();
    seen.insert(start, String::new());
    q.push_back(start);
    while let Some(n) = q.pop_front() {
        let path = seen[&n].clone();
        for (m, e) in g.neighbors(n) {
            if !seen.contains_key(&m) {
                seen.insert(m, format!("{}{}", path, e));
                q.push_back(m);
            }
        }
    }
    let mut out: Vec<_> = seen.into_iter().collect();
    out.sort();
    out
}

fn sum_nested<I>(it: I) -> i64
where
    I: IntoIterator,
    I::Item: IntoIterator,
    <I::Item as IntoIterator>::Item: Into<i64>,
{
    it.into_iter().map(|inner| inner.into_iter().map(Into::into).sum::<i64>()).sum()
}

fn describe_bytes<I: Iterator<Item = u8>>(it: I) -> String {
    it.map(|b| format!("{:02X}", b)).collect()
}

struct Cache<F, A, R>
where
    F: Fn(&A) -> R,
    A: Hash + Eq + Clone,
{
    f: F,
    map: HashMap<A, R>,
}
impl<F: Fn(&A) -> R, A: Hash + Eq + Clone, R: Clone> Cache<F, A, R> {
    fn get(&mut self, a: &A) -> R {
        if let Some(r) = self.map.get(a) {
            return r.clone();
        }
        let r = (self.f)(a);
        self.map.insert(a.clone(), r.clone());
        r
    }
}

fn main() {
    let s = Stack { v: vec![1, 2, 3] };
    println!("{} first={:?}", join_all(&s), s.first());
    let p = Pairs { v: vec![("a", 1.5), ("b", -2.0)] };
    println!("{} first={:?}", join_all(&p), p.first());
    for (n, path) in bfs(&Grid { w: 3, h: 2 }, (0, 0)) {
        println!("{:?} via {:?}", n, path);
    }
    println!("{}", sum_nested(vec![vec![1i32, 2], vec![3]]));
    println!("{}", sum_nested([[1u8, 2, 3], [4, 5, 6]]));
    println!("{}", describe_bytes("Hi!".bytes()));
    println!("{}", describe_bytes([1u8, 254].into_iter().chain(std::iter::once(16))));
    let mut c = Cache { f: |s: &String| s.len() * 2, map: HashMap::new() };
    println!("{} {} {} size={}", c.get(&"abc".to_string()), c.get(&"abc".to_string()), c.get(&"x".to_string()), c.map.len());
}
