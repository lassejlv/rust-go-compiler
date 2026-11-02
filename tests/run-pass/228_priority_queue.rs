// Priority queues: BinaryHeap with Reverse and custom Ord, plus a hand-written d-ary indexed heap.
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

#[derive(Debug, PartialEq, Eq)]
struct Job {
    priority: u8,
    seq: u32,
    name: &'static str,
}

impl Ord for Job {
    fn cmp(&self, o: &Self) -> Ordering {
        self.priority.cmp(&o.priority).then_with(|| o.seq.cmp(&self.seq))
    }
}
impl PartialOrd for Job {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

// f64 wrapper with total ordering for heap use
#[derive(Debug, Clone, Copy, PartialEq)]
struct OrdF64(f64);
impl Eq for OrdF64 {}
impl PartialOrd for OrdF64 {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for OrdF64 {
    fn cmp(&self, o: &Self) -> Ordering {
        self.0.total_cmp(&o.0)
    }
}

// Indexed min-heap supporting decrease-key (used for Dijkstra-like scheduling)
struct IndexedHeap<const D: usize> {
    heap: Vec<usize>,
    pos: Vec<Option<usize>>,
    key: Vec<i64>,
}

impl<const D: usize> IndexedHeap<D> {
    fn new(n: usize) -> Self {
        IndexedHeap { heap: vec![], pos: vec![None; n], key: vec![i64::MAX; n] }
    }
    fn swap(&mut self, a: usize, b: usize) {
        self.heap.swap(a, b);
        self.pos[self.heap[a]] = Some(a);
        self.pos[self.heap[b]] = Some(b);
    }
    fn sift_up(&mut self, mut i: usize) {
        while i > 0 {
            let p = (i - 1) / D;
            if self.key[self.heap[i]] < self.key[self.heap[p]] {
                self.swap(i, p);
                i = p;
            } else {
                break;
            }
        }
    }
    fn sift_down(&mut self, mut i: usize) {
        loop {
            let mut best = i;
            for c in (D * i + 1)..=(D * i + D) {
                if c < self.heap.len() && self.key[self.heap[c]] < self.key[self.heap[best]] {
                    best = c;
                }
            }
            if best == i {
                break;
            }
            self.swap(i, best);
            i = best;
        }
    }
    fn push_or_decrease(&mut self, item: usize, k: i64) -> bool {
        match self.pos[item] {
            Some(i) => {
                if k < self.key[item] {
                    self.key[item] = k;
                    self.sift_up(i);
                    true
                } else {
                    false
                }
            }
            None => {
                self.key[item] = k;
                self.heap.push(item);
                let i = self.heap.len() - 1;
                self.pos[item] = Some(i);
                self.sift_up(i);
                true
            }
        }
    }
    fn pop(&mut self) -> Option<(usize, i64)> {
        if self.heap.is_empty() {
            return None;
        }
        let last = self.heap.len() - 1;
        self.swap(0, last);
        let item = self.heap.pop().unwrap();
        self.pos[item] = None;
        if !self.heap.is_empty() {
            self.sift_down(0);
        }
        Some((item, self.key[item]))
    }
}

fn dijkstra(adj: &[Vec<(usize, i64)>], src: usize) -> Vec<Option<i64>> {
    let mut dist = vec![None; adj.len()];
    let mut h = IndexedHeap::<4>::new(adj.len());
    h.push_or_decrease(src, 0);
    while let Some((u, d)) = h.pop() {
        dist[u] = Some(d);
        for &(v, w) in &adj[u] {
            if dist[v].is_none() {
                h.push_or_decrease(v, d + w);
            }
        }
    }
    dist
}

fn main() {
    let mut jobs = BinaryHeap::new();
    let specs = [(2, "compile"), (5, "render"), (2, "lint"), (9, "input"), (5, "layout"), (1, "gc")];
    for (seq, (p, n)) in specs.iter().enumerate() {
        jobs.push(Job { priority: *p, seq: seq as u32, name: n });
    }
    println!("peek: {:?}", jobs.peek().map(|j| j.name));
    let mut order = vec![];
    while let Some(j) = jobs.pop() {
        order.push(format!("{}:{}", j.priority, j.name));
    }
    println!("{}", order.join(" "));

    let mut mins: BinaryHeap<Reverse<(u32, &str)>> = BinaryHeap::new();
    for (k, v) in [(5, "e"), (1, "a"), (3, "c"), (1, "aa"), (4, "d")] {
        mins.push(Reverse((k, v)));
    }
    let sorted: Vec<_> = std::iter::from_fn(|| mins.pop().map(|Reverse(x)| x)).collect();
    println!("{:?}", sorted);

    let mut fh: BinaryHeap<OrdF64> = [3.5, -1.0, 2.25, f64::INFINITY, 0.0, -0.0].iter().map(|&x| OrdF64(x)).collect();
    print!("floats desc:");
    while let Some(OrdF64(x)) = fh.pop() {
        print!(" {}", x);
    }
    println!();

    // top-k smallest using a bounded max-heap
    let data = [42, 7, 19, 3, 88, 1, 56, 23, 7, 14];
    let mut topk = BinaryHeap::with_capacity(4);
    for &x in &data {
        topk.push(x);
        if topk.len() > 4 {
            topk.pop();
        }
    }
    println!("4 smallest: {:?}", topk.into_sorted_vec());

    let heap_vec = BinaryHeap::from(vec![4, 1, 8, 3]).into_vec().len();
    println!("into_vec len {}", heap_vec);

    let adj = vec![
        vec![(1, 4), (2, 1)],
        vec![(3, 1)],
        vec![(1, 2), (3, 5)],
        vec![(4, 3)],
        vec![],
        vec![(0, 1)],
    ];
    println!("{:?}", dijkstra(&adj, 0));
    let mut h = IndexedHeap::<2>::new(6);
    for (i, k) in [(0, 50), (1, 40), (2, 30), (3, 20), (4, 10)] {
        h.push_or_decrease(i, k);
    }
    println!("decrease 0->5: {} ; 4->99: {}", h.push_or_decrease(0, 5), h.push_or_decrease(4, 99));
    let mut out = vec![];
    while let Some(x) = h.pop() {
        out.push(x);
    }
    println!("{:?}", out);
}
