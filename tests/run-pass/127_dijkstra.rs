use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

struct Graph {
    names: Vec<&'static str>,
    adj: Vec<Vec<(usize, u32)>>,
}

impl Graph {
    fn new(names: Vec<&'static str>) -> Self {
        let n = names.len();
        Graph { names, adj: vec![Vec::new(); n] }
    }
    fn idx(&self, name: &str) -> usize {
        self.names.iter().position(|n| *n == name).unwrap()
    }
    fn edge(&mut self, a: &str, b: &str, w: u32) {
        let (i, j) = (self.idx(a), self.idx(b));
        self.adj[i].push((j, w));
        self.adj[j].push((i, w));
    }
    fn dijkstra(&self, src: usize) -> (Vec<Option<u32>>, Vec<Option<usize>>) {
        let n = self.names.len();
        let mut dist: Vec<Option<u32>> = vec![None; n];
        let mut prev = vec![None; n];
        let mut heap = BinaryHeap::new();
        dist[src] = Some(0);
        heap.push(Reverse((0u32, src)));
        while let Some(Reverse((d, u))) = heap.pop() {
            if let Some(best) = dist[u] {
                if d > best {
                    continue;
                }
            }
            for &(v, w) in &self.adj[u] {
                let nd = d + w;
                let better = match dist[v] {
                    None => true,
                    Some(cur) => nd < cur,
                };
                if better {
                    dist[v] = Some(nd);
                    prev[v] = Some(u);
                    heap.push(Reverse((nd, v)));
                }
            }
        }
        (dist, prev)
    }
    fn path(&self, prev: &[Option<usize>], mut t: usize) -> Vec<&str> {
        let mut p = vec![self.names[t]];
        while let Some(u) = prev[t] {
            p.push(self.names[u]);
            t = u;
        }
        p.reverse();
        p
    }
}

fn main() {
    let mut g = Graph::new(vec!["A", "B", "C", "D", "E", "F", "G", "H"]);
    g.edge("A", "B", 4);
    g.edge("A", "C", 2);
    g.edge("B", "C", 5);
    g.edge("B", "D", 10);
    g.edge("C", "E", 3);
    g.edge("E", "D", 4);
    g.edge("D", "F", 11);
    g.edge("E", "F", 20);
    g.edge("F", "G", 1);
    let src = g.idx("A");
    let (dist, prev) = g.dijkstra(src);
    for (i, d) in dist.iter().enumerate() {
        match d {
            Some(d) => println!("A -> {}: {:>3} via {}", g.names[i], d, g.path(&prev, i).join("-")),
            None => println!("A -> {}: unreachable", g.names[i]),
        }
    }
    let mut heap = BinaryHeap::new();
    for x in [5, 1, 8, 3, 9, 2] {
        heap.push(x);
    }
    println!("max-heap peek = {:?}, len = {}", heap.peek(), heap.len());
    let mut drained = Vec::new();
    while let Some(x) = heap.pop() {
        drained.push(x);
    }
    println!("max-heap order = {:?}", drained);
    let mut minheap: BinaryHeap<Reverse<(u32, &str)>> = BinaryHeap::new();
    for (p, t) in [(3, "low"), (1, "urgent"), (2, "normal"), (1, "also urgent")] {
        minheap.push(Reverse((p, t)));
    }
    while let Some(Reverse((p, t))) = minheap.pop() {
        println!("task p{} {}", p, t);
    }
    let words = ["delta", "alpha", "charlie", "bravo"];
    let sorted = BinaryHeap::from(words.to_vec()).into_sorted_vec();
    println!("into_sorted_vec = {:?}", sorted);
    let mut counts: HashMap<char, u32> = HashMap::new();
    for c in "programming language".chars().filter(|c| c.is_alphabetic()) {
        *counts.entry(c).or_insert(0) += 1;
    }
    let mut top: BinaryHeap<(u32, Reverse<char>)> = counts.iter().map(|(c, n)| (*n, Reverse(*c))).collect();
    let mut best = Vec::new();
    for _ in 0..3 {
        if let Some((n, Reverse(c))) = top.pop() {
            best.push(format!("{}={}", c, n));
        }
    }
    println!("top 3 letters = {}", best.join(", "));
}
