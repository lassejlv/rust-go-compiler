// Generic graph algorithms over a Graph trait with associated node/edge types.
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};
use std::cmp::Reverse;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Add;

trait Weight: Copy + Ord + Add<Output = Self> + Debug {
    fn zero() -> Self;
}
impl Weight for u32 { fn zero() -> Self { 0 } }
impl Weight for i64 { fn zero() -> Self { 0 } }

trait Graph {
    type Node: Clone + Ord + Hash + Debug;
    type W: Weight;
    fn nodes(&self) -> Vec<Self::Node>;
    fn edges_from(&self, n: &Self::Node) -> Vec<(Self::Node, Self::W)>;

    fn neighbors(&self, n: &Self::Node) -> Vec<Self::Node> {
        self.edges_from(n).into_iter().map(|(m, _)| m).collect()
    }
    fn edge_count(&self) -> usize {
        self.nodes().iter().map(|n| self.edges_from(n).len()).sum()
    }
}

#[derive(Debug, Clone)]
struct MapGraph<N: Ord, W> {
    adj: BTreeMap<N, Vec<(N, W)>>,
}

impl<N: Clone + Ord + Hash + Debug, W: Weight> MapGraph<N, W> {
    fn new() -> Self { MapGraph { adj: BTreeMap::new() } }
    fn add_edge(&mut self, a: N, b: N, w: W) -> &mut Self {
        self.adj.entry(b.clone()).or_default();
        self.adj.entry(a).or_default().push((b, w));
        self
    }
    fn add_undirected(&mut self, a: N, b: N, w: W) -> &mut Self {
        self.add_edge(a.clone(), b.clone(), w);
        self.add_edge(b, a, w)
    }
}

impl<N: Clone + Ord + Hash + Debug, W: Weight> Graph for MapGraph<N, W> {
    type Node = N;
    type W = W;
    fn nodes(&self) -> Vec<N> { self.adj.keys().cloned().collect() }
    fn edges_from(&self, n: &N) -> Vec<(N, W)> { self.adj.get(n).cloned().unwrap_or_default() }
}

// Implicit grid graph: nodes are (x, y) coordinates, walls block.
struct Grid { w: i32, h: i32, walls: BTreeSet<(i32, i32)> }
impl Graph for Grid {
    type Node = (i32, i32);
    type W = u32;
    fn nodes(&self) -> Vec<(i32, i32)> {
        let mut v = vec![];
        for y in 0..self.h { for x in 0..self.w { if !self.walls.contains(&(x, y)) { v.push((x, y)); } } }
        v
    }
    fn edges_from(&self, &(x, y): &(i32, i32)) -> Vec<((i32, i32), u32)> {
        [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .map(|(dx, dy)| (x + dx, y + dy))
            .filter(|&(a, b)| a >= 0 && b >= 0 && a < self.w && b < self.h && !self.walls.contains(&(a, b)))
            .map(|p| (p, 1))
            .collect()
    }
}

fn bfs<G: Graph>(g: &G, start: &G::Node) -> Vec<G::Node> {
    let mut seen = BTreeSet::new();
    let mut q = VecDeque::new();
    let mut order = vec![];
    seen.insert(start.clone());
    q.push_back(start.clone());
    while let Some(n) = q.pop_front() {
        order.push(n.clone());
        for m in g.neighbors(&n) {
            if seen.insert(m.clone()) { q.push_back(m); }
        }
    }
    order
}

fn dfs<G: Graph>(g: &G, start: &G::Node) -> Vec<G::Node> {
    fn go<G: Graph>(g: &G, n: &G::Node, seen: &mut BTreeSet<G::Node>, out: &mut Vec<G::Node>) {
        if !seen.insert(n.clone()) { return; }
        out.push(n.clone());
        for m in g.neighbors(n) { go(g, &m, seen, out); }
    }
    let mut seen = BTreeSet::new();
    let mut out = vec![];
    go(g, start, &mut seen, &mut out);
    out
}

fn dijkstra<G: Graph>(g: &G, src: &G::Node) -> BTreeMap<G::Node, (G::W, Option<G::Node>)> {
    let mut best: BTreeMap<G::Node, (G::W, Option<G::Node>)> = BTreeMap::new();
    let mut heap = BinaryHeap::new();
    best.insert(src.clone(), (G::W::zero(), None));
    heap.push(Reverse((G::W::zero(), src.clone())));
    while let Some(Reverse((d, n))) = heap.pop() {
        if let Some(&(bd, _)) = best.get(&n) { if d > bd { continue; } }
        for (m, w) in g.edges_from(&n) {
            let nd = d + w;
            let better = match best.get(&m) { None => true, Some(&(od, _)) => nd < od };
            if better {
                best.insert(m.clone(), (nd, Some(n.clone())));
                heap.push(Reverse((nd, m)));
            }
        }
    }
    best
}

fn path_to<N: Ord + Clone, W>(info: &BTreeMap<N, (W, Option<N>)>, target: &N) -> Option<Vec<N>> {
    let mut cur = target.clone();
    let mut p = vec![cur.clone()];
    info.get(target)?;
    while let Some((_, Some(prev))) = info.get(&cur) {
        p.push(prev.clone());
        cur = prev.clone();
    }
    p.reverse();
    Some(p)
}

fn topo_sort<G: Graph>(g: &G) -> Result<Vec<G::Node>, G::Node> {
    let mut indeg: BTreeMap<G::Node, usize> = g.nodes().into_iter().map(|n| (n, 0)).collect();
    for n in g.nodes() { for m in g.neighbors(&n) { *indeg.get_mut(&m).unwrap() += 1; } }
    let mut ready: BTreeSet<G::Node> = indeg.iter().filter(|(_, &d)| d == 0).map(|(n, _)| n.clone()).collect();
    let mut out = vec![];
    while let Some(n) = ready.iter().next().cloned() {
        ready.remove(&n);
        out.push(n.clone());
        for m in g.neighbors(&n) {
            let d = indeg.get_mut(&m).unwrap();
            *d -= 1;
            if *d == 0 { ready.insert(m); }
        }
    }
    if out.len() == indeg.len() { Ok(out) } else {
        Err(indeg.into_iter().find(|(_, d)| *d > 0).map(|(n, _)| n).unwrap())
    }
}

fn components<G: Graph>(g: &G) -> Vec<Vec<G::Node>> {
    let mut seen: BTreeSet<G::Node> = BTreeSet::new();
    let mut comps = vec![];
    for n in g.nodes() {
        if seen.contains(&n) { continue; }
        let c = bfs(g, &n);
        for x in &c { seen.insert(x.clone()); }
        let mut c = c;
        c.sort();
        comps.push(c);
    }
    comps
}

fn render_grid(g: &Grid, path: &[(i32, i32)]) -> String {
    let mut s = String::new();
    for y in 0..g.h {
        for x in 0..g.w {
            let c = if g.walls.contains(&(x, y)) { '#' } else if path.contains(&(x, y)) { '*' } else { '.' };
            s.push(c);
        }
        s.push('\n');
    }
    s
}

fn main() {
    let mut g: MapGraph<&str, u32> = MapGraph::new();
    g.add_edge("a", "b", 4).add_edge("a", "c", 1).add_edge("c", "b", 2).add_edge("b", "d", 5).add_edge("c", "d", 8).add_edge("d", "e", 3);
    println!("nodes {:?} edges {}", g.nodes(), g.edge_count());
    println!("bfs {:?}", bfs(&g, &"a"));
    println!("dfs {:?}", dfs(&g, &"a"));
    let dj = dijkstra(&g, &"a");
    for (n, (d, p)) in &dj { println!("  {} dist={:?} via={:?}", n, d, p); }
    println!("path {:?}", path_to(&dj, &"e"));
    println!("topo {:?}", topo_sort(&g));
    g.add_edge("e", "c", 1);
    println!("topo after cycle {:?}", topo_sort(&g));

    let mut ug: MapGraph<u8, i64> = MapGraph::new();
    ug.add_undirected(1, 2, 7).add_undirected(2, 3, -1).add_undirected(4, 5, 2).add_undirected(6, 6, 0);
    println!("components {:?}", components(&ug));

    let walls: BTreeSet<(i32, i32)> = [(1, 0), (1, 1), (1, 2), (3, 1), (3, 2), (3, 3)].into_iter().collect();
    let grid = Grid { w: 5, h: 4, walls };
    let info = dijkstra(&grid, &(0, 0));
    let p = path_to(&info, &(4, 3)).unwrap();
    println!("grid path len {} cost {:?}", p.len(), info[&(4, 3)].0);
    print!("{}", render_grid(&grid, &p));
    println!("reachable {}", bfs(&grid, &(0, 0)).len());
    println!("unreachable {:?}", path_to(&dijkstra(&grid, &(0, 0)), &(9, 9)));
}
