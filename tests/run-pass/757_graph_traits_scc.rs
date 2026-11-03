// Generic graph algorithms over a Graph trait with associated types:
// Tarjan's SCC (recursive with explicit state), condensation DAG,
// topological order, BFS layers, bipartiteness, and Kruskal MST with a
// union-find, on both adjacency-list and implicit grid graphs.
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::hash::Hash;

trait Graph {
    type Node: Copy + Ord + Hash + std::fmt::Debug;
    fn nodes(&self) -> Vec<Self::Node>;
    fn neighbors(&self, n: Self::Node) -> Vec<Self::Node>;
}

struct AdjList { adj: BTreeMap<u32, Vec<u32>> }
impl AdjList {
    fn from_edges(edges: &[(u32, u32)]) -> Self {
        let mut adj: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for &(a, b) in edges { adj.entry(a).or_default().push(b); adj.entry(b).or_default(); }
        AdjList { adj }
    }
}
impl Graph for AdjList {
    type Node = u32;
    fn nodes(&self) -> Vec<u32> { self.adj.keys().copied().collect() }
    fn neighbors(&self, n: u32) -> Vec<u32> { self.adj.get(&n).cloned().unwrap_or_default() }
}

struct Grid { w: i32, h: i32, walls: BTreeSet<(i32, i32)> }
impl Graph for Grid {
    type Node = (i32, i32);
    fn nodes(&self) -> Vec<(i32, i32)> { (0..self.h).flat_map(|r| (0..self.w).map(move |c| (r, c))).filter(|p| !self.walls.contains(p)).collect() }
    fn neighbors(&self, (r, c): (i32, i32)) -> Vec<(i32, i32)> {
        [(r - 1, c), (r + 1, c), (r, c - 1), (r, c + 1)].into_iter()
            .filter(|&(a, b)| a >= 0 && b >= 0 && a < self.h && b < self.w && !self.walls.contains(&(a, b))).collect()
    }
}

struct Tarjan<'g, G: Graph> { g: &'g G, index: BTreeMap<G::Node, usize>, low: BTreeMap<G::Node, usize>, stack: Vec<G::Node>, on: BTreeSet<G::Node>, next: usize, out: Vec<Vec<G::Node>> }
impl<'g, G: Graph> Tarjan<'g, G> {
    fn run(g: &'g G) -> Vec<Vec<G::Node>> {
        let mut t = Tarjan { g, index: BTreeMap::new(), low: BTreeMap::new(), stack: vec![], on: BTreeSet::new(), next: 0, out: vec![] };
        for n in g.nodes() { if !t.index.contains_key(&n) { t.visit(n); } }
        t.out
    }
    fn visit(&mut self, v: G::Node) {
        self.index.insert(v, self.next);
        self.low.insert(v, self.next);
        self.next += 1;
        self.stack.push(v);
        self.on.insert(v);
        for w in self.g.neighbors(v) {
            if !self.index.contains_key(&w) {
                self.visit(w);
                let lw = self.low[&w];
                let lv = self.low.get_mut(&v).unwrap();
                *lv = (*lv).min(lw);
            } else if self.on.contains(&w) {
                let iw = self.index[&w];
                let lv = self.low.get_mut(&v).unwrap();
                *lv = (*lv).min(iw);
            }
        }
        if self.low[&v] == self.index[&v] {
            let mut comp = vec![];
            loop {
                let w = self.stack.pop().unwrap();
                self.on.remove(&w);
                comp.push(w);
                if w == v { break; }
            }
            comp.sort();
            self.out.push(comp);
        }
    }
}

fn bfs_layers<G: Graph>(g: &G, start: G::Node) -> BTreeMap<usize, Vec<G::Node>> {
    let mut dist = BTreeMap::new();
    let mut q = VecDeque::from([start]);
    dist.insert(start, 0usize);
    while let Some(n) = q.pop_front() {
        let d = dist[&n];
        for m in g.neighbors(n) {
            if !dist.contains_key(&m) { dist.insert(m, d + 1); q.push_back(m); }
        }
    }
    let mut layers: BTreeMap<usize, Vec<G::Node>> = BTreeMap::new();
    for (n, d) in dist { layers.entry(d).or_default().push(n); }
    layers
}

fn bipartite<G: Graph>(g: &G) -> Result<usize, (G::Node, G::Node)> {
    let mut color: BTreeMap<G::Node, bool> = BTreeMap::new();
    for s in g.nodes() {
        if color.contains_key(&s) { continue; }
        color.insert(s, false);
        let mut q = VecDeque::from([s]);
        while let Some(n) = q.pop_front() {
            for m in g.neighbors(n) {
                match color.get(&m) {
                    None => { color.insert(m, !color[&n]); q.push_back(m); }
                    Some(&c) if c == color[&n] => return Err((n, m)),
                    _ => {}
                }
            }
        }
    }
    Ok(color.values().filter(|&&c| c).count())
}

fn topo<G: Graph>(g: &G) -> Option<Vec<G::Node>> {
    let mut indeg: BTreeMap<G::Node, usize> = g.nodes().into_iter().map(|n| (n, 0)).collect();
    for n in g.nodes() { for m in g.neighbors(n) { *indeg.get_mut(&m).unwrap() += 1; } }
    let mut ready: BTreeSet<G::Node> = indeg.iter().filter(|(_, &d)| d == 0).map(|(n, _)| *n).collect();
    let mut out = vec![];
    while let Some(n) = ready.pop_first() {
        out.push(n);
        for m in g.neighbors(n) { let d = indeg.get_mut(&m).unwrap(); *d -= 1; if *d == 0 { ready.insert(m); } }
    }
    if out.len() == indeg.len() { Some(out) } else { None }
}

fn kruskal(n: usize, edges: &mut Vec<(u32, usize, usize)>) -> (u32, Vec<(usize, usize)>) {
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut Vec<usize>, x: usize) -> usize { if p[x] != x { let r = find(p, p[x]); p[x] = r; } p[x] }
    edges.sort();
    let mut total = 0;
    let mut used = vec![];
    for &(w, a, b) in edges.iter() {
        let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
        if ra != rb { parent[ra] = rb; total += w; used.push((a, b)); }
    }
    (total, used)
}

fn main() {
    let g = AdjList::from_edges(&[(1, 2), (2, 3), (3, 1), (3, 4), (4, 5), (5, 6), (6, 4), (7, 6), (7, 8), (8, 7), (9, 9)]);
    let sccs = Tarjan::run(&g);
    println!("sccs: {:?}", sccs);
    let comp_of: BTreeMap<u32, usize> = sccs.iter().enumerate().flat_map(|(i, c)| c.iter().map(move |n| (*n, i))).collect();
    let dag_edges: BTreeSet<(usize, usize)> = g.nodes().iter().flat_map(|&n| g.neighbors(n).into_iter().map(move |m| (n, m))).map(|(a, b)| (comp_of[&a], comp_of[&b])).filter(|(a, b)| a != b).collect();
    let dag = AdjList::from_edges(&dag_edges.iter().map(|&(a, b)| (a as u32, b as u32)).collect::<Vec<_>>());
    println!("condensation edges {:?} topo {:?}", dag_edges, topo(&dag));
    println!("topo of cyclic graph: {:?}", topo(&g));
    println!("bfs from 1: {:?}", bfs_layers(&g, 1));
    let und = AdjList::from_edges(&[(1, 2), (2, 1), (2, 3), (3, 2), (3, 4), (4, 3), (4, 1), (1, 4)]);
    let tri = AdjList::from_edges(&[(1, 2), (2, 1), (2, 3), (3, 2), (3, 1), (1, 3)]);
    println!("bipartite square {:?} triangle {:?}", bipartite(&und), bipartite(&tri));

    let walls: BTreeSet<(i32, i32)> = [(0, 1), (1, 1), (2, 3), (3, 1), (3, 2), (3, 3)].into_iter().collect();
    let grid = Grid { w: 5, h: 5, walls };
    let layers = bfs_layers(&grid, (0, 0));
    for (d, ns) in &layers { println!("  dist {}: {:?}", d, ns); }
    println!("grid nodes {} sccs {} bipartite {:?}", grid.nodes().len(), Tarjan::run(&grid).len(), bipartite(&grid));
    let mut edges = vec![(7, 0, 1), (5, 0, 3), (8, 1, 2), (9, 1, 3), (7, 1, 4), (5, 2, 4), (15, 3, 4), (6, 3, 5), (8, 4, 5), (9, 4, 6), (11, 5, 6)];
    let (w, used) = kruskal(7, &mut edges);
    println!("mst weight {} edges {:?}", w, used);
}
