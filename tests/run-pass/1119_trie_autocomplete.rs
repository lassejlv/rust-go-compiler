// Trie-based autocomplete: generic Trie<V> over chars with BTreeMap children,
// a custom borrowing iterator (DFS with explicit stack), prefix search,
// ranking by weight, fuzzy matching with edit distance bound, deletion with
// pruning, and a compressed radix view.
use std::collections::{BTreeMap, BinaryHeap};
use std::cmp::Reverse;
use std::fmt::Debug;

#[derive(Debug)]
struct Node<V> {
    children: BTreeMap<char, Node<V>>,
    value: Option<V>,
}

impl<V> Default for Node<V> {
    fn default() -> Self { Node { children: BTreeMap::new(), value: None } }
}

#[derive(Debug, Default)]
pub struct Trie<V> {
    root: Node<V>,
    len: usize,
}

pub struct Iter<'a, V> {
    stack: Vec<(String, &'a Node<V>)>,
}

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = (String, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((prefix, node)) = self.stack.pop() {
            // push children in reverse so iteration is lexicographic
            for (c, child) in node.children.iter().rev() {
                let mut p = prefix.clone();
                p.push(*c);
                self.stack.push((p, child));
            }
            if let Some(v) = &node.value {
                return Some((prefix, v));
            }
        }
        None
    }
}

impl<V> Trie<V> {
    pub fn new() -> Self { Trie { root: Node::default(), len: 0 } }

    pub fn insert(&mut self, key: &str, v: V) -> Option<V> {
        let mut n = &mut self.root;
        for c in key.chars() {
            n = n.children.entry(c).or_default();
        }
        let old = n.value.replace(v);
        if old.is_none() {
            self.len += 1;
        }
        old
    }

    fn find(&self, key: &str) -> Option<&Node<V>> {
        let mut n = &self.root;
        for c in key.chars() {
            n = n.children.get(&c)?;
        }
        Some(n)
    }

    pub fn get(&self, key: &str) -> Option<&V> { self.find(key)?.value.as_ref() }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        let mut n = &mut self.root;
        for c in key.chars() {
            n = n.children.get_mut(&c)?;
        }
        n.value.as_mut()
    }

    pub fn iter(&self) -> Iter<'_, V> { Iter { stack: vec![(String::new(), &self.root)] } }

    pub fn prefix_iter<'a>(&'a self, prefix: &str) -> impl Iterator<Item = (String, &'a V)> + 'a {
        let start = self.find(prefix).map(|n| (prefix.to_string(), n));
        Iter { stack: start.into_iter().collect() }
    }

    pub fn remove(&mut self, key: &str) -> Option<V> {
        fn go<V>(n: &mut Node<V>, key: &[char]) -> (Option<V>, bool) {
            match key.split_first() {
                None => {
                    let v = n.value.take();
                    (v, n.children.is_empty())
                }
                Some((c, rest)) => {
                    let Some(child) = n.children.get_mut(c) else { return (None, false) };
                    let (v, prune) = go(child, rest);
                    if prune {
                        n.children.remove(c);
                    }
                    (v, n.value.is_none() && n.children.is_empty())
                }
            }
        }
        let chars: Vec<char> = key.chars().collect();
        let (v, _) = go(&mut self.root, &chars);
        if v.is_some() {
            self.len -= 1;
        }
        v
    }

    pub fn node_count(&self) -> usize {
        fn count<V>(n: &Node<V>) -> usize { 1 + n.children.values().map(count).sum::<usize>() }
        count(&self.root)
    }

    pub fn longest_prefix_of<'k>(&self, text: &'k str) -> Option<&'k str> {
        let mut n = &self.root;
        let mut best = if n.value.is_some() { Some(0) } else { None };
        for (i, c) in text.char_indices() {
            match n.children.get(&c) {
                Some(ch) => {
                    n = ch;
                    if n.value.is_some() {
                        best = Some(i + c.len_utf8());
                    }
                }
                None => break,
            }
        }
        best.map(|e| &text[..e])
    }

    /// radix-compressed rendering: chains of single children are merged
    pub fn radix_lines(&self) -> Vec<String>
    where
        V: Debug,
    {
        fn go<V: Debug>(n: &Node<V>, depth: usize, out: &mut Vec<String>) {
            for (c, child) in &n.children {
                let mut label = c.to_string();
                let mut cur = child;
                while cur.value.is_none() && cur.children.len() == 1 {
                    let (c2, next) = cur.children.iter().next().unwrap();
                    label.push(*c2);
                    cur = next;
                }
                let v = cur.value.as_ref().map(|v| format!(" = {:?}", v)).unwrap_or_default();
                out.push(format!("{}{}{}", "  ".repeat(depth), label, v));
                go(cur, depth + 1, out);
            }
        }
        let mut out = Vec::new();
        go(&self.root, 0, &mut out);
        out
    }
}

impl<V> FromIterator<(String, V)> for Trie<V> {
    fn from_iter<I: IntoIterator<Item = (String, V)>>(it: I) -> Self {
        let mut t = Trie::new();
        for (k, v) in it {
            t.insert(&k, v);
        }
        t
    }
}

impl<'a, V> IntoIterator for &'a Trie<V> {
    type Item = (String, &'a V);
    type IntoIter = Iter<'a, V>;
    fn into_iter(self) -> Iter<'a, V> { self.iter() }
}

/// fuzzy search: all keys within edit distance `max` of `query`, using the
/// trie to share DP rows between keys with a common prefix.
fn fuzzy<V: Copy>(t: &Trie<V>, query: &str, max: usize) -> Vec<(usize, String, V)> {
    let q: Vec<char> = query.chars().collect();
    let first: Vec<usize> = (0..=q.len()).collect();
    let mut out = Vec::new();
    fn walk<V: Copy>(n: &Node<V>, prefix: &mut String, row: &[usize], q: &[char], max: usize, out: &mut Vec<(usize, String, V)>) {
        if let Some(v) = n.value {
            if row[q.len()] <= max {
                out.push((row[q.len()], prefix.clone(), v));
            }
        }
        if *row.iter().min().unwrap() > max {
            return;
        }
        for (&c, child) in &n.children {
            let mut next = vec![row[0] + 1];
            for j in 1..=q.len() {
                let cost = if q[j - 1] == c { 0 } else { 1 };
                next.push((next[j - 1] + 1).min(row[j] + 1).min(row[j - 1] + cost));
            }
            prefix.push(c);
            walk(child, prefix, &next, q, max, out);
            prefix.pop();
        }
    }
    walk(&t.root, &mut String::new(), &first, &q, max, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    out
}

fn top_k<'a>(t: &'a Trie<u32>, prefix: &str, k: usize) -> Vec<(String, u32)> {
    let mut heap: BinaryHeap<Reverse<(u32, Reverse<String>)>> = BinaryHeap::new();
    for (w, &f) in t.prefix_iter(prefix) {
        heap.push(Reverse((f, Reverse(w))));
        if heap.len() > k {
            heap.pop();
        }
    }
    let mut v: Vec<(String, u32)> = heap.into_iter().map(|Reverse((f, Reverse(w)))| (w, f)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v
}

const TEXT: &str = "the quick brown fox jumps over the lazy dog then the dog sleeps there \
    while the fox thinks about theory and thermal thermometers; the theme of the thesis \
    is that quick quiet quilts quote quizzes, and brown bread browns in the brewery. \
    Über straße naïve café cafés cafeteria tea team teams teammate tear tears";

fn main() {
    let mut t: Trie<u32> = Trie::new();
    for w in TEXT.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()) {
        let w = w.to_lowercase();
        match t.get_mut(&w) {
            Some(c) => *c += 1,
            None => {
                t.insert(&w, 1);
            }
        }
    }
    println!("words={} nodes={}", t.len, t.node_count());
    let all: Vec<String> = t.iter().map(|(k, v)| format!("{}:{}", k, v)).collect();
    println!("{}", all.join(" "));
    for p in ["the", "qu", "br", "caf", "tea", "x", ""] {
        let found: Vec<String> = t.prefix_iter(p).map(|(k, _)| k).collect();
        println!("prefix {:?}: {} -> {:?}", p, found.len(), found.iter().take(6).collect::<Vec<_>>());
    }
    for p in ["the", "t", "qui", "b"] {
        println!("top3 {:?}: {:?}", p, top_k(&t, p, 3));
    }
    for q in ["teh", "quik", "thermos", "cafe", "strasse", "dgo"] {
        let f = fuzzy(&t, q, 2);
        println!("fuzzy {:?}: {:?}", q, f.iter().take(5).map(|(d, w, _)| format!("{}({})", w, d)).collect::<Vec<_>>());
    }
    for text in ["thermometers!", "theorem", "teammates", "zzz", "tea"] {
        println!("longest prefix of {:?}: {:?}", text, t.longest_prefix_of(text));
    }
    let before = t.node_count();
    println!("remove 'thermometers' -> {:?}", t.remove("thermometers"));
    println!("remove 'thermo' -> {:?}", t.remove("thermo"));
    println!("remove 'the' -> {:?}", t.remove("the"));
    println!("get 'then' -> {:?}, get 'the' -> {:?}", t.get("then"), t.get("the"));
    println!("nodes {} -> {}, words {}", before, t.node_count(), t.len);

    let small: Trie<usize> = ["romane", "romanus", "romulus", "rubens", "ruber", "rubicon", "rubicundus", "r"]
        .iter()
        .enumerate()
        .map(|(i, w)| (w.to_string(), i))
        .collect();
    for l in small.radix_lines() {
        println!("{}", l);
    }
    let mut n = 0;
    for (k, v) in &small {
        n += k.len() * v;
    }
    println!("weighted len sum {}", n);
    let mut u: Trie<()> = Trie::new();
    println!("{:?} {:?} {}", u.insert("", ()), u.insert("", ()), u.len);
    println!("{:?}", u.iter().map(|(k, _)| k).collect::<Vec<_>>());
    println!("{:?} {:?}", u.longest_prefix_of("abc"), u.remove(""));
}
