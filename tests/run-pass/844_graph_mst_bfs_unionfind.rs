use std::collections::{BTreeMap, BTreeSet, VecDeque};

struct Dsu { parent: Vec<usize>, rank: Vec<u8>, sets: usize }
impl Dsu {
    fn new(n: usize) -> Self { Dsu { parent: (0..n).collect(), rank: vec![0; n], sets: n } }
    fn find(&mut self, x: usize) -> usize { if self.parent[x] != x { let r = self.find(self.parent[x]); self.parent[x] = r; } self.parent[x] }
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

#[derive(Debug, Clone, Copy)]
struct Edge { u: usize, v: usize, w: f64 }

fn kruskal(n: usize, edges: &[Edge]) -> (f64, Vec<Edge>) {
    let mut es = edges.to_vec();
    es.sort_by(|a, b| a.w.total_cmp(&b.w).then(a.u.cmp(&b.u)).then(a.v.cmp(&b.v)));
    let mut d = Dsu::new(n);
    let mut out = vec![];
    for e in es { if d.union(e.u, e.v) { out.push(e); } }
    (out.iter().map(|e| e.w).sum(), out)
}

fn bfs_grid(grid: &[&str], start: (usize, usize), goal: (usize, usize)) -> Option<(usize, String)> {
    let h = grid.len(); let w = grid[0].len();
    let cells: Vec<&[u8]> = grid.iter().map(|r| r.as_bytes()).collect();
    let mut prev = vec![vec![None; w]; h];
    let mut seen = vec![vec![false; w]; h];
    let mut q = VecDeque::from([start]);
    seen[start.0][start.1] = true;
    let dirs = [(0i32, 1i32, 'R'), (1, 0, 'D'), (0, -1, 'L'), (-1, 0, 'U')];
    while let Some((r, c)) = q.pop_front() {
        if (r, c) == goal {
            let mut path = String::new();
            let mut cur = goal;
            while let Some((pr, pc, d)) = prev[cur.0][cur.1] { path.push(d); cur = (pr, pc); }
            let path: String = path.chars().rev().collect();
            return Some((path.len(), path));
        }
        for &(dr, dc, ch) in &dirs {
            let (nr, nc) = (r as i32 + dr, c as i32 + dc);
            if nr < 0 || nc < 0 || nr >= h as i32 || nc >= w as i32 { continue; }
            let (nr, nc) = (nr as usize, nc as usize);
            if cells[nr][nc] == b'#' || seen[nr][nc] { continue; }
            seen[nr][nc] = true;
            prev[nr][nc] = Some((r, c, ch));
            q.push_back((nr, nc));
        }
    }
    None
}

fn components(adj: &BTreeMap<u32, BTreeSet<u32>>) -> Vec<Vec<u32>> {
    let mut seen = BTreeSet::new();
    let mut comps = vec![];
    for &start in adj.keys() {
        if !seen.insert(start) { continue; }
        let mut comp = vec![];
        let mut stack = vec![start];
        while let Some(n) = stack.pop() {
            comp.push(n);
            for &m in adj[&n].iter().rev() { if seen.insert(m) { stack.push(m); } }
        }
        comp.sort();
        comps.push(comp);
    }
    comps
}

fn bipartite(adj: &BTreeMap<u32, BTreeSet<u32>>) -> Result<BTreeMap<u32, u8>, (u32, u32)> {
    let mut color: BTreeMap<u32, u8> = BTreeMap::new();
    for &s in adj.keys() {
        if color.contains_key(&s) { continue; }
        color.insert(s, 0);
        let mut q = VecDeque::from([s]);
        while let Some(n) = q.pop_front() {
            let cn = color[&n];
            for &m in &adj[&n] {
                match color.get(&m) {
                    None => { color.insert(m, 1 - cn); q.push_back(m); }
                    Some(&cm) if cm == cn => return Err((n, m)),
                    _ => {}
                }
            }
        }
    }
    Ok(color)
}

fn main() {
    let pts: [(f64, f64); 7] = [(0.0, 0.0), (2.0, 1.0), (4.0, 0.0), (1.0, 3.0), (3.0, 4.0), (5.0, 3.0), (2.5, 2.0)];
    let mut edges = vec![];
    for i in 0..pts.len() { for j in i + 1..pts.len() {
        let d = ((pts[i].0 - pts[j].0).powi(2) + (pts[i].1 - pts[j].1).powi(2)).sqrt();
        edges.push(Edge { u: i, v: j, w: (d * 1000.0).round() / 1000.0 });
    } }
    let (total, mst) = kruskal(pts.len(), &edges);
    println!("mst total {:.3}", total);
    for e in &mst { println!("  {}-{} {:.3}", e.u, e.v, e.w); }
    let grid = ["S..#......", ".#.#.####.", ".#...#....", ".####.#.#.", "......#.#G"];
    println!("{:?}", bfs_grid(&grid, (0, 0), (4, 9)));
    println!("{:?}", bfs_grid(&["S#", "#G"], (0, 0), (1, 1)));
    let mut adj: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for (a, b) in [(1, 2), (2, 3), (4, 5), (6, 6), (7, 8), (8, 9), (9, 7), (3, 10)] {
        adj.entry(a).or_default().insert(b);
        adj.entry(b).or_default().insert(a);
    }
    println!("{:?}", components(&adj));
    println!("{:?}", bipartite(&adj));
    adj.remove(&6); for s in adj.values_mut() { s.remove(&6); }
    adj.remove(&9); for s in adj.values_mut() { s.remove(&9); }
    println!("{:?}", bipartite(&adj).map(|c| c.into_iter().filter(|&(_, v)| v == 1).map(|(k, _)| k).collect::<Vec<_>>()));
    let mut d = Dsu::new(10);
    for (a, b) in [(0, 1), (2, 3), (1, 3), (5, 6), (0, 2), (7, 7)] { print!("{} ", d.union(a, b)); }
    println!("sets={}", d.sets);
    let groups: BTreeMap<usize, Vec<usize>> = (0..10).fold(BTreeMap::new(), |mut m, i| { let r = d.find(i); m.entry(r).or_insert_with(Vec::new).push(i); m });
    println!("{:?}", groups.values().collect::<Vec<_>>());
}
