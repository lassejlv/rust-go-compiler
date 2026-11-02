// Associated-type projections in bounds, as found in iterator-heavy / taffy-like code.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::iter::FromIterator;

fn collect_into<C, I>(it: I) -> C
where
    I: IntoIterator,
    C: FromIterator<I::Item>,
{
    it.into_iter().collect()
}

fn group_by_key<I, K, V, F>(it: I, key: F) -> BTreeMap<K, Vec<V>>
where
    I: IntoIterator<Item = V>,
    K: Ord,
    F: Fn(&V) -> K,
{
    let mut m: BTreeMap<K, Vec<V>> = BTreeMap::new();
    for v in it {
        m.entry(key(&v)).or_default().push(v);
    }
    m
}

fn sum_nested<O, I>(outer: O) -> i64
where
    O: IntoIterator<Item = I>,
    I: IntoIterator,
    I::Item: Into<i64>,
{
    outer.into_iter().flat_map(|i| i.into_iter().map(Into::into)).sum()
}

trait Graph {
    type Node: Copy + Ord + Debug;
    type Edges<'a>: Iterator<Item = Self::Node>
    where
        Self: 'a;
    fn neighbors<'a>(&'a self, n: Self::Node) -> Self::Edges<'a>;
}

struct AdjList {
    adj: Vec<Vec<usize>>,
}

impl Graph for AdjList {
    type Node = usize;
    type Edges<'a> = std::iter::Copied<std::slice::Iter<'a, usize>>;
    fn neighbors<'a>(&'a self, n: usize) -> Self::Edges<'a> {
        self.adj[n].iter().copied()
    }
}

fn bfs<G: Graph>(g: &G, start: G::Node) -> Vec<G::Node> {
    let mut seen = BTreeSet::new();
    let mut order = vec![];
    let mut q = std::collections::VecDeque::new();
    q.push_back(start);
    seen.insert(start);
    while let Some(n) = q.pop_front() {
        order.push(n);
        for m in g.neighbors(n) {
            if seen.insert(m) {
                q.push_back(m);
            }
        }
    }
    order
}

trait Monoid {
    type Elem: Clone + Debug;
    fn empty() -> Self::Elem;
    fn combine(a: &Self::Elem, b: &Self::Elem) -> Self::Elem;
}

struct SumM;
struct MaxM;
struct ConcatM;
impl Monoid for SumM {
    type Elem = i32;
    fn empty() -> i32 {
        0
    }
    fn combine(a: &i32, b: &i32) -> i32 {
        a + b
    }
}
impl Monoid for MaxM {
    type Elem = Option<i32>;
    fn empty() -> Option<i32> {
        None
    }
    fn combine(a: &Option<i32>, b: &Option<i32>) -> Option<i32> {
        match (a, b) {
            (Some(x), Some(y)) => Some(*x.max(y)),
            (x, None) => *x,
            (None, y) => *y,
        }
    }
}
impl Monoid for ConcatM {
    type Elem = String;
    fn empty() -> String {
        String::new()
    }
    fn combine(a: &String, b: &String) -> String {
        format!("{}{}", a, b)
    }
}

fn fold_all<M: Monoid>(xs: &[M::Elem]) -> M::Elem {
    xs.iter().fold(M::empty(), |acc, x| M::combine(&acc, x))
}

// Segment tree over any monoid
struct SegTree<M: Monoid> {
    n: usize,
    t: Vec<M::Elem>,
}

impl<M: Monoid> SegTree<M> {
    fn new(xs: &[M::Elem]) -> Self {
        let n = xs.len().next_power_of_two();
        let mut t = vec![M::empty(); 2 * n];
        for (i, x) in xs.iter().enumerate() {
            t[n + i] = x.clone();
        }
        for i in (1..n).rev() {
            t[i] = M::combine(&t[2 * i], &t[2 * i + 1]);
        }
        SegTree { n, t }
    }
    fn query(&self, mut l: usize, mut r: usize) -> M::Elem {
        let mut left = M::empty();
        let mut right = M::empty();
        l += self.n;
        r += self.n;
        while l < r {
            if l & 1 == 1 {
                left = M::combine(&left, &self.t[l]);
                l += 1;
            }
            if r & 1 == 1 {
                r -= 1;
                right = M::combine(&self.t[r], &right);
            }
            l >>= 1;
            r >>= 1;
        }
        M::combine(&left, &right)
    }
}

fn main() {
    let v: Vec<u8> = collect_into("abc".bytes());
    let s: String = collect_into(vec!['x', 'y']);
    let set: BTreeSet<i32> = collect_into(vec![3, 1, 3, 2]);
    println!("{:?} {:?} {:?}", v, s, set);
    let g = group_by_key(vec!["apple", "avocado", "banana", "blueberry", "cherry"], |s| s.chars().next().unwrap());
    println!("{:?}", g);
    let g2 = group_by_key(1..=10, |n| n % 3);
    println!("{:?}", g2);
    println!("{}", sum_nested(vec![vec![1i32, 2], vec![3, 4, 5]]));
    println!("{}", sum_nested([[10u8; 3], [1u8; 3]]));

    let graph = AdjList { adj: vec![vec![1, 2], vec![3], vec![3, 4], vec![5], vec![5], vec![]] };
    println!("bfs {:?}", bfs(&graph, 0));
    println!("bfs {:?}", bfs(&graph, 2));

    println!("{}", fold_all::<SumM>(&[1, 2, 3, 4]));
    println!("{:?}", fold_all::<MaxM>(&[Some(3), None, Some(9), Some(-1)]));
    println!("{:?}", fold_all::<ConcatM>(&["a".into(), "b".into(), "c".into()]));

    let st = SegTree::<SumM>::new(&[5, 3, 8, 6, 1, 4, 7]);
    println!("{} {} {}", st.query(0, 7), st.query(2, 5), st.query(3, 4));
    let sm = SegTree::<MaxM>::new(&[Some(5), Some(3), Some(8), None, Some(1)]);
    println!("{:?} {:?} {:?}", sm.query(0, 2), sm.query(1, 5), sm.query(3, 4));
    let sc = SegTree::<ConcatM>::new(&["h".into(), "e".into(), "l".into(), "l".into(), "o".into()]);
    println!("{:?} {:?}", sc.query(0, 5), sc.query(1, 4));
}
