use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Default)]
struct Node { val: i32, children: Vec<Node> }

impl Node {
    fn leaf(v: i32) -> Node { Node { val: v, children: vec![] } }
    fn find_mut(&mut self, v: i32) -> Option<&mut Node> {
        if self.val == v { return Some(self); }
        for c in self.children.iter_mut() {
            if let Some(n) = c.find_mut(v) { return Some(n); }
        }
        None
    }
    fn deepest_mut(&mut self) -> &mut Node {
        let mut cur = self;
        while !cur.children.is_empty() {
            let last = cur.children.len() - 1;
            cur = &mut cur.children[last];
        }
        cur
    }
    fn total(&self) -> i32 { self.val + self.children.iter().map(Node::total).sum::<i32>() }
    fn max_child_mut(&mut self) -> Option<&mut Node> { self.children.iter_mut().max_by_key(|c| c.val) }
}

struct Inventory { items: Vec<(String, u32)>, index: HashMap<String, usize>, log: Vec<String> }

impl Inventory {
    fn new() -> Self { Inventory { items: vec![], index: HashMap::new(), log: vec![] } }
    fn slot(&mut self, name: &str) -> &mut u32 {
        let idx = match self.index.get(name) {
            Some(&i) => i,
            None => { self.items.push((name.to_string(), 0)); let i = self.items.len() - 1; self.index.insert(name.to_string(), i); i }
        };
        &mut self.items[idx].1
    }
    fn add(&mut self, name: &str, n: u32) { *self.slot(name) += n; self.log.push(format!("+{} {}", n, name)); }
    fn take(&mut self, name: &str, n: u32) -> Result<u32, String> {
        let Some(&i) = self.index.get(name) else { return Err(format!("no {}", name)) };
        let (item, log) = (&mut self.items[i], &mut self.log);
        if item.1 < n { return Err(format!("only {} {}", item.1, item.0)); }
        item.1 -= n;
        log.push(format!("-{} {}", n, item.0));
        Ok(item.1)
    }
    fn transfer_all(&mut self, from: &str, to: &str) {
        let (Some(&a), Some(&b)) = (self.index.get(from), self.index.get(to)) else { return };
        if a == b { return; }
        let (x, y) = if a < b { let (l, r) = self.items.split_at_mut(b); (&mut l[a], &mut r[0]) } else { let (l, r) = self.items.split_at_mut(a); (&mut r[0], &mut l[b]) };
        y.1 += std::mem::take(&mut x.1);
    }
}

fn longest_run(v: &mut Vec<i32>) -> &[i32] {
    v.sort();
    let mut best = (0, 0);
    let mut start = 0;
    for i in 1..=v.len() {
        if i == v.len() || v[i] != v[i - 1] + 1 {
            if i - start > best.1 - best.0 { best = (start, i); }
            start = i;
        }
    }
    &v[best.0..best.1]
}

fn bump_all(m: &mut BTreeMap<String, Vec<i32>>, extra: &[i32]) {
    let keys: Vec<String> = m.keys().cloned().collect();
    for (k, e) in keys.iter().zip(extra.iter().cycle()) {
        let len = m[k].len() as i32;
        m.get_mut(k).unwrap().push(e + len);
    }
    if let Some(first) = m.values_mut().next() { first.retain(|x| x % 2 == 0); }
    let total: i32 = m.values().flatten().sum();
    m.entry("total".into()).or_default().push(total);
}

fn main() {
    let mut root = Node { val: 1, children: vec![Node { val: 2, children: vec![Node::leaf(4), Node::leaf(5)] }, Node { val: 3, children: vec![Node::leaf(6)] }] };
    if let Some(n) = root.find_mut(5) { n.val = 50; n.children.push(Node::leaf(7)); }
    if let Some(n) = root.find_mut(99) { n.val = 0; } else { println!("99 not found"); }
    root.deepest_mut().val *= 10;
    let total_before = root.total();
    if let Some(c) = root.max_child_mut() { c.val += 100; c.children.clear(); }
    println!("totals {} {}", total_before, root.total());
    println!("{:?}", root);

    let mut inv = Inventory::new();
    inv.add("apple", 5); inv.add("pear", 2); inv.add("apple", 3);
    println!("{:?} {:?} {:?}", inv.take("apple", 6), inv.take("pear", 3), inv.take("plum", 1));
    inv.add("plum", 9);
    inv.transfer_all("plum", "pear");
    inv.transfer_all("apple", "plum");
    println!("{:?}", inv.items);
    println!("{:?}", inv.log);

    let mut v = vec![10, 4, 2, 3, 11, 12, 1, 13, 14, 20];
    let run = longest_run(&mut v).to_vec();
    println!("{:?} from {:?}", run, v);

    let mut m: BTreeMap<String, Vec<i32>> = BTreeMap::new();
    for (k, x) in [("b", 1), ("a", 2), ("b", 3), ("c", 4)] { m.entry(k.to_string()).or_default().push(x); }
    bump_all(&mut m, &[10, 20]);
    println!("{:?}", m);

    let mut grid = vec![vec![0i32; 5]; 4];
    for (r, row) in grid.iter_mut().enumerate() { for (c, cell) in row.iter_mut().enumerate() { *cell = (r * 5 + c) as i32; } }
    for r in 1..grid.len() {
        let (prev, cur) = grid.split_at_mut(r);
        let above = &prev[r - 1];
        for (c, cell) in cur[0].iter_mut().enumerate() { *cell += above[c]; }
    }
    println!("{:?}", grid);
    let mut words: Vec<String> = "delta alpha charlie bravo".split(' ').map(String::from).collect();
    let longest = words.iter().map(|w| w.len()).max().unwrap_or(0);
    for w in words.iter_mut() { while w.len() < longest { w.push('.'); } }
    words.sort();
    let first = &words[0];
    let n = first.len();
    words.push(format!("len{}", n));
    println!("{:?}", words);
    let mut stack = vec![vec![1], vec![2, 3]];
    while let Some(mut top) = stack.pop() {
        if let Some(x) = top.pop() { if x < 6 { top.push(x * 2 + 1); stack.push(top); } else { println!("big {} rest {:?}", x, top); } }
    }
    let mut counts = [0usize; 3];
    let data = [2, 0, 1, 2, 2, 0];
    data.iter().for_each(|&d| counts[d] += 1);
    let cref = &mut counts;
    cref.sort_unstable();
    println!("{:?}", counts);
}
