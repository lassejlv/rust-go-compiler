// topological sort (Kahn + DFS), cycle detection, build-order with BTreeMap determinism, SCC (Tarjan)
use std::collections::{BTreeMap, BTreeSet, VecDeque};

fn kahn(deps: &BTreeMap<&str, Vec<&str>>) -> Result<Vec<String>, Vec<String>> {
    let mut indeg: BTreeMap<&str, usize> = BTreeMap::new();
    let mut rev: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (&k, vs) in deps {
        indeg.entry(k).or_insert(0);
        for &v in vs {
            indeg.entry(v).or_insert(0);
            *indeg.get_mut(k).unwrap() += 1;
            rev.entry(v).or_default().push(k);
        }
    }
    let mut ready: BTreeSet<&str> = indeg.iter().filter(|(_, &d)| d == 0).map(|(&k, _)| k).collect();
    let mut out = vec![];
    while let Some(&n) = ready.iter().next() {
        ready.remove(n);
        out.push(n.to_string());
        if let Some(users) = rev.get(n) {
            for &u in users {
                let d = indeg.get_mut(u).unwrap();
                *d -= 1;
                if *d == 0 {
                    ready.insert(u);
                }
            }
        }
    }
    if out.len() == indeg.len() {
        Ok(out)
    } else {
        Err(indeg.iter().filter(|(_, &d)| d > 0).map(|(k, _)| k.to_string()).collect())
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Mark {
    None,
    Temp,
    Done,
}

fn dfs_topo(n: usize, edges: &[(usize, usize)]) -> Option<Vec<usize>> {
    let mut adj = vec![vec![]; n];
    for &(a, b) in edges {
        adj[a].push(b);
    }
    fn visit(u: usize, adj: &[Vec<usize>], mark: &mut [Mark], out: &mut Vec<usize>) -> bool {
        match mark[u] {
            Mark::Done => return true,
            Mark::Temp => return false,
            Mark::None => {}
        }
        mark[u] = Mark::Temp;
        for &v in &adj[u] {
            if !visit(v, adj, mark, out) {
                return false;
            }
        }
        mark[u] = Mark::Done;
        out.push(u);
        true
    }
    let mut mark = vec![Mark::None; n];
    let mut out = vec![];
    for u in 0..n {
        if !visit(u, &adj, &mut mark, &mut out) {
            return None;
        }
    }
    out.reverse();
    Some(out)
}

struct Tarjan<'a> {
    adj: &'a [Vec<usize>],
    index: Vec<Option<usize>>,
    low: Vec<usize>,
    on: Vec<bool>,
    stack: Vec<usize>,
    next: usize,
    sccs: Vec<Vec<usize>>,
}
impl<'a> Tarjan<'a> {
    fn run(adj: &'a [Vec<usize>]) -> Vec<Vec<usize>> {
        let n = adj.len();
        let mut t = Tarjan { adj, index: vec![None; n], low: vec![0; n], on: vec![false; n], stack: vec![], next: 0, sccs: vec![] };
        for v in 0..n {
            if t.index[v].is_none() {
                t.strong(v);
            }
        }
        t.sccs
    }
    fn strong(&mut self, v: usize) {
        self.index[v] = Some(self.next);
        self.low[v] = self.next;
        self.next += 1;
        self.stack.push(v);
        self.on[v] = true;
        for &w in &self.adj[v] {
            match self.index[w] {
                None => {
                    self.strong(w);
                    self.low[v] = self.low[v].min(self.low[w]);
                }
                Some(iw) if self.on[w] => self.low[v] = self.low[v].min(iw),
                _ => {}
            }
        }
        if Some(self.low[v]) == self.index[v] {
            let mut comp = vec![];
            loop {
                let w = self.stack.pop().unwrap();
                self.on[w] = false;
                comp.push(w);
                if w == v {
                    break;
                }
            }
            comp.sort();
            self.sccs.push(comp);
        }
    }
}

fn main() {
    let mut deps: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    deps.insert("app", vec!["net", "ui", "log"]);
    deps.insert("ui", vec!["gfx", "log"]);
    deps.insert("net", vec!["log", "tls"]);
    deps.insert("tls", vec!["crypto"]);
    deps.insert("gfx", vec![]);
    println!("{:?}", kahn(&deps));
    deps.insert("crypto", vec!["app"]);
    println!("{:?}", kahn(&deps));
    println!("{:?}", dfs_topo(6, &[(5, 2), (5, 0), (4, 0), (4, 1), (2, 3), (3, 1)]));
    println!("{:?}", dfs_topo(3, &[(0, 1), (1, 2), (2, 0)]));
    let adj = vec![vec![1], vec![2, 3], vec![0], vec![4], vec![5, 7], vec![6], vec![4], vec![]];
    let mut s = Tarjan::run(&adj);
    s.sort();
    println!("{:?}", s);
    let mut q: VecDeque<usize> = VecDeque::new();
    let mut level = vec![usize::MAX; adj.len()];
    level[0] = 0;
    q.push_back(0);
    while let Some(u) = q.pop_front() {
        for &v in &adj[u] {
            if level[v] == usize::MAX {
                level[v] = level[u] + 1;
                q.push_back(v);
            }
        }
    }
    println!("{:?}", level);
}
