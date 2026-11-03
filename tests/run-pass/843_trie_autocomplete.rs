use std::collections::BTreeMap;

#[derive(Default, Debug)]
struct TrieNode { children: BTreeMap<char, TrieNode>, count: u32, terminal: bool }

#[derive(Default)]
struct Trie { root: TrieNode, words: usize }

impl Trie {
    fn insert(&mut self, w: &str) {
        let mut node = &mut self.root;
        for c in w.chars() { node = node.children.entry(c).or_default(); node.count += 1; }
        if !node.terminal { self.words += 1; }
        node.terminal = true;
    }
    fn find(&self, prefix: &str) -> Option<&TrieNode> {
        let mut node = &self.root;
        for c in prefix.chars() { node = node.children.get(&c)?; }
        Some(node)
    }
    fn contains(&self, w: &str) -> bool { self.find(w).map_or(false, |n| n.terminal) }
    fn complete(&self, prefix: &str, limit: usize) -> Vec<String> {
        let mut out = vec![];
        if let Some(n) = self.find(prefix) {
            let mut buf = prefix.to_string();
            Self::collect(n, &mut buf, &mut out, limit);
        }
        out
    }
    fn collect(n: &TrieNode, buf: &mut String, out: &mut Vec<String>, limit: usize) {
        if out.len() >= limit { return; }
        if n.terminal { out.push(buf.clone()); }
        for (&c, child) in &n.children {
            buf.push(c);
            Self::collect(child, buf, out, limit);
            buf.pop();
            if out.len() >= limit { return; }
        }
    }
    fn remove(&mut self, w: &str) -> bool {
        fn rec(n: &mut TrieNode, cs: &[char]) -> bool {
            match cs.split_first() {
                None => { if n.terminal { n.terminal = false; true } else { false } }
                Some((c, rest)) => {
                    let Some(child) = n.children.get_mut(c) else { return false };
                    let removed = rec(child, rest);
                    if removed { child.count -= 1; if child.count == 0 { n.children.remove(c); } }
                    removed
                }
            }
        }
        let cs: Vec<char> = w.chars().collect();
        let r = rec(&mut self.root, &cs);
        if r { self.words -= 1; }
        r
    }
    fn longest_common_prefix(&self) -> String {
        let mut s = String::new();
        let mut n = &self.root;
        while n.children.len() == 1 && !n.terminal {
            let (&c, child) = n.children.iter().next().unwrap();
            s.push(c);
            n = child;
        }
        s
    }
    fn node_count(n: &TrieNode) -> usize { 1 + n.children.values().map(Self::node_count).sum::<usize>() }
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + if a[i - 1] == b[j - 1] { 0 } else { 1 });
        }
        prev = cur;
    }
    prev[b.len()]
}

fn main() {
    let mut t = Trie::default();
    let text = "car card care careful cart cat catalog category dog dodge door doorman do zebra zéro zèle car";
    for w in text.split(' ') { t.insert(w); }
    println!("words={} nodes={}", t.words, Trie::node_count(&t.root));
    for p in ["car", "cat", "do", "z", "x", ""] { println!("{:>4}: {:?}", p, t.complete(p, 5)); }
    println!("contains car={} ca={} doorman={}", t.contains("car"), t.contains("ca"), t.contains("doorman"));
    println!("prefix counts: ca={:?} do={:?}", t.find("ca").map(|n| n.count), t.find("do").map(|n| n.count));
    println!("remove card={} card again={} ca={}", t.remove("card"), t.remove("card"), t.remove("ca"));
    println!("remove careful={} -> {:?}", t.remove("careful"), t.complete("car", 10));
    println!("words={} nodes={}", t.words, Trie::node_count(&t.root));
    let mut t2 = Trie::default();
    for w in ["interview", "internet", "interval", "internal"] { t2.insert(w); }
    println!("lcp={:?}", t2.longest_common_prefix());
    t2.insert("in");
    println!("lcp after 'in' = {:?}", t2.longest_common_prefix());
    let dict: Vec<String> = t.complete("", 100);
    for typo in ["dor", "catt", "zero", "cabbage"] {
        let mut best: Vec<(usize, &String)> = dict.iter().map(|w| (edit_distance(typo, w), w)).collect();
        best.sort();
        println!("{} -> {:?}", typo, &best[..3]);
    }
}
