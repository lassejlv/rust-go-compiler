// Classic graph algorithms over a generic adjacency-list graph:
// Tarjan SCC, Kahn topological sort, Kruskal MST with union-find (path
// compression + union by rank), Prim MST, Edmonds-Karp max flow with
// min-cut, bipartite check, articulation points/bridges, and Euler tour.
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, VecDeque};
use std::cmp::Reverse;
use std::hash::Hash;
use std::fmt::Debug;

struct Graph<N> {
    nodes: Vec<N>,
    index: HashMap<N, usize>,
    adj: Vec<Vec<(usize, i64)>>,
    directed: bool,
}

impl<N: Clone + Eq + Hash + Debug + Ord> Graph<N> {
    fn new(directed: bool) -> Self { Graph { nodes: vec![], index: HashMap::new(), adj: vec![], directed } }
    fn node(&mut self, n: N) -> usize {
        if let Some(&i) = self.index.get(&n) { return i; }
        self.nodes.push(n.clone());
        self.adj.push(vec![]);
        self.index.insert(n, self.nodes.len() - 1);
        self.nodes.len() - 1
    }
    fn edge(&mut self, a: N, b: N, w: i64) {
        let (x, y) = (self.node(a), self.node(b));
        self.adj[x].push((y, w));
        if !self.directed { self.adj[y].push((x, w)); }
    }
    fn names(&self, ids: &[usize]) -> Vec<N> { ids.iter().map(|&i| self.nodes[i].clone()).collect() }

    fn tarjan_scc(&self) -> Vec<Vec<N>> {
        struct St { idx: usize, index: Vec<Option<usize>>, low: Vec<usize>, on: Vec<bool>, stack: Vec<usize>, out: Vec<Vec<usize>> }
        fn strong<N>(g: &Graph<N>, v: usize, s: &mut St) {
            s.index[v] = Some(s.idx);
            s.low[v] = s.idx;
            s.idx += 1;
            s.stack.push(v);
            s.on[v] = true;
            for &(w, _) in &g.adj[v] {
                match s.index[w] {
                    None => { strong(g, w, s); s.low[v] = s.low[v].min(s.low[w]); }
                    Some(iw) if s.on[w] => s.low[v] = s.low[v].min(iw),
                    _ => {}
                }
            }
            if Some(s.low[v]) == s.index[v] {
                let mut comp = vec![];
                loop {
                    let w = s.stack.pop().unwrap();
                    s.on[w] = false;
                    comp.push(w);
                    if w == v { break; }
                }
                s.out.push(comp);
            }
        }
        let n = self.nodes.len();
        let mut s = St { idx: 0, index: vec![None; n], low: vec![0; n], on: vec![false; n], stack: vec![], out: vec![] };
        for v in 0..n { if s.index[v].is_none() { strong(self, v, &mut s); } }
        let mut comps: Vec<Vec<N>> = s.out.iter().map(|c| { let mut v = self.names(c); v.sort(); v }).collect();
        comps.sort();
        comps
    }

    fn topo(&self) -> Result<Vec<N>, Vec<N>> {
        let n = self.nodes.len();
        let mut indeg = vec![0; n];
        for v in 0..n { for &(w, _) in &self.adj[v] { indeg[w] += 1; } }
        // use a min-heap on names for deterministic order
        let mut q: BinaryHeap<Reverse<(N, usize)>> = (0..n).filter(|&v| indeg[v] == 0).map(|v| Reverse((self.nodes[v].clone(), v))).collect();
        let mut order = vec![];
        while let Some(Reverse((_, v))) = q.pop() {
            order.push(v);
            for &(w, _) in &self.adj[v] {
                indeg[w] -= 1;
                if indeg[w] == 0 { q.push(Reverse((self.nodes[w].clone(), w))); }
            }
        }
        if order.len() == n { Ok(self.names(&order)) } else {
            let mut stuck: Vec<N> = (0..n).filter(|&v| indeg[v] > 0).map(|v| self.nodes[v].clone()).collect();
            stuck.sort();
            Err(stuck)
        }
    }

    fn edges(&self) -> Vec<(i64, usize, usize)> {
        let mut e = vec![];
        for v in 0..self.adj.len() { for &(w, wt) in &self.adj[v] { if self.directed || v < w { e.push((wt, v, w)); } } }
        e
    }

    fn kruskal(&self) -> (i64, Vec<(N, N, i64)>) {
        let mut uf = UnionFind::new(self.nodes.len());
        let mut e = self.edges();
        e.sort();
        let mut total = 0;
        let mut chosen = vec![];
        for (w, a, b) in e {
            if uf.union(a, b) {
                total += w;
                chosen.push((self.nodes[a].clone(), self.nodes[b].clone(), w));
            }
        }
        (total, chosen)
    }

    fn prim(&self, start: usize) -> i64 {
        let mut seen = vec![false; self.nodes.len()];
        let mut heap = BinaryHeap::new();
        heap.push(Reverse((0i64, start)));
        let mut total = 0;
        while let Some(Reverse((w, v))) = heap.pop() {
            if seen[v] { continue; }
            seen[v] = true;
            total += w;
            for &(u, wu) in &self.adj[v] { if !seen[u] { heap.push(Reverse((wu, u))); } }
        }
        total
    }

    fn bipartite(&self) -> Result<(Vec<N>, Vec<N>), (N, N)> {
        let n = self.nodes.len();
        let mut color: Vec<Option<bool>> = vec![None; n];
        for s in 0..n {
            if color[s].is_some() { continue; }
            color[s] = Some(false);
            let mut q = VecDeque::from([s]);
            while let Some(v) = q.pop_front() {
                for &(w, _) in &self.adj[v] {
                    match color[w] {
                        None => { color[w] = Some(!color[v].unwrap()); q.push_back(w); }
                        Some(c) if c == color[v].unwrap() => return Err((self.nodes[v].clone(), self.nodes[w].clone())),
                        _ => {}
                    }
                }
            }
        }
        let (mut a, mut b): (Vec<N>, Vec<N>) = (vec![], vec![]);
        for v in 0..n { if color[v] == Some(false) { a.push(self.nodes[v].clone()) } else { b.push(self.nodes[v].clone()) } }
        a.sort();
        b.sort();
        Ok((a, b))
    }

    fn articulation(&self) -> (BTreeSet<N>, BTreeSet<(N, N)>) {
        let n = self.nodes.len();
        let mut disc = vec![usize::MAX; n];
        let mut low = vec![0; n];
        let mut timer = 0;
        let mut aps = BTreeSet::new();
        let mut bridges = BTreeSet::new();
        fn dfs<N: Clone + Ord>(g: &Graph<N>, v: usize, parent: Option<usize>, t: &mut usize, disc: &mut Vec<usize>, low: &mut Vec<usize>, aps: &mut BTreeSet<N>, br: &mut BTreeSet<(N, N)>) {
            disc[v] = *t;
            low[v] = *t;
            *t += 1;
            let mut children = 0;
            for &(w, _) in &g.adj[v] {
                if Some(w) == parent { continue; }
                if disc[w] == usize::MAX {
                    children += 1;
                    dfs(g, w, Some(v), t, disc, low, aps, br);
                    low[v] = low[v].min(low[w]);
                    if parent.is_some() && low[w] >= disc[v] { aps.insert(g.nodes[v].clone()); }
                    if low[w] > disc[v] {
                        let (a, b) = (g.nodes[v].clone(), g.nodes[w].clone());
                        br.insert(if a < b { (a, b) } else { (b, a) });
                    }
                } else {
                    low[v] = low[v].min(disc[w]);
                }
            }
            if parent.is_none() && children > 1 { aps.insert(g.nodes[v].clone()); }
        }
        for v in 0..n { if disc[v] == usize::MAX { dfs(self, v, None, &mut timer, &mut disc, &mut low, &mut aps, &mut bridges); } }
        (aps, bridges)
    }
}

struct UnionFind { parent: Vec<usize>, rank: Vec<u8>, sets: usize }
impl UnionFind {
    fn new(n: usize) -> Self { UnionFind { parent: (0..n).collect(), rank: vec![0; n], sets: n } }
    fn find(&mut self, x: usize) -> usize {
        let mut r = x;
        while self.parent[r] != r { r = self.parent[r]; }
        let mut c = x;
        while self.parent[c] != r { let next = self.parent[c]; self.parent[c] = r; c = next; }
        r
    }
    fn union(&mut self, a: usize, b: usize) -> bool {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb { return false; }
        match self.rank[ra].cmp(&self.rank[rb]) {
            std::cmp::Ordering::Less => self.parent[ra] = rb,
            std::cmp::Ordering::Greater => self.parent[rb] = ra,
            std::cmp::Ordering::Equal => { self.parent[rb] = ra; self.rank[ra] += 1; }
        }
        self.sets -= 1;
        true
    }
}

fn max_flow(n: usize, edges: &[(usize, usize, i64)], s: usize, t: usize) -> (i64, Vec<usize>, usize) {
    let mut cap = vec![vec![0i64; n]; n];
    for &(a, b, c) in edges { cap[a][b] += c; }
    let mut flow = 0;
    let mut augmentations = 0;
    loop {
        let mut prev = vec![usize::MAX; n];
        prev[s] = s;
        let mut q = VecDeque::from([s]);
        while let Some(v) = q.pop_front() {
            for w in 0..n {
                if prev[w] == usize::MAX && cap[v][w] > 0 { prev[w] = v; q.push_back(w); }
            }
        }
        if prev[t] == usize::MAX {
            let cut: Vec<usize> = (0..n).filter(|&v| prev[v] != usize::MAX).collect();
            return (flow, cut, augmentations);
        }
        let mut bottleneck = i64::MAX;
        let mut v = t;
        while v != s { bottleneck = bottleneck.min(cap[prev[v]][v]); v = prev[v]; }
        let mut v = t;
        while v != s { cap[prev[v]][v] -= bottleneck; cap[v][prev[v]] += bottleneck; v = prev[v]; }
        flow += bottleneck;
        augmentations += 1;
    }
}

fn euler_path(edges: &[(char, char)]) -> Option<String> {
    let mut adj: BTreeMap<char, Vec<char>> = BTreeMap::new();
    let mut deg: BTreeMap<char, i32> = BTreeMap::new();
    for &(a, b) in edges {
        adj.entry(a).or_default().push(b);
        adj.entry(b).or_default().push(a);
        *deg.entry(a).or_default() += 1;
        *deg.entry(b).or_default() += 1;
    }
    for v in adj.values_mut() { v.sort_unstable_by(|a, b| b.cmp(a)); }
    let odd: Vec<char> = deg.iter().filter(|(_, &d)| d % 2 == 1).map(|(&c, _)| c).collect();
    let start = match odd.len() { 0 => *adj.keys().next()?, 2 => odd[0], _ => return None };
    let mut used: HashMap<(char, char), usize> = HashMap::new();
    for &(a, b) in edges { *used.entry((a.min(b), a.max(b))).or_default() += 1; }
    let mut stack = vec![start];
    let mut path = String::new();
    while let Some(&v) = stack.last() {
        let next = loop {
            match adj.get_mut(&v).and_then(|l| l.pop()) {
                None => break None,
                Some(w) => {
                    let k = (v.min(w), v.max(w));
                    if let Some(c) = used.get_mut(&k).filter(|c| **c > 0) { *c -= 1; break Some(w); }
                }
            }
        };
        match next { Some(w) => stack.push(w), None => { path.push(v); stack.pop(); } }
    }
    if path.len() == edges.len() + 1 { Some(path.chars().rev().collect()) } else { None }
}

fn main() {
    let mut g: Graph<&str> = Graph::new(true);
    for (a, b) in [("a", "b"), ("b", "c"), ("c", "a"), ("b", "d"), ("d", "e"), ("e", "f"), ("f", "d"), ("g", "f"), ("g", "h"), ("h", "i"), ("i", "g"), ("j", "j")] {
        g.edge(a, b, 1);
    }
    println!("SCCs: {:?}", g.tarjan_scc());
    println!("topo (cyclic): {:?}", g.topo());

    let mut dag: Graph<String> = Graph::new(true);
    let deps = "std:core alloc:core std:alloc collections:alloc io:std net:io net:std fs:io app:net app:fs app:collections test:app";
    for pair in deps.split(' ') {
        let (a, b) = pair.split_once(':').unwrap();
        dag.edge(b.to_string(), a.to_string(), 1);
    }
    println!("build order: {:?}", dag.topo());

    let mut w: Graph<char> = Graph::new(false);
    for (a, b, c) in [('A', 'B', 7), ('A', 'D', 5), ('B', 'C', 8), ('B', 'D', 9), ('B', 'E', 7), ('C', 'E', 5), ('D', 'E', 15), ('D', 'F', 6), ('E', 'F', 8), ('E', 'G', 9), ('F', 'G', 11)] {
        w.edge(a, b, c);
    }
    let (total, chosen) = w.kruskal();
    println!("kruskal: {} {:?}", total, chosen);
    println!("prim from each node: {:?}", (0..w.nodes.len()).map(|s| w.prim(s)).collect::<Vec<_>>());
    println!("bipartite: {:?}", w.bipartite());
    let mut grid: Graph<(i32, i32)> = Graph::new(false);
    for x in 0..3 { for y in 0..3 { if x < 2 { grid.edge((x, y), (x + 1, y), 1); } if y < 2 { grid.edge((x, y), (x, y + 1), 1); } } }
    println!("grid bipartite: {:?}", grid.bipartite().map(|(a, b)| (a.len(), b.len())));

    let mut net: Graph<u8> = Graph::new(false);
    for (a, b) in [(1, 2), (2, 3), (3, 1), (3, 4), (4, 5), (5, 6), (6, 4), (6, 7), (8, 9)] { net.edge(a, b, 1); }
    let (aps, bridges) = net.articulation();
    println!("articulation points: {:?}\nbridges: {:?}", aps, bridges);

    let edges = [(0, 1, 16), (0, 2, 13), (1, 2, 10), (2, 1, 4), (1, 3, 12), (3, 2, 9), (2, 4, 14), (4, 3, 7), (3, 5, 20), (4, 5, 4)];
    let (f, cut, aug) = max_flow(6, &edges, 0, 5);
    let cut_edges: Vec<_> = edges.iter().filter(|(a, b, _)| cut.contains(a) && !cut.contains(b)).collect();
    println!("max flow {} in {} augmentations, source side {:?}, cut edges {:?} (cap {})", f, aug, cut, cut_edges, cut_edges.iter().map(|e| e.2).sum::<i64>());

    let mut uf = UnionFind::new(10);
    for (a, b) in [(0, 1), (2, 3), (1, 3), (5, 6), (7, 8), (8, 9), (9, 7), (0, 2)] { uf.union(a, b); }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..10 { let r = uf.find(i); groups.entry(r).or_default().push(i); }
    let mut gs: Vec<Vec<usize>> = groups.into_values().collect();
    gs.sort();
    println!("union-find sets={} groups={:?}", uf.sets, gs);

    println!("euler (house): {:?}", euler_path(&[('a', 'b'), ('b', 'c'), ('c', 'd'), ('d', 'a'), ('a', 'c'), ('b', 'd'), ('c', 'e'), ('d', 'e')]));
    println!("euler (cycle): {:?}", euler_path(&[('x', 'y'), ('y', 'z'), ('z', 'x')]));
    println!("euler (none): {:?}", euler_path(&[('a', 'b'), ('a', 'c'), ('a', 'd')]));
}
