// A* and Dijkstra on a character grid with weighted terrain, BinaryHeap with
// Reverse, path reconstruction, and rendering the path.
use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct P { r: i32, c: i32 }

#[derive(Debug, Eq, PartialEq)]
struct Node { f: u32, g: u32, p: P }

impl Ord for Node {
    fn cmp(&self, o: &Self) -> Ordering {
        o.f.cmp(&self.f).then_with(|| self.g.cmp(&o.g)).then_with(|| o.p.cmp(&self.p))
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) }
}

struct Grid { cells: Vec<Vec<u8>>, rows: i32, cols: i32 }

impl Grid {
    fn parse(s: &str) -> (Grid, P, P) {
        let mut start = P { r: 0, c: 0 };
        let mut goal = start;
        let cells: Vec<Vec<u8>> = s
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .enumerate()
            .map(|(r, l)| {
                l.bytes()
                    .enumerate()
                    .map(|(c, b)| {
                        match b {
                            b'S' => start = P { r: r as i32, c: c as i32 },
                            b'G' => goal = P { r: r as i32, c: c as i32 },
                            _ => {}
                        }
                        b
                    })
                    .collect()
            })
            .collect();
        let rows = cells.len() as i32;
        let cols = cells[0].len() as i32;
        (Grid { cells, rows, cols }, start, goal)
    }
    fn cost(&self, p: P) -> Option<u32> {
        if p.r < 0 || p.c < 0 || p.r >= self.rows || p.c >= self.cols { return None; }
        match self.cells[p.r as usize][p.c as usize] {
            b'#' => None,
            b'~' => Some(5),
            b'^' => Some(3),
            _ => Some(1),
        }
    }
    fn neighbors(&self, p: P) -> impl Iterator<Item = (P, u32)> + '_ {
        [(0, 1), (1, 0), (0, -1), (-1, 0)]
            .into_iter()
            .map(move |(dr, dc)| P { r: p.r + dr, c: p.c + dc })
            .filter_map(move |q| self.cost(q).map(|c| (q, c)))
    }
}

fn manhattan(a: P, b: P) -> u32 { ((a.r - b.r).abs() + (a.c - b.c).abs()) as u32 }

fn search(g: &Grid, s: P, t: P, heuristic: impl Fn(P, P) -> u32) -> Option<(u32, Vec<P>, usize)> {
    let mut open = BinaryHeap::new();
    let mut best: HashMap<P, u32> = HashMap::new();
    let mut came: HashMap<P, P> = HashMap::new();
    let mut closed = HashSet::new();
    best.insert(s, 0);
    open.push(Node { f: heuristic(s, t), g: 0, p: s });
    while let Some(Node { g: cost, p, .. }) = open.pop() {
        if !closed.insert(p) { continue; }
        if p == t {
            let mut path = vec![t];
            let mut cur = t;
            while let Some(&prev) = came.get(&cur) {
                path.push(prev);
                cur = prev;
            }
            path.reverse();
            return Some((cost, path, closed.len()));
        }
        for (q, c) in g.neighbors(p) {
            let ng = cost + c;
            if best.get(&q).map_or(true, |&old| ng < old) {
                best.insert(q, ng);
                came.insert(q, p);
                open.push(Node { f: ng + heuristic(q, t), g: ng, p: q });
            }
        }
    }
    None
}

fn dijkstra_all(g: &Grid, s: P) -> Vec<Vec<Option<u32>>> {
    let mut dist = vec![vec![None; g.cols as usize]; g.rows as usize];
    let mut heap = BinaryHeap::new();
    heap.push(Reverse((0u32, s)));
    while let Some(Reverse((d, p))) = heap.pop() {
        let slot = &mut dist[p.r as usize][p.c as usize];
        if slot.is_some() { continue; }
        *slot = Some(d);
        for (q, c) in g.neighbors(p) {
            if dist[q.r as usize][q.c as usize].is_none() {
                heap.push(Reverse((d + c, q)));
            }
        }
    }
    dist
}

fn main() {
    let map = "
        S..#......
        .#.#.~~~..
        .#...~#~..
        .####.#^^.
        ...^..#..G
        .#~~~.#.#.
        ...#......
    ";
    let (g, s, t) = Grid::parse(map);
    println!("grid {}x{} start={:?} goal={:?}", g.rows, g.cols, s, t);
    let (cost_a, path_a, exp_a) = search(&g, s, t, manhattan).expect("path");
    let (cost_d, path_d, exp_d) = search(&g, s, t, |_, _| 0).expect("path");
    println!("A*: cost={} len={} expanded={}", cost_a, path_a.len(), exp_a);
    println!("Dijkstra: cost={} len={} expanded={}", cost_d, path_d.len(), exp_d);
    println!("same cost: {}, A* expanded fewer or equal: {}", cost_a == cost_d, exp_a <= exp_d);
    let on_path: HashSet<P> = path_a.iter().copied().collect();
    for r in 0..g.rows {
        let line: String = (0..g.cols)
            .map(|c| {
                let p = P { r, c };
                let ch = g.cells[r as usize][c as usize] as char;
                if on_path.contains(&p) && ch != 'S' && ch != 'G' { '*' } else { ch }
            })
            .collect();
        println!("  {}", line);
    }
    let d = dijkstra_all(&g, s);
    for row in &d {
        let s: Vec<String> = row.iter().map(|x| match x { Some(v) => format!("{:>3}", v), None => "  #".into() }).collect();
        println!("{}", s.join(""));
    }
    let reachable = d.iter().flatten().filter(|x| x.is_some()).count();
    let far = d.iter().flatten().filter_map(|x| *x).max().unwrap();
    println!("reachable={} farthest={}", reachable, far);
    let (g2, s2, t2) = Grid::parse("S#G\n.#.\n.#.");
    println!("blocked: {:?}", search(&g2, s2, t2, manhattan).map(|x| x.0));
    let mut heap: BinaryHeap<Node> = vec![
        Node { f: 5, g: 1, p: P { r: 0, c: 0 } },
        Node { f: 3, g: 2, p: P { r: 1, c: 0 } },
        Node { f: 3, g: 4, p: P { r: 2, c: 0 } },
        Node { f: 9, g: 0, p: P { r: 3, c: 0 } },
    ].into_iter().collect();
    let mut order = vec![];
    while let Some(n) = heap.pop() { order.push((n.f, n.g)); }
    println!("heap order: {:?}", order);
}
