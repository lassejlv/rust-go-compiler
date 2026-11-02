use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};
use std::fmt::Debug;

trait Graph {
    type Node: Copy + Ord + Debug;
    fn nodes(&self) -> Vec<Self::Node>;
    fn neighbors(&self, n: Self::Node) -> Vec<(Self::Node, u32)>;

    fn bfs(&self, start: Self::Node) -> Vec<Self::Node> {
        let mut seen = BTreeSet::new();
        let mut q = VecDeque::new();
        let mut out = Vec::new();
        seen.insert(start);
        q.push_back(start);
        while let Some(n) = q.pop_front() {
            out.push(n);
            for (m, _) in self.neighbors(n) {
                if seen.insert(m) {
                    q.push_back(m);
                }
            }
        }
        out
    }

    fn dfs(&self, start: Self::Node) -> Vec<Self::Node> {
        fn go<G: Graph + ?Sized>(g: &G, n: G::Node, seen: &mut BTreeSet<G::Node>, out: &mut Vec<G::Node>) {
            if !seen.insert(n) {
                return;
            }
            out.push(n);
            for (m, _) in g.neighbors(n) {
                go(g, m, seen, out);
            }
        }
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        go(self, start, &mut seen, &mut out);
        out
    }

    fn topo_sort(&self) -> Result<Vec<Self::Node>, Self::Node> {
        let mut indeg: BTreeMap<Self::Node, usize> = self.nodes().into_iter().map(|n| (n, 0)).collect();
        for n in self.nodes() {
            for (m, _) in self.neighbors(n) {
                *indeg.get_mut(&m).unwrap() += 1;
            }
        }
        let mut ready: BTreeSet<Self::Node> = indeg.iter().filter(|(_, &d)| d == 0).map(|(&n, _)| n).collect();
        let mut out = Vec::new();
        while let Some(&n) = ready.iter().next() {
            ready.remove(&n);
            out.push(n);
            for (m, _) in self.neighbors(n) {
                let d = indeg.get_mut(&m).unwrap();
                *d -= 1;
                if *d == 0 {
                    ready.insert(m);
                }
            }
        }
        if out.len() == indeg.len() {
            Ok(out)
        } else {
            Err(*indeg.iter().find(|(_, &d)| d > 0).unwrap().0)
        }
    }

    fn dijkstra(&self, src: Self::Node) -> BTreeMap<Self::Node, (u32, Vec<Self::Node>)> {
        let mut dist: BTreeMap<Self::Node, u32> = BTreeMap::new();
        let mut prev: BTreeMap<Self::Node, Self::Node> = BTreeMap::new();
        let mut heap = BinaryHeap::new();
        dist.insert(src, 0);
        heap.push(Reverse((0u32, src)));
        while let Some(Reverse((d, n))) = heap.pop() {
            if d > dist[&n] {
                continue;
            }
            for (m, w) in self.neighbors(n) {
                let nd = d + w;
                if dist.get(&m).map_or(true, |&cur| nd < cur) {
                    dist.insert(m, nd);
                    prev.insert(m, n);
                    heap.push(Reverse((nd, m)));
                }
            }
        }
        dist.iter()
            .map(|(&n, &d)| {
                let mut path = vec![n];
                let mut cur = n;
                while let Some(&p) = prev.get(&cur) {
                    path.push(p);
                    cur = p;
                }
                path.reverse();
                (n, (d, path))
            })
            .collect()
    }
}

struct AdjList {
    edges: BTreeMap<u32, Vec<(u32, u32)>>,
}

impl AdjList {
    fn new(n: u32) -> Self {
        AdjList { edges: (0..n).map(|i| (i, Vec::new())).collect() }
    }
    fn add(&mut self, a: u32, b: u32, w: u32) {
        self.edges.get_mut(&a).unwrap().push((b, w));
    }
}

impl Graph for AdjList {
    type Node = u32;
    fn nodes(&self) -> Vec<u32> {
        self.edges.keys().copied().collect()
    }
    fn neighbors(&self, n: u32) -> Vec<(u32, u32)> {
        self.edges[&n].clone()
    }
}

struct Grid {
    rows: Vec<&'static str>,
}

impl Graph for Grid {
    type Node = (i32, i32);
    fn nodes(&self) -> Vec<(i32, i32)> {
        let mut v = vec![];
        for (r, row) in self.rows.iter().enumerate() {
            for (c, ch) in row.chars().enumerate() {
                if ch != '#' {
                    v.push((r as i32, c as i32));
                }
            }
        }
        v
    }
    fn neighbors(&self, (r, c): (i32, i32)) -> Vec<((i32, i32), u32)> {
        let mut v = vec![];
        for (dr, dc) in [(0, 1), (1, 0), (0, -1), (-1, 0)] {
            let (nr, nc) = (r + dr, c + dc);
            if nr < 0 || nc < 0 || nr as usize >= self.rows.len() {
                continue;
            }
            match self.rows[nr as usize].as_bytes().get(nc as usize) {
                Some(b'#') | None => {}
                Some(b'~') => v.push(((nr, nc), 5)),
                Some(_) => v.push(((nr, nc), 1)),
            }
        }
        v
    }
}

fn main() {
    let mut g = AdjList::new(7);
    for &(a, b, w) in &[(0, 1, 4), (0, 2, 1), (2, 1, 2), (1, 3, 1), (2, 3, 5), (3, 4, 3), (4, 5, 1), (2, 5, 10), (5, 6, 2)] {
        g.add(a, b, w);
    }
    println!("bfs {:?}", g.bfs(0));
    println!("dfs {:?}", g.dfs(0));
    println!("topo {:?}", g.topo_sort());
    for (n, (d, path)) in g.dijkstra(0) {
        println!("  0 -> {}: dist {:>2} via {:?}", n, d, path);
    }
    g.add(6, 2, 1);
    println!("topo with cycle {:?}", g.topo_sort());
    println!("bfs from 4 {:?}", g.bfs(4));

    let grid = Grid { rows: vec!["S..#....", ".#.#.##.", ".#...#..", ".####.~.", "......#E"] };
    println!("open cells {}", grid.nodes().len());
    let res = grid.dijkstra((0, 0));
    let (d, path) = &res[&(4, 7)];
    println!("grid S->E dist {} steps {}", d, path.len() - 1);
    let mut canvas: Vec<Vec<char>> = grid.rows.iter().map(|r| r.chars().collect()).collect();
    for &(r, c) in &path[1..path.len() - 1] {
        canvas[r as usize][c as usize] = '*';
    }
    for row in canvas {
        println!("  {}", row.into_iter().collect::<String>());
    }
    println!("reachable from S: {}", grid.bfs((0, 0)).len());
}
