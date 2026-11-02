// Dijkstra shortest paths on a weighted directed graph parsed from text, with BinaryHeap<Reverse<..>>,
// path reconstruction via predecessor map, unreachable nodes, A* on a grid with Manhattan heuristic,
// Bellman-Ford with negative edges and negative-cycle detection, and Floyd-Warshall all-pairs.
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashMap};

struct Graph<'a> {
    names: Vec<&'a str>,
    idx: HashMap<&'a str, usize>,
    adj: Vec<Vec<(usize, i64)>>,
}

impl<'a> Graph<'a> {
    fn parse(src: &'a str) -> Graph<'a> {
        let mut g = Graph { names: vec![], idx: HashMap::new(), adj: vec![] };
        for line in src.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let [a, b, w] = parts[..] else { panic!("bad line {}", line) };
            let (ia, ib) = (g.id(a), g.id(b));
            g.adj[ia].push((ib, w.parse().unwrap()));
        }
        g
    }
    fn id(&mut self, n: &'a str) -> usize {
        if let Some(&i) = self.idx.get(n) {
            return i;
        }
        self.names.push(n);
        self.adj.push(vec![]);
        self.idx.insert(n, self.names.len() - 1);
        self.names.len() - 1
    }
    fn dijkstra(&self, src: usize) -> (Vec<Option<i64>>, Vec<Option<usize>>) {
        let n = self.names.len();
        let mut dist = vec![None; n];
        let mut prev = vec![None; n];
        let mut heap = BinaryHeap::new();
        dist[src] = Some(0);
        heap.push(Reverse((0i64, src)));
        while let Some(Reverse((d, u))) = heap.pop() {
            if dist[u].map_or(false, |best| d > best) {
                continue;
            }
            for &(v, w) in &self.adj[u] {
                let nd = d + w;
                if dist[v].map_or(true, |cur| nd < cur) {
                    dist[v] = Some(nd);
                    prev[v] = Some(u);
                    heap.push(Reverse((nd, v)));
                }
            }
        }
        (dist, prev)
    }
    fn path(&self, prev: &[Option<usize>], mut t: usize) -> String {
        let mut p = vec![self.names[t]];
        while let Some(u) = prev[t] {
            p.push(self.names[u]);
            t = u;
        }
        p.reverse();
        p.join("->")
    }
    fn bellman_ford(&self, src: usize) -> Result<Vec<Option<i64>>, String> {
        let n = self.names.len();
        let mut dist: Vec<Option<i64>> = vec![None; n];
        dist[src] = Some(0);
        for round in 0..n {
            let mut changed = false;
            for u in 0..n {
                let Some(du) = dist[u] else { continue };
                for &(v, w) in &self.adj[u] {
                    if dist[v].map_or(true, |dv| du + w < dv) {
                        dist[v] = Some(du + w);
                        changed = true;
                    }
                }
            }
            if !changed {
                return Ok(dist);
            }
            if round == n - 1 {
                return Err("negative cycle".into());
            }
        }
        Ok(dist)
    }
}

fn astar(grid: &[&str], start: (usize, usize), goal: (usize, usize)) -> Option<(usize, usize)> {
    let h = |p: (usize, usize)| p.0.abs_diff(goal.0) + p.1.abs_diff(goal.1);
    let mut open = BinaryHeap::new();
    let mut g: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    let mut expanded = 0;
    g.insert(start, 0);
    open.push(Reverse((h(start), 0usize, start)));
    while let Some(Reverse((_, cost, p))) = open.pop() {
        if p == goal {
            return Some((cost, expanded));
        }
        if cost > g[&p] {
            continue;
        }
        expanded += 1;
        let (r, c) = p;
        let cand = [(r.wrapping_sub(1), c), (r + 1, c), (r, c.wrapping_sub(1)), (r, c + 1)];
        for (nr, nc) in cand {
            let Some(row) = grid.get(nr) else { continue };
            if row.as_bytes().get(nc).map_or(true, |&b| b == b'#') {
                continue;
            }
            let nc_cost = cost + 1;
            if g.get(&(nr, nc)).map_or(true, |&old| nc_cost < old) {
                g.insert((nr, nc), nc_cost);
                open.push(Reverse((nc_cost + h((nr, nc)), nc_cost, (nr, nc))));
            }
        }
    }
    None
}

fn main() {
    let src = "
        # from to weight
        A B 4
        A C 2
        C B 1
        B D 5
        C D 8
        C E 10
        D E 2
        E F 3
        D F 6
        G A 1
    ";
    let g = Graph::parse(src);
    let a = g.idx["A"];
    let (dist, prev) = g.dijkstra(a);
    for (i, d) in dist.iter().enumerate() {
        match d {
            Some(d) => println!("{}: {:>3} via {}", g.names[i], d, g.path(&prev, i)),
            None => println!("{}: unreachable", g.names[i]),
        }
    }
    println!("{:?}", g.bellman_ford(a));
    let neg = Graph::parse("S A 4\nS B 5\nB A -3\nA C 2\nC D -1");
    println!("{:?}", neg.bellman_ford(0).map(|d| d.into_iter().map(|x| x.unwrap_or(i64::MAX)).collect::<Vec<_>>()));
    let cyc = Graph::parse("X Y 1\nY Z -2\nZ X 0");
    println!("{:?}", cyc.bellman_ford(0));

    let n = g.names.len();
    let mut fw = vec![vec![i64::MAX / 4; n]; n];
    for u in 0..n {
        fw[u][u] = 0;
        for &(v, w) in &g.adj[u] {
            fw[u][v] = fw[u][v].min(w);
        }
    }
    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                if fw[i][k] + fw[k][j] < fw[i][j] {
                    fw[i][j] = fw[i][k] + fw[k][j];
                }
            }
        }
    }
    print!("   ");
    for nm in &g.names {
        print!("{:>4}", nm);
    }
    println!();
    for i in 0..n {
        let row: String = fw[i].iter().map(|&d| if d >= i64::MAX / 4 { "   -".to_string() } else { format!("{:>4}", d) }).collect();
        println!("{:>3}{}", g.names[i], row);
    }

    let grid = ["S..#....", ".#.#.##.", ".#...#..", ".####.#.", "......#G"];
    println!("{:?}", astar(&grid, (0, 0), (4, 7)));
    let blocked = ["S#", "#G"];
    println!("{:?}", astar(&blocked, (0, 0), (1, 1)));
}
