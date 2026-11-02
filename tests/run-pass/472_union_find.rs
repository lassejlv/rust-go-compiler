// Union-find (disjoint set union) with path compression and union by size, generic over hashable
// keys via an interning map; Kruskal's MST, connected components on a grid of chars, cycle
// detection in undirected edges, and an offline "are they connected" query batch.
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

struct Dsu {
    parent: Vec<usize>,
    size: Vec<usize>,
    sets: usize,
}

impl Dsu {
    fn new(n: usize) -> Self {
        Dsu { parent: (0..n).collect(), size: vec![1; n], sets: n }
    }
    fn find(&mut self, x: usize) -> usize {
        let mut root = x;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        let mut cur = x;
        while self.parent[cur] != root {
            let next = self.parent[cur];
            self.parent[cur] = root;
            cur = next;
        }
        root
    }
    fn union(&mut self, a: usize, b: usize) -> bool {
        let (mut ra, mut rb) = (self.find(a), self.find(b));
        if ra == rb {
            return false;
        }
        if self.size[ra] < self.size[rb] || (self.size[ra] == self.size[rb] && ra > rb) {
            std::mem::swap(&mut ra, &mut rb);
        }
        self.parent[rb] = ra;
        self.size[ra] += self.size[rb];
        self.sets -= 1;
        true
    }
}

struct KeyedDsu<K> {
    ids: HashMap<K, usize>,
    keys: Vec<K>,
    dsu: Dsu,
}
impl<K: Hash + Eq + Clone + Ord> KeyedDsu<K> {
    fn new() -> Self {
        KeyedDsu { ids: HashMap::new(), keys: vec![], dsu: Dsu::new(0) }
    }
    fn id(&mut self, k: &K) -> usize {
        if let Some(&i) = self.ids.get(k) {
            return i;
        }
        let i = self.keys.len();
        self.keys.push(k.clone());
        self.ids.insert(k.clone(), i);
        self.dsu.parent.push(i);
        self.dsu.size.push(1);
        self.dsu.sets += 1;
        i
    }
    fn union(&mut self, a: &K, b: &K) -> bool {
        let (x, y) = (self.id(a), self.id(b));
        self.dsu.union(x, y)
    }
    fn groups(&mut self) -> Vec<Vec<K>> {
        let mut m: BTreeMap<usize, Vec<K>> = BTreeMap::new();
        for i in 0..self.keys.len() {
            let r = self.dsu.find(i);
            m.entry(r).or_default().push(self.keys[i].clone());
        }
        let mut gs: Vec<Vec<K>> = m.into_values().map(|mut g| {
            g.sort();
            g
        }).collect();
        gs.sort();
        gs
    }
}

fn main() {
    // Kruskal
    let edges = [(0, 1, 7), (0, 3, 5), (1, 2, 8), (1, 3, 9), (1, 4, 7), (2, 4, 5), (3, 4, 15), (3, 5, 6), (4, 5, 8), (4, 6, 9), (5, 6, 11)];
    let mut sorted = edges.to_vec();
    sorted.sort_by_key(|&(a, b, w)| (w, a, b));
    let mut d = Dsu::new(7);
    let mut mst = vec![];
    let mut total = 0;
    for (a, b, w) in sorted {
        if d.union(a, b) {
            mst.push((a, b, w));
            total += w;
        }
    }
    println!("mst {:?} total {} sets {}", mst, total, d.sets);

    // grid components
    let grid = ["aab.c", "a.bbc", "dd..c", "d.eee"];
    let h = grid.len();
    let w = grid[0].len();
    let cell = |r: usize, c: usize| grid[r].as_bytes()[c];
    let mut g = Dsu::new(h * w);
    for r in 0..h {
        for c in 0..w {
            if cell(r, c) == b'.' {
                continue;
            }
            if c + 1 < w && cell(r, c + 1) == cell(r, c) {
                g.union(r * w + c, r * w + c + 1);
            }
            if r + 1 < h && cell(r + 1, c) == cell(r, c) {
                g.union(r * w + c, (r + 1) * w + c);
            }
        }
    }
    let mut comps: BTreeMap<char, Vec<usize>> = BTreeMap::new();
    let mut seen = std::collections::BTreeSet::new();
    for i in 0..h * w {
        let ch = cell(i / w, i % w) as char;
        if ch != '.' && seen.insert(g.find(i)) {
            let root = g.find(i);
            comps.entry(ch).or_default().push(g.size[root]);
        }
    }
    println!("components {:?}", comps);

    // cycle detection
    for es in [vec![(1, 2), (2, 3), (3, 4)], vec![(1, 2), (2, 3), (3, 1)], vec![(0, 0)]] {
        let mut d = Dsu::new(5);
        let first_cycle = es.iter().find(|&&(a, b)| !d.union(a, b));
        println!("{:?} -> cycle edge {:?}", es, first_cycle);
    }

    // keyed
    let mut k: KeyedDsu<String> = KeyedDsu::new();
    let friendships = [("ann", "bob"), ("cat", "dan"), ("bob", "eve"), ("fay", "fay"), ("dan", "gus"), ("eve", "ann")];
    for (a, b) in friendships {
        let merged = k.union(&a.to_string(), &b.to_string());
        println!("{}-{} merged={}", a, b, merged);
    }
    println!("{:?} sets={}", k.groups(), k.dsu.sets);
    let queries = [("ann", "eve"), ("ann", "gus"), ("cat", "gus"), ("fay", "fay")];
    for (a, b) in queries {
        let (x, y) = (k.ids[a], k.ids[b]);
        println!("{} ~ {}: {}", a, b, k.dsu.find(x) == k.dsu.find(y));
    }
}
