use std::fmt::Debug;
use std::cmp::Ordering;

struct Heap<T, F> where F: Fn(&T, &T) -> bool {
    data: Vec<T>,
    before: F,
}

impl<T: Debug, F: Fn(&T, &T) -> bool> Heap<T, F> {
    fn new(before: F) -> Self { Heap { data: Vec::new(), before } }
    fn len(&self) -> usize { self.data.len() }
    fn push(&mut self, x: T) {
        self.data.push(x);
        let mut i = self.data.len() - 1;
        while i > 0 {
            let p = (i - 1) / 2;
            if (self.before)(&self.data[i], &self.data[p]) { self.data.swap(i, p); i = p; } else { break; }
        }
    }
    fn pop(&mut self) -> Option<T> {
        if self.data.is_empty() { return None; }
        let n = self.data.len() - 1;
        self.data.swap(0, n);
        let top = self.data.pop();
        let mut i = 0;
        loop {
            let (l, r) = (2 * i + 1, 2 * i + 2);
            let mut best = i;
            if l < self.data.len() && (self.before)(&self.data[l], &self.data[best]) { best = l; }
            if r < self.data.len() && (self.before)(&self.data[r], &self.data[best]) { best = r; }
            if best == i { break; }
            self.data.swap(i, best);
            i = best;
        }
        top
    }
    fn peek(&self) -> Option<&T> { self.data.first() }
    fn drain_sorted(mut self) -> Vec<T> {
        let mut v = vec![];
        while let Some(x) = self.pop() { v.push(x); }
        v
    }
}

fn min_heap<T: Ord + Debug>() -> Heap<T, impl Fn(&T, &T) -> bool> { Heap::new(|a: &T, b: &T| a < b) }
fn max_heap<T: Ord + Debug>() -> Heap<T, impl Fn(&T, &T) -> bool> { Heap::new(|a: &T, b: &T| a > b) }

#[derive(Debug, PartialEq, Eq)]
struct Task { pri: u8, seq: u32, name: String }
impl Ord for Task {
    fn cmp(&self, o: &Self) -> Ordering { self.pri.cmp(&o.pri).then_with(|| o.seq.cmp(&self.seq)) }
}
impl PartialOrd for Task { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }

struct PQ<T: Ord> { inner: std::collections::BinaryHeap<T> }
impl<T: Ord + Debug> PQ<T> {
    fn new() -> Self { PQ { inner: std::collections::BinaryHeap::new() } }
    fn add(&mut self, t: T) -> &mut Self { self.inner.push(t); self }
    fn take_n(&mut self, n: usize) -> Vec<T> { (0..n).filter_map(|_| self.inner.pop()).collect() }
}

fn dijkstra(adj: &[Vec<(usize, u32)>], src: usize) -> Vec<Option<u32>> {
    let mut dist = vec![None; adj.len()];
    let mut h = min_heap::<(u32, usize)>();
    h.push((0, src));
    while let Some((d, u)) = h.pop() {
        if dist[u].is_some() { continue; }
        dist[u] = Some(d);
        for &(v, w) in &adj[u] { if dist[v].is_none() { h.push((d + w, v)); } }
    }
    dist
}

fn main() {
    let mut h = min_heap();
    for x in [5, 3, 8, 1, 9, 2, 7] { h.push(x); }
    println!("peek {:?} len {}", h.peek(), h.len());
    println!("{:?}", h.drain_sorted());
    let mut mh = max_heap();
    for w in ["delta", "alpha", "echo", "bravo"] { mh.push(w.to_string()); }
    println!("{:?} {:?}", mh.pop(), mh.pop());
    let mut byl = Heap::new(|a: &&str, b: &&str| a.len() < b.len() || (a.len() == b.len() && a < b));
    for w in ["ccc", "a", "bb", "aa", "dddd", "b"] { byl.push(w); }
    println!("{:?}", byl.drain_sorted());
    let mut pq = PQ::new();
    let mut seq = 0;
    for (p, n) in [(2, "write"), (5, "urgent"), (2, "read"), (1, "idle"), (5, "fire")] {
        seq += 1;
        pq.add(Task { pri: p, seq, name: n.into() });
    }
    for t in pq.take_n(10) { println!("{} {} {}", t.pri, t.seq, t.name); }
    let mut pq2: PQ<std::cmp::Reverse<i32>> = PQ::new();
    pq2.add(std::cmp::Reverse(3)).add(std::cmp::Reverse(-1)).add(std::cmp::Reverse(10));
    println!("{:?}", pq2.take_n(2));
    let adj = vec![
        vec![(1, 4), (2, 1)],
        vec![(3, 1)],
        vec![(1, 2), (3, 5)],
        vec![(4, 3)],
        vec![],
        vec![(0, 1)],
    ];
    println!("{:?}", dijkstra(&adj, 0));
}
