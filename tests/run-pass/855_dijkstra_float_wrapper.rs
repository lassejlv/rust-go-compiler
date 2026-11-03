use std::cmp::Ordering;
use std::collections::{BinaryHeap, BTreeMap};

#[derive(Debug, Clone, Copy, PartialEq)]
struct Cost(f64);
impl Eq for Cost {}
impl PartialOrd for Cost { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }
impl Ord for Cost { fn cmp(&self, o: &Self) -> Ordering { o.0.total_cmp(&self.0) } } // reversed: min-heap

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct State { cost: Cost, node: usize }

fn dijkstra(adj: &[Vec<(usize, f64)>], src: usize) -> (Vec<f64>, Vec<Option<usize>>) {
    let mut dist = vec![f64::INFINITY; adj.len()];
    let mut prev = vec![None; adj.len()];
    let mut heap = BinaryHeap::new();
    dist[src] = 0.0;
    heap.push(State { cost: Cost(0.0), node: src });
    while let Some(State { cost: Cost(c), node }) = heap.pop() {
        if c > dist[node] { continue; }
        for &(nb, w) in &adj[node] {
            let nc = c + w;
            if nc < dist[nb] { dist[nb] = nc; prev[nb] = Some(node); heap.push(State { cost: Cost(nc), node: nb }); }
        }
    }
    (dist, prev)
}

fn path(prev: &[Option<usize>], mut t: usize) -> Vec<usize> { let mut p = vec![t]; while let Some(u) = prev[t] { p.push(u); t = u; } p.reverse(); p }

fn grid_costs(grid: &[&str]) -> (Vec<Vec<(usize, f64)>>, usize, usize) {
    let h = grid.len(); let w = grid[0].len();
    let cell = |r: usize, c: usize| grid[r].as_bytes()[c];
    let mut adj = vec![vec![]; h * w];
    for r in 0..h { for c in 0..w {
        if cell(r, c) == b'#' { continue; }
        for (dr, dc) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1), (1, 1), (-1, -1), (1, -1), (-1, 1)] {
            let (nr, nc) = (r as i32 + dr, c as i32 + dc);
            if nr < 0 || nc < 0 || nr >= h as i32 || nc >= w as i32 { continue; }
            let (nr, nc) = (nr as usize, nc as usize);
            let ch = cell(nr, nc);
            if ch == b'#' { continue; }
            let terrain = match ch { b'~' => 5.0, b'^' => 2.5, _ => 1.0 };
            let step = if dr != 0 && dc != 0 { std::f64::consts::SQRT_2 } else { 1.0 };
            adj[r * w + c].push((nr * w + nc, terrain * step));
        }
    } }
    (adj, h, w)
}

fn main() {
    let edges = [(0, 1, 7.0), (0, 2, 9.0), (0, 5, 14.0), (1, 2, 10.0), (1, 3, 15.0), (2, 3, 11.0), (2, 5, 2.0), (3, 4, 6.0), (4, 5, 9.0), (6, 7, 1.0)];
    let mut adj = vec![vec![]; 8];
    for &(a, b, w) in &edges { adj[a].push((b, w)); adj[b].push((a, w)); }
    let (d, p) = dijkstra(&adj, 0);
    for (i, di) in d.iter().enumerate() { println!("0 -> {}: {:>5} via {:?}", i, if di.is_finite() { format!("{:.1}", di) } else { "inf".into() }, if di.is_finite() { path(&p, i) } else { vec![] }); }
    let grid = ["S....~~...", ".##..~~.#.", "...^^^..#.", ".#.^#^....", ".#....##.G"];
    let (gadj, h, w) = grid_costs(&grid);
    let (gd, gp) = dijkstra(&gadj, 0);
    let goal = 4 * w + 9;
    println!("grid cost {:.4}", gd[goal]);
    let route = path(&gp, goal);
    let mut canvas: Vec<Vec<u8>> = grid.iter().map(|r| r.as_bytes().to_vec()).collect();
    for &n in &route[1..route.len() - 1] { canvas[n / w][n % w] = b'*'; }
    for row in canvas { println!("{}", String::from_utf8(row).unwrap()); }
    let reachable: BTreeMap<String, usize> = gd.iter().filter(|x| x.is_finite()).fold(BTreeMap::new(), |mut m, &x| { *m.entry(format!("{:02}", x.floor() as u32 / 3 * 3)).or_insert(0) += 1; m });
    println!("{:?} h={}", reachable, h);
    let mut heap: BinaryHeap<Cost> = [3.5, -1.0, f64::INFINITY, 0.0, -0.0, 2.25, f64::NAN].iter().map(|&x| Cost(x)).collect();
    let mut order = vec![];
    while let Some(Cost(x)) = heap.pop() { order.push(x); }
    println!("{:?}", order);
}
