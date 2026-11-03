// Dijkstra with BinaryHeap<Reverse>, path reconstruction, A* on a grid, Bellman-Ford with negative edges
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashMap};

struct Graph {
    adj: BTreeMap<&'static str, Vec<(&'static str, u32)>>,
}

impl Graph {
    fn new(edges: &[(&'static str, &'static str, u32)]) -> Self {
        let mut adj: BTreeMap<&str, Vec<(&str, u32)>> = BTreeMap::new();
        for &(a, b, w) in edges {
            adj.entry(a).or_default().push((b, w));
            adj.entry(b).or_default().push((a, w));
        }
        Graph { adj }
    }
    fn dijkstra(&self, src: &'static str) -> (BTreeMap<&'static str, u32>, HashMap<&'static str, &'static str>) {
        let mut dist = BTreeMap::new();
        let mut prev = HashMap::new();
        let mut h = BinaryHeap::new();
        dist.insert(src, 0);
        h.push(Reverse((0u32, src)));
        while let Some(Reverse((d, u))) = h.pop() {
            if d > *dist.get(u).unwrap_or(&u32::MAX) {
                continue;
            }
            for &(v, w) in self.adj.get(u).map(|v| v.as_slice()).unwrap_or(&[]) {
                let nd = d + w;
                if nd < *dist.get(v).unwrap_or(&u32::MAX) {
                    dist.insert(v, nd);
                    prev.insert(v, u);
                    h.push(Reverse((nd, v)));
                }
            }
        }
        (dist, prev)
    }
}

fn path(prev: &HashMap<&'static str, &'static str>, mut t: &'static str) -> Vec<&'static str> {
    let mut p = vec![t];
    while let Some(&u) = prev.get(t) {
        p.push(u);
        t = u;
    }
    p.reverse();
    p
}

fn astar(grid: &[&str], start: (usize, usize), goal: (usize, usize)) -> Option<(usize, Vec<(usize, usize)>)> {
    let h = |p: (usize, usize)| p.0.abs_diff(goal.0) + p.1.abs_diff(goal.1);
    let rows = grid.len();
    let cols = grid[0].len();
    let cell = |r: usize, c: usize| grid[r].as_bytes()[c];
    let mut g: HashMap<(usize, usize), usize> = HashMap::new();
    let mut came: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
    let mut open = BinaryHeap::new();
    g.insert(start, 0);
    open.push(Reverse((h(start), 0usize, start)));
    while let Some(Reverse((_, cost, p))) = open.pop() {
        if p == goal {
            let mut route = vec![p];
            let mut cur = p;
            while let Some(&q) = came.get(&cur) {
                route.push(q);
                cur = q;
            }
            route.reverse();
            return Some((cost, route));
        }
        if cost > g[&p] {
            continue;
        }
        let (r, c) = p;
        let mut nbrs = vec![];
        if r > 0 { nbrs.push((r - 1, c)); }
        if r + 1 < rows { nbrs.push((r + 1, c)); }
        if c > 0 { nbrs.push((r, c - 1)); }
        if c + 1 < cols { nbrs.push((r, c + 1)); }
        for n in nbrs {
            if cell(n.0, n.1) == b'#' {
                continue;
            }
            let step = if cell(n.0, n.1) == b'~' { 5 } else { 1 };
            let nc = cost + step;
            if nc < *g.get(&n).unwrap_or(&usize::MAX) {
                g.insert(n, nc);
                came.insert(n, p);
                open.push(Reverse((nc + h(n), nc, n)));
            }
        }
    }
    None
}

fn bellman_ford(n: usize, edges: &[(usize, usize, i64)], src: usize) -> Result<Vec<Option<i64>>, String> {
    let mut d: Vec<Option<i64>> = vec![None; n];
    d[src] = Some(0);
    for _ in 0..n - 1 {
        for &(u, v, w) in edges {
            if let Some(du) = d[u] {
                if d[v].map_or(true, |dv| du + w < dv) {
                    d[v] = Some(du + w);
                }
            }
        }
    }
    for &(u, v, w) in edges {
        if let (Some(du), Some(dv)) = (d[u], d[v]) {
            if du + w < dv {
                return Err(format!("negative cycle via {}->{}", u, v));
            }
        }
    }
    Ok(d)
}

fn main() {
    let g = Graph::new(&[
        ("A", "B", 7), ("A", "C", 9), ("A", "F", 14), ("B", "C", 10), ("B", "D", 15),
        ("C", "D", 11), ("C", "F", 2), ("D", "E", 6), ("E", "F", 9), ("G", "H", 1),
    ]);
    let (dist, prev) = g.dijkstra("A");
    for (k, v) in &dist {
        println!("{} {:>2} via {:?}", k, v, path(&prev, k));
    }
    println!("reachable G? {}", dist.contains_key("G"));
    let grid = ["S..#....", ".#.#.##.", ".#...#..", ".####.#.", "...~~...", "#.#..#.G"];
    let find = |ch: u8| {
        grid.iter().enumerate().find_map(|(r, row)| row.bytes().position(|b| b == ch).map(|c| (r, c)))
    };
    let (s, t) = (find(b'S').unwrap(), find(b'G').unwrap());
    match astar(&grid, s, t) {
        Some((cost, route)) => {
            println!("A* cost {} len {}", cost, route.len());
            let mut canvas: Vec<Vec<u8>> = grid.iter().map(|r| r.bytes().collect()).collect();
            for &(r, c) in &route[1..route.len() - 1] {
                canvas[r][c] = b'*';
            }
            for row in canvas {
                println!("{}", String::from_utf8(row).unwrap());
            }
        }
        None => println!("no path"),
    }
    println!("{:?}", astar(&["S#G"], (0, 0), (0, 2)));
    let edges = [(0, 1, 4), (0, 2, 5), (1, 2, -3), (2, 3, 4), (3, 1, 2)];
    println!("{:?}", bellman_ford(5, &edges, 0));
    let neg = [(0, 1, 1), (1, 2, -1), (2, 1, -1)];
    println!("{:?}", bellman_ford(3, &neg, 0));
}
