// Graph algorithms on generated graphs: Dijkstra, A* on a grid, Dinic max-flow,
// Kruskal with union-find, Tarjan SCC, topological sort, a generic segment tree,
// a Fenwick tree and LCA by binary lifting -- all cross-checked.
use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};
use std::fmt::Debug;

struct Lcg(u64);
impl Lcg {
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.0 >> 33) as u32
    }
    fn below(&mut self, n: usize) -> usize {
        self.next_u32() as usize % n
    }
}

#[derive(Clone, Debug)]
struct Graph<W> {
    adj: Vec<Vec<(usize, W)>>,
}

impl<W: Copy + Ord + Default + std::ops::Add<Output = W> + Debug> Graph<W> {
    fn new(n: usize) -> Self {
        Graph { adj: vec![Vec::new(); n] }
    }
    fn add_edge(&mut self, a: usize, b: usize, w: W) {
        self.adj[a].push((b, w));
    }
    fn n(&self) -> usize {
        self.adj.len()
    }
    fn edges(&self) -> impl Iterator<Item = (usize, usize, W)> + '_ {
        self.adj.iter().enumerate().flat_map(|(a, es)| es.iter().map(move |&(b, w)| (a, b, w)))
    }
    fn dijkstra(&self, src: usize) -> (Vec<Option<W>>, Vec<Option<usize>>) {
        let mut dist: Vec<Option<W>> = vec![None; self.n()];
        let mut prev = vec![None; self.n()];
        let mut pq = BinaryHeap::new();
        dist[src] = Some(W::default());
        pq.push(Reverse((W::default(), src)));
        while let Some(Reverse((d, u))) = pq.pop() {
            if dist[u].map_or(false, |best| d > best) {
                continue;
            }
            for &(v, w) in &self.adj[u] {
                let nd = d + w;
                if dist[v].map_or(true, |cur| nd < cur) {
                    dist[v] = Some(nd);
                    prev[v] = Some(u);
                    pq.push(Reverse((nd, v)));
                }
            }
        }
        (dist, prev)
    }
    fn bellman_ford(&self, src: usize) -> Vec<Option<W>> {
        let mut dist = vec![None; self.n()];
        dist[src] = Some(W::default());
        for _ in 0..self.n() {
            let mut changed = false;
            for (a, b, w) in self.edges() {
                if let Some(da) = dist[a] {
                    let nd = da + w;
                    if dist[b].map_or(true, |db| nd < db) {
                        dist[b] = Some(nd);
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        dist
    }
}

fn path_to(prev: &[Option<usize>], mut t: usize) -> Vec<usize> {
    let mut p = vec![t];
    while let Some(u) = prev[t] {
        p.push(u);
        t = u;
    }
    p.reverse();
    p
}

// ---------------- A* on a grid ----------------
fn astar(grid: &[Vec<u8>], start: (usize, usize), goal: (usize, usize)) -> Option<(u32, usize)> {
    let h = |(r, c): (usize, usize)| (r.abs_diff(goal.0) + c.abs_diff(goal.1)) as u32;
    let (rows, cols) = (grid.len(), grid[0].len());
    let mut g: HashMap<(usize, usize), u32> = HashMap::new();
    let mut open = BinaryHeap::new();
    let mut closed = HashSet::new();
    g.insert(start, 0);
    open.push(Reverse((h(start), 0u32, start)));
    while let Some(Reverse((_, cost, cur))) = open.pop() {
        if cur == goal {
            return Some((cost, closed.len()));
        }
        if !closed.insert(cur) {
            continue;
        }
        let (r, c) = cur;
        let dirs: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for (dr, dc) in dirs {
            let (nr, nc) = (r as isize + dr, c as isize + dc);
            if nr < 0 || nc < 0 || nr as usize >= rows || nc as usize >= cols {
                continue;
            }
            let nxt = (nr as usize, nc as usize);
            let cell = grid[nxt.0][nxt.1];
            if cell == b'#' {
                continue;
            }
            let step = if cell == b'~' { 5 } else { 1 };
            let ng = cost + step;
            if g.get(&nxt).map_or(true, |&old| ng < old) {
                g.insert(nxt, ng);
                open.push(Reverse((ng + h(nxt), ng, nxt)));
            }
        }
    }
    None
}

// ---------------- Dinic max flow ----------------
struct FlowEdge {
    to: usize,
    cap: i64,
    rev: usize,
}

struct Dinic {
    g: Vec<Vec<FlowEdge>>,
    level: Vec<i32>,
    it: Vec<usize>,
}

impl Dinic {
    fn new(n: usize) -> Self {
        Dinic { g: (0..n).map(|_| Vec::new()).collect(), level: vec![0; n], it: vec![0; n] }
    }
    fn add(&mut self, a: usize, b: usize, cap: i64) {
        let ra = self.g[b].len();
        let rb = self.g[a].len();
        self.g[a].push(FlowEdge { to: b, cap, rev: ra });
        self.g[b].push(FlowEdge { to: a, cap: 0, rev: rb });
    }
    fn bfs(&mut self, s: usize) {
        self.level.iter_mut().for_each(|l| *l = -1);
        let mut q = VecDeque::from([s]);
        self.level[s] = 0;
        while let Some(u) = q.pop_front() {
            for e in &self.g[u] {
                if e.cap > 0 && self.level[e.to] < 0 {
                    self.level[e.to] = self.level[u] + 1;
                    q.push_back(e.to);
                }
            }
        }
    }
    fn dfs(&mut self, u: usize, t: usize, f: i64) -> i64 {
        if u == t {
            return f;
        }
        while self.it[u] < self.g[u].len() {
            let i = self.it[u];
            let (to, cap) = (self.g[u][i].to, self.g[u][i].cap);
            if cap > 0 && self.level[u] < self.level[to] {
                let d = self.dfs(to, t, f.min(cap));
                if d > 0 {
                    self.g[u][i].cap -= d;
                    let r = self.g[u][i].rev;
                    self.g[to][r].cap += d;
                    return d;
                }
            }
            self.it[u] += 1;
        }
        0
    }
    fn max_flow(&mut self, s: usize, t: usize) -> i64 {
        let mut flow = 0;
        loop {
            self.bfs(s);
            if self.level[t] < 0 {
                return flow;
            }
            self.it.iter_mut().for_each(|x| *x = 0);
            loop {
                let f = self.dfs(s, t, i64::MAX);
                if f == 0 {
                    break;
                }
                flow += f;
            }
        }
    }
    fn min_cut(&self, s: usize) -> Vec<usize> {
        let mut seen = vec![false; self.g.len()];
        let mut stack = vec![s];
        seen[s] = true;
        while let Some(u) = stack.pop() {
            for e in self.g[u].iter().filter(|e| e.cap > 0) {
                if !std::mem::replace(&mut seen[e.to], true) {
                    stack.push(e.to);
                }
            }
        }
        seen.iter().enumerate().filter(|(_, &b)| b).map(|(i, _)| i).collect()
    }
}

// ---------------- union find + kruskal ----------------
struct Dsu {
    parent: Vec<usize>,
    size: Vec<usize>,
}

impl Dsu {
    fn new(n: usize) -> Self {
        Dsu { parent: (0..n).collect(), size: vec![1; n] }
    }
    fn find(&mut self, x: usize) -> usize {
        let p = self.parent[x];
        if p == x {
            return x;
        }
        let r = self.find(p);
        self.parent[x] = r;
        r
    }
    fn union(&mut self, a: usize, b: usize) -> bool {
        let (mut a, mut b) = (self.find(a), self.find(b));
        if a == b {
            return false;
        }
        if self.size[a] < self.size[b] {
            std::mem::swap(&mut a, &mut b);
        }
        self.parent[b] = a;
        self.size[a] += self.size[b];
        true
    }
}

fn kruskal(n: usize, edges: &[(u32, usize, usize)]) -> (u64, Vec<(usize, usize)>) {
    let mut es = edges.to_vec();
    es.sort_unstable();
    let mut d = Dsu::new(n);
    let mut total = 0u64;
    let mut used = Vec::new();
    for (w, a, b) in es {
        if d.union(a, b) {
            total += w as u64;
            used.push((a.min(b), a.max(b)));
        }
    }
    (total, used)
}

fn prim(n: usize, edges: &[(u32, usize, usize)]) -> u64 {
    let mut adj = vec![vec![]; n];
    for &(w, a, b) in edges {
        adj[a].push((w, b));
        adj[b].push((w, a));
    }
    let mut in_tree = vec![false; n];
    let mut pq = BinaryHeap::from(vec![Reverse((0u32, 0usize))]);
    let mut total = 0u64;
    while let Some(Reverse((w, u))) = pq.pop() {
        if in_tree[u] {
            continue;
        }
        in_tree[u] = true;
        total += w as u64;
        pq.extend(adj[u].iter().filter(|(_, v)| !in_tree[*v]).map(|&(w, v)| Reverse((w, v))));
    }
    total
}

// ---------------- Tarjan SCC + topo sort ----------------
fn tarjan(adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    struct St<'g> {
        adj: &'g [Vec<usize>],
        idx: Vec<Option<usize>>,
        low: Vec<usize>,
        on: Vec<bool>,
        stack: Vec<usize>,
        next: usize,
        out: Vec<Vec<usize>>,
    }
    fn go(s: &mut St, v: usize) {
        s.idx[v] = Some(s.next);
        s.low[v] = s.next;
        s.next += 1;
        s.stack.push(v);
        s.on[v] = true;
        for &w in s.adj[v].iter() {
            match s.idx[w] {
                None => {
                    go(s, w);
                    s.low[v] = s.low[v].min(s.low[w]);
                }
                Some(iw) if s.on[w] => s.low[v] = s.low[v].min(iw),
                _ => {}
            }
        }
        if Some(s.low[v]) == s.idx[v] {
            let mut comp = Vec::new();
            loop {
                let w = s.stack.pop().unwrap();
                s.on[w] = false;
                comp.push(w);
                if w == v {
                    break;
                }
            }
            comp.sort();
            s.out.push(comp);
        }
    }
    let n = adj.len();
    let mut s = St { adj, idx: vec![None; n], low: vec![0; n], on: vec![false; n], stack: vec![], next: 0, out: vec![] };
    for v in 0..n {
        if s.idx[v].is_none() {
            go(&mut s, v);
        }
    }
    s.out
}

fn topo(adj: &[Vec<usize>]) -> Result<Vec<usize>, usize> {
    let n = adj.len();
    let mut indeg = vec![0; n];
    adj.iter().flatten().for_each(|&v| indeg[v] += 1);
    let mut ready: BTreeSet<usize> = (0..n).filter(|&v| indeg[v] == 0).collect();
    let mut order = Vec::with_capacity(n);
    while let Some(&u) = ready.iter().next() {
        ready.remove(&u);
        order.push(u);
        for &v in &adj[u] {
            indeg[v] -= 1;
            if indeg[v] == 0 {
                ready.insert(v);
            }
        }
    }
    if order.len() == n {
        Ok(order)
    } else {
        Err(n - order.len())
    }
}

// ---------------- segment tree (generic monoid) ----------------
trait Monoid {
    type T: Copy + Debug + PartialEq;
    const ID: Self::T;
    fn op(a: Self::T, b: Self::T) -> Self::T;
}
struct MinM;
impl Monoid for MinM {
    type T = i64;
    const ID: i64 = i64::MAX;
    fn op(a: i64, b: i64) -> i64 {
        a.min(b)
    }
}
struct SumM;
impl Monoid for SumM {
    type T = i64;
    const ID: i64 = 0;
    fn op(a: i64, b: i64) -> i64 {
        a + b
    }
}
struct GcdM;
impl Monoid for GcdM {
    type T = u64;
    const ID: u64 = 0;
    fn op(mut a: u64, mut b: u64) -> u64 {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    }
}

struct SegTree<M: Monoid> {
    n: usize,
    t: Vec<M::T>,
}

impl<M: Monoid> SegTree<M> {
    fn from(xs: &[M::T]) -> Self {
        let n = xs.len().next_power_of_two();
        let mut t = vec![M::ID; 2 * n];
        t[n..n + xs.len()].copy_from_slice(xs);
        for i in (1..n).rev() {
            t[i] = M::op(t[2 * i], t[2 * i + 1]);
        }
        SegTree { n, t }
    }
    fn set(&mut self, mut i: usize, v: M::T) {
        i += self.n;
        self.t[i] = v;
        while i > 1 {
            i /= 2;
            self.t[i] = M::op(self.t[2 * i], self.t[2 * i + 1]);
        }
    }
    fn query(&self, l: usize, r: usize) -> M::T {
        let (mut l, mut r) = (l + self.n, r + self.n);
        let (mut al, mut ar) = (M::ID, M::ID);
        while l < r {
            if l & 1 == 1 {
                al = M::op(al, self.t[l]);
                l += 1;
            }
            if r & 1 == 1 {
                r -= 1;
                ar = M::op(self.t[r], ar);
            }
            l >>= 1;
            r >>= 1;
        }
        M::op(al, ar)
    }
}

struct Fenwick(Vec<i64>);
impl Fenwick {
    fn add(&mut self, mut i: usize, d: i64) {
        i += 1;
        while i <= self.0.len() {
            self.0[i - 1] += d;
            i += i & i.wrapping_neg();
        }
    }
    fn prefix(&self, mut i: usize) -> i64 {
        let mut s = 0;
        while i > 0 {
            s += self.0[i - 1];
            i &= i - 1;
        }
        s
    }
}

// ---------------- LCA ----------------
struct Lca {
    up: Vec<Vec<usize>>,
    depth: Vec<usize>,
}
impl Lca {
    fn new(parent: &[usize], root: usize) -> Self {
        let n = parent.len();
        let lg = (usize::BITS - n.leading_zeros()) as usize + 1;
        let mut children = vec![vec![]; n];
        for (v, &p) in parent.iter().enumerate() {
            if v != root {
                children[p].push(v);
            }
        }
        let mut depth = vec![0; n];
        let mut order = vec![root];
        let mut i = 0;
        while i < order.len() {
            let u = order[i];
            for &c in &children[u] {
                depth[c] = depth[u] + 1;
                order.push(c);
            }
            i += 1;
        }
        let mut up = vec![parent.to_vec()];
        up[0][root] = root;
        for k in 1..lg {
            let prev = &up[k - 1];
            let row: Vec<usize> = (0..n).map(|v| prev[prev[v]]).collect();
            up.push(row);
        }
        Lca { up, depth }
    }
    fn query(&self, mut a: usize, mut b: usize) -> usize {
        if self.depth[a] < self.depth[b] {
            std::mem::swap(&mut a, &mut b);
        }
        let diff = self.depth[a] - self.depth[b];
        for k in 0..self.up.len() {
            if diff >> k & 1 == 1 {
                a = self.up[k][a];
            }
        }
        if a == b {
            return a;
        }
        for k in (0..self.up.len()).rev() {
            if self.up[k][a] != self.up[k][b] {
                a = self.up[k][a];
                b = self.up[k][b];
            }
        }
        self.up[0][a]
    }
    fn naive(&self, parent: &[usize], mut a: usize, mut b: usize) -> usize {
        while self.depth[a] > self.depth[b] {
            a = parent[a];
        }
        while self.depth[b] > self.depth[a] {
            b = parent[b];
        }
        while a != b {
            a = parent[a];
            b = parent[b];
        }
        a
    }
}

fn main() {
    let mut rng = Lcg(2024);
    // shortest paths
    let n = 60;
    let mut g: Graph<u64> = Graph::new(n);
    for _ in 0..240 {
        let (a, b) = (rng.below(n), rng.below(n));
        g.add_edge(a, b, 1 + rng.below(100) as u64);
    }
    let (dist, prev) = g.dijkstra(0);
    let bf = g.bellman_ford(0);
    assert_eq!(dist, bf);
    let reach = dist.iter().filter(|d| d.is_some()).count();
    let far = (0..n).filter(|&v| dist[v].is_some()).max_by(|&a, &b| dist[a].cmp(&dist[b]).then(b.cmp(&a))).unwrap();
    println!("dijkstra: reachable {}/{}, farthest {} at {:?}, path {:?}", reach, n, far, dist[far], path_to(&prev, far));
    let total: u64 = dist.iter().flatten().sum();
    println!("sum of distances {}, edges {}", total, g.edges().count());
    let gi: Graph<i32> = {
        let mut gi = Graph::new(5);
        for (a, b, w) in [(0, 1, 4), (0, 2, 1), (2, 1, -2), (1, 3, 1), (2, 3, 5), (3, 4, 3)] {
            gi.add_edge(a, b, w);
        }
        gi
    };
    println!("bellman-ford with negative edge: {:?}", gi.bellman_ford(0));

    // A*
    let maze_src = [
        "S....#........~~~.....",
        ".###.#.######.~#~.###.",
        ".#...#......#.~#~...#.",
        ".#.#####.##.#.~#~##.#.",
        ".#.....#..#.#..#....#.",
        ".#####.##.#.####.####.",
        ".....#....#......#...G",
    ];
    let maze: Vec<Vec<u8>> = maze_src.iter().map(|r| r.bytes().collect()).collect();
    let find = |ch: u8| -> (usize, usize) {
        maze.iter().enumerate().find_map(|(r, row)| row.iter().position(|&c| c == ch).map(|c| (r, c))).unwrap()
    };
    println!("A*: {:?}", astar(&maze, find(b'S'), find(b'G')));
    let mut blocked = maze.clone();
    blocked[5][21] = b'#';
    blocked[6][20] = b'#';
    println!("A* blocked: {:?}", astar(&blocked, find(b'S'), find(b'G')));

    // max flow on a layered random network, verified by min-cut capacity
    let layers = [1usize, 6, 8, 6, 1];
    let mut ids = vec![];
    let mut next = 0;
    for &l in &layers {
        ids.push((next..next + l).collect::<Vec<_>>());
        next += l;
    }
    let mut din = Dinic::new(next);
    let mut caps = vec![];
    for w in ids.windows(2) {
        for &a in &w[0] {
            for &b in &w[1] {
                if rng.below(3) != 0 {
                    let c = 1 + rng.below(20) as i64;
                    din.add(a, b, c);
                    caps.push((a, b, c));
                }
            }
        }
    }
    let flow = din.max_flow(0, next - 1);
    let side: HashSet<usize> = din.min_cut(0).into_iter().collect();
    let cut: i64 = caps.iter().filter(|(a, b, _)| side.contains(a) && !side.contains(b)).map(|e| e.2).sum();
    println!("max flow {} == min cut {}: {}", flow, cut, flow == cut);

    // MST
    let m = 40;
    let mut es = vec![];
    for i in 1..m {
        es.push((rng.below(1000) as u32, i, rng.below(i)));
    }
    for _ in 0..80 {
        let (a, b) = (rng.below(m), rng.below(m));
        if a != b {
            es.push((rng.below(1000) as u32, a, b));
        }
    }
    let (kw, used) = kruskal(m, &es);
    println!("kruskal {} prim {} edges {} first {:?}", kw, prim(m, &es), used.len(), &used[..3]);

    // SCC + topo
    let k = 16;
    let mut adj = vec![vec![]; k];
    for _ in 0..24 {
        let (a, b) = (rng.below(k), rng.below(k));
        if a != b && !adj[a].contains(&b) {
            adj[a].push(b);
        }
    }
    let sccs = tarjan(&adj);
    let mut big: Vec<&Vec<usize>> = sccs.iter().filter(|c| c.len() > 1).collect();
    big.sort();
    println!("{} sccs, nontrivial: {:?}", sccs.len(), big);
    println!("topo: {:?}", topo(&adj));
    let dag: Vec<Vec<usize>> = (0..k).map(|i| adj[i].iter().copied().filter(|&j| j > i).collect()).collect();
    let order = topo(&dag).unwrap();
    let pos: HashMap<usize, usize> = order.iter().enumerate().map(|(i, &v)| (v, i)).collect();
    println!("dag topo ok: {} {:?}", dag.iter().enumerate().all(|(a, bs)| bs.iter().all(|b| pos[&a] < pos[b])), order);

    // segment trees vs naive
    let xs: Vec<i64> = (0..37).map(|_| rng.below(1000) as i64 - 500).collect();
    let mut st_min = SegTree::<MinM>::from(&xs);
    let mut st_sum = SegTree::<SumM>::from(&xs);
    let mut fw = Fenwick(vec![0; xs.len()]);
    xs.iter().enumerate().for_each(|(i, &x)| fw.add(i, x));
    let mut ys = xs.clone();
    let mut checks = 0;
    for step in 0..300 {
        if step % 3 == 0 {
            let i = rng.below(ys.len());
            let v = rng.below(1000) as i64 - 500;
            fw.add(i, v - ys[i]);
            ys[i] = v;
            st_min.set(i, v);
            st_sum.set(i, v);
        }
        let l = rng.below(ys.len());
        let r = l + 1 + rng.below(ys.len() - l);
        assert_eq!(st_min.query(l, r), *ys[l..r].iter().min().unwrap());
        assert_eq!(st_sum.query(l, r), ys[l..r].iter().sum::<i64>());
        assert_eq!(fw.prefix(r) - fw.prefix(l), ys[l..r].iter().sum::<i64>());
        checks += 1;
    }
    let gs = SegTree::<GcdM>::from(&[12, 18, 24, 36, 7, 14, 21]);
    println!("segtree checks {}, gcd[0..4]={} gcd[4..7]={} gcd all={}", checks, gs.query(0, 4), gs.query(4, 7), gs.query(0, 7));
    println!("min ID {:?} sum ID {:?}", <MinM as Monoid>::ID, SumM::ID);

    // LCA on random tree
    let tn = 200;
    let parent: Vec<usize> = (0..tn).map(|v| if v == 0 { 0 } else { rng.below(v) }).collect();
    let lca = Lca::new(&parent, 0);
    let mut agree = 0;
    let mut sample = vec![];
    for _ in 0..500 {
        let (a, b) = (rng.below(tn), rng.below(tn));
        let x = lca.query(a, b);
        if x == lca.naive(&parent, a, b) {
            agree += 1;
        }
        if sample.len() < 5 {
            sample.push((a, b, x));
        }
    }
    println!("lca agree {}/500, max depth {}, sample {:?}", agree, lca.depth.iter().max().unwrap(), sample);
    let mut ord: Vec<(usize, usize)> = (0..10).map(|v| (lca.depth[v], v)).collect();
    ord.sort_by(|a, b| match b.0.cmp(&a.0) {
        Ordering::Equal => a.1.cmp(&b.1),
        o => o,
    });
    println!("{:?}", ord);
}
