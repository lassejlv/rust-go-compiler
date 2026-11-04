// Priority-queue shortest paths: Dijkstra on a generated weighted graph,
// A* on a generated grid maze, plus Bellman-Ford and Floyd-Warshall cross-checks.
use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct State {
    cost: u64,
    node: usize,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.cmp(&self.cost).then_with(|| self.node.cmp(&other.node))
    }
}
impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct Graph {
    adj: Vec<Vec<(usize, u64)>>,
}

impl Graph {
    fn generate(n: usize, extra: usize, seed: u64) -> Graph {
        let mut s = seed;
        let mut rnd = || {
            s = s.wrapping_mul(0x5851F42D4C957F2D).wrapping_add(0x14057B7EF767814F);
            (s >> 32) as usize
        };
        let mut adj = vec![Vec::new(); n];
        // a random spanning chain so most nodes are reachable
        for i in 1..n {
            if i % 17 == 0 {
                continue; // leave some gaps
            }
            let j = rnd() % i;
            let w = (rnd() % 50 + 1) as u64;
            adj[j].push((i, w));
            adj[i].push((j, w + rnd() as u64 % 5));
        }
        for _ in 0..extra {
            let a = rnd() % n;
            let b = rnd() % n;
            if a != b {
                adj[a].push((b, (rnd() % 100 + 1) as u64));
            }
        }
        Graph { adj }
    }

    fn dijkstra(&self, src: usize) -> (Vec<Option<u64>>, Vec<Option<usize>>, usize) {
        let n = self.adj.len();
        let mut dist: Vec<Option<u64>> = vec![None; n];
        let mut prev = vec![None; n];
        let mut heap = BinaryHeap::new();
        let mut pops = 0;
        dist[src] = Some(0);
        heap.push(State { cost: 0, node: src });
        while let Some(State { cost, node }) = heap.pop() {
            pops += 1;
            if let Some(d) = dist[node] {
                if cost > d {
                    continue;
                }
            }
            for &(nb, w) in &self.adj[node] {
                let nc = cost + w;
                if dist[nb].map_or(true, |d| nc < d) {
                    dist[nb] = Some(nc);
                    prev[nb] = Some(node);
                    heap.push(State { cost: nc, node: nb });
                }
            }
        }
        (dist, prev, pops)
    }

    fn bellman_ford(&self, src: usize) -> Vec<Option<u64>> {
        let n = self.adj.len();
        let mut d = vec![None; n];
        d[src] = Some(0u64);
        for _ in 0..n {
            let mut changed = false;
            for u in 0..n {
                if let Some(du) = d[u] {
                    for &(v, w) in &self.adj[u] {
                        if d[v].map_or(true, |dv| du + w < dv) {
                            d[v] = Some(du + w);
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        d
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

// ---------- grid A* ----------
struct Grid {
    w: usize,
    h: usize,
    cells: Vec<u8>, // b'#' wall, b'.' open, b'~' water (cost 5), b'^' hill (cost 3)
}

impl Grid {
    fn generate(w: usize, h: usize, seed: u32) -> Grid {
        let mut s = seed;
        let mut cells = vec![b'.'; w * h];
        for (i, c) in cells.iter_mut().enumerate() {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            let (x, y) = (i % w, i / w);
            *c = match s % 100 {
                0..=21 => b'#',
                22..=29 => b'~',
                30..=37 => b'^',
                _ => b'.',
            };
            if (x < 2 && y < 2) || (x + 2 >= w && y + 2 >= h) {
                *c = b'.';
            }
        }
        Grid { w, h, cells }
    }
    fn cost(&self, x: usize, y: usize) -> Option<u32> {
        match self.cells[y * self.w + x] {
            b'#' => None,
            b'~' => Some(5),
            b'^' => Some(3),
            _ => Some(1),
        }
    }
    fn neighbors(&self, x: usize, y: usize) -> impl Iterator<Item = (usize, usize)> + '_ {
        let dirs: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        dirs.into_iter().filter_map(move |(dx, dy)| {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx >= 0 && ny >= 0 && (nx as usize) < self.w && (ny as usize) < self.h {
                Some((nx as usize, ny as usize))
            } else {
                None
            }
        })
    }

    fn astar(&self, start: (usize, usize), goal: (usize, usize), use_heuristic: bool) -> Option<(u32, Vec<(usize, usize)>, usize)> {
        let hfn = |p: (usize, usize)| -> u32 {
            if use_heuristic {
                (p.0.abs_diff(goal.0) + p.1.abs_diff(goal.1)) as u32
            } else {
                0
            }
        };
        let mut open = BinaryHeap::new();
        let mut g: HashMap<(usize, usize), u32> = HashMap::new();
        let mut came: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
        let mut closed = HashSet::new();
        g.insert(start, 0);
        open.push(Reverse((hfn(start), 0u32, start)));
        while let Some(Reverse((_, gc, p))) = open.pop() {
            if p == goal {
                let mut path = vec![p];
                let mut cur = p;
                while let Some(&q) = came.get(&cur) {
                    path.push(q);
                    cur = q;
                }
                path.reverse();
                return Some((gc, path, closed.len()));
            }
            if !closed.insert(p) {
                continue;
            }
            for (nx, ny) in self.neighbors(p.0, p.1) {
                if let Some(c) = self.cost(nx, ny) {
                    let ng = gc + c;
                    if g.get(&(nx, ny)).map_or(true, |&old| ng < old) {
                        g.insert((nx, ny), ng);
                        came.insert((nx, ny), p);
                        open.push(Reverse((ng + hfn((nx, ny)), ng, (nx, ny))));
                    }
                }
            }
        }
        None
    }

    fn render(&self, path: &[(usize, usize)]) -> String {
        let on: HashSet<&(usize, usize)> = path.iter().collect();
        let mut s = String::new();
        for y in 0..self.h {
            for x in 0..self.w {
                s.push(if on.contains(&(x, y)) { '*' } else { self.cells[y * self.w + x] as char });
            }
            s.push('\n');
        }
        s
    }
}

fn floyd(n: usize, edges: &[(usize, usize, i64)]) -> Vec<Vec<Option<i64>>> {
    let mut d = vec![vec![None; n]; n];
    for i in 0..n {
        d[i][i] = Some(0);
    }
    for &(a, b, w) in edges {
        d[a][b] = Some(d[a][b].map_or(w, |x: i64| x.min(w)));
    }
    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                if let (Some(a), Some(b)) = (d[i][k], d[k][j]) {
                    if d[i][j].map_or(true, |c| a + b < c) {
                        d[i][j] = Some(a + b);
                    }
                }
            }
        }
    }
    d
}

fn main() {
    let g = Graph::generate(200, 400, 12345);
    let edges: usize = g.adj.iter().map(|v| v.len()).sum();
    println!("graph: {} nodes, {} edges", g.adj.len(), edges);
    let (dist, prev, pops) = g.dijkstra(0);
    let bf = g.bellman_ford(0);
    println!("dijkstra pops={} agrees with bellman-ford: {}", pops, dist == bf);
    let reachable = dist.iter().filter(|d| d.is_some()).count();
    let far = dist.iter().enumerate().filter_map(|(i, d)| d.map(|d| (d, i))).max().unwrap();
    println!("reachable={} farthest node {} at {}", reachable, far.1, far.0);
    let p = path_to(&prev, far.1);
    println!("path ({} hops): {:?}", p.len() - 1, p);
    let check: u64 = p.windows(2).map(|w| g.adj[w[0]].iter().filter(|e| e.0 == w[1]).map(|e| e.1).min().unwrap()).sum();
    println!("path weight recomputed: {}", check);
    let unreachable: Vec<usize> = dist.iter().enumerate().filter(|(_, d)| d.is_none()).map(|(i, _)| i).collect();
    println!("unreachable: {:?}", unreachable);
    let mut hist: BTreeMap<u64, usize> = BTreeMap::new();
    for d in dist.iter().flatten() {
        *hist.entry(d / 25 * 25).or_default() += 1;
    }
    for (bucket, count) in &hist {
        println!("  {:>4}-{:<4} {}", bucket, bucket + 24, "#".repeat(*count));
    }

    for (w, h, seed) in [(30usize, 12usize, 7u32), (40, 15, 99), (20, 8, 3)] {
        let grid = Grid::generate(w, h, seed);
        let goal = (w - 1, h - 1);
        let a = grid.astar((0, 0), goal, true);
        let d = grid.astar((0, 0), goal, false);
        match (a, d) {
            (Some((ca, path, ea)), Some((cd, _, ed))) => {
                println!("grid {}x{} seed {}: cost {} (dijkstra {}), path len {}, expanded A* {} vs {}", w, h, seed, ca, cd, path.len(), ea, ed);
                print!("{}", grid.render(&path));
            }
            (None, None) => {
                println!("grid {}x{} seed {}: no path", w, h, seed);
                print!("{}", grid.render(&[]));
            }
            _ => println!("MISMATCH"),
        }
    }

    let edges = [(0, 1, 4), (0, 2, 1), (2, 1, -2), (1, 3, 5), (2, 3, 8), (3, 4, 3), (4, 0, 2), (1, 4, 10)];
    let d = floyd(5, &edges);
    println!("floyd-warshall:");
    for row in &d {
        let cells: Vec<String> = row.iter().map(|x| x.map_or("inf".to_string(), |v| v.to_string())).collect();
        println!("  {}", cells.iter().map(|c| format!("{:>4}", c)).collect::<String>());
    }
    let mut heap: BinaryHeap<(Reverse<u32>, &str)> = BinaryHeap::new();
    for (p, name) in [(5, "e"), (1, "a"), (3, "c"), (1, "b"), (4, "d")] {
        heap.push((Reverse(p), name));
    }
    println!("peek: {:?}", heap.peek());
    let order: Vec<_> = std::iter::from_fn(|| heap.pop()).map(|(Reverse(p), n)| format!("{}{}", n, p)).collect();
    println!("pop order: {:?}", order);
    let v = BinaryHeap::from(vec![3, 1, 4, 1, 5, 9, 2, 6]).into_sorted_vec();
    println!("sorted: {:?}", v);
}
