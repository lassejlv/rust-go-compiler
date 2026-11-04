// Ownership-heavy data structure code written the way NLL allows: an owned
// binary search tree with iterative &mut cursor insert/delete, a singly
// linked list with tail-append via `&mut Option<Box<_>>` walking, an LRU
// cache over an index-linked list + HashMap, a trie with get_or_insert
// descent, and assorted two-phase / disjoint-field borrow idioms.
use std::collections::HashMap;
use std::fmt::Debug;

// ---------------- BST ----------------
#[derive(Debug)]
struct Node<K, V> {
    key: K,
    val: V,
    left: Tree<K, V>,
    right: Tree<K, V>,
}
type Tree<K, V> = Option<Box<Node<K, V>>>;

struct Bst<K, V> {
    root: Tree<K, V>,
    len: usize,
}

impl<K: Ord + Debug, V: Debug> Bst<K, V> {
    fn new() -> Self {
        Bst { root: None, len: 0 }
    }
    fn insert(&mut self, key: K, val: V) -> Option<V> {
        let mut cur = &mut self.root;
        while let Some(node) = cur {
            if key == node.key {
                return Some(std::mem::replace(&mut node.val, val));
            }
            cur = if key < node.key { &mut node.left } else { &mut node.right };
        }
        *cur = Some(Box::new(Node { key, val, left: None, right: None }));
        self.len += 1;
        None
    }
    fn get(&self, key: &K) -> Option<&V> {
        let mut cur = &self.root;
        while let Some(n) = cur {
            match key.cmp(&n.key) {
                std::cmp::Ordering::Equal => return Some(&n.val),
                std::cmp::Ordering::Less => cur = &n.left,
                std::cmp::Ordering::Greater => cur = &n.right,
            }
        }
        None
    }
    fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let mut cur = self.root.as_deref_mut();
        while let Some(n) = cur {
            if *key == n.key {
                return Some(&mut n.val);
            }
            cur = if *key < n.key { n.left.as_deref_mut() } else { n.right.as_deref_mut() };
        }
        None
    }
    fn remove(&mut self, key: &K) -> Option<V> {
        let mut cur = &mut self.root;
        loop {
            match cur {
                None => return None,
                Some(n) if *key < n.key => cur = &mut cur.as_mut().unwrap().left,
                Some(n) if *key > n.key => cur = &mut cur.as_mut().unwrap().right,
                Some(_) => break,
            }
        }
        let mut node = cur.take().unwrap();
        self.len -= 1;
        *cur = match (node.left.take(), node.right.take()) {
            (None, None) => None,
            (Some(l), None) => Some(l),
            (None, Some(r)) => Some(r),
            (Some(l), Some(r)) => {
                // detach the minimum of the right subtree
                let mut right = Some(r);
                let mut min_slot = &mut right;
                while min_slot.as_ref().unwrap().left.is_some() {
                    min_slot = &mut min_slot.as_mut().unwrap().left;
                }
                let mut min = min_slot.take().unwrap();
                *min_slot = min.right.take();
                min.left = Some(l);
                min.right = right;
                Some(min)
            }
        };
        Some(node.val)
    }
    fn height(t: &Tree<K, V>) -> usize {
        t.as_ref().map_or(0, |n| 1 + Self::height(&n.left).max(Self::height(&n.right)))
    }
    fn in_order(&self) -> Vec<(&K, &V)> {
        let mut out = Vec::with_capacity(self.len);
        let mut stack: Vec<&Node<K, V>> = Vec::new();
        let mut cur = self.root.as_deref();
        loop {
            while let Some(n) = cur {
                stack.push(n);
                cur = n.left.as_deref();
            }
            match stack.pop() {
                Some(n) => {
                    out.push((&n.key, &n.val));
                    cur = n.right.as_deref();
                }
                None => break,
            }
        }
        out
    }
    fn for_each_mut(&mut self, mut f: impl FnMut(&K, &mut V)) {
        let mut stack: Vec<&mut Node<K, V>> = Vec::new();
        if let Some(r) = self.root.as_deref_mut() {
            stack.push(r);
        }
        while let Some(n) = stack.pop() {
            f(&n.key, &mut n.val);
            if let Some(l) = n.left.as_deref_mut() {
                stack.push(l);
            }
            if let Some(r) = n.right.as_deref_mut() {
                stack.push(r);
            }
        }
    }
}

// ---------------- linked list ----------------
struct List<T> {
    head: Option<Box<LNode<T>>>,
}
struct LNode<T> {
    v: T,
    next: Option<Box<LNode<T>>>,
}

impl<T: PartialEq + Debug> List<T> {
    fn push_back(&mut self, v: T) {
        let mut cur = &mut self.head;
        while let Some(node) = cur {
            cur = &mut node.next;
        }
        *cur = Some(Box::new(LNode { v, next: None }));
    }
    fn remove_all(&mut self, target: &T) -> usize {
        let mut removed = 0;
        let mut cur = &mut self.head;
        loop {
            match cur {
                None => break,
                Some(node) if node.v == *target => {
                    *cur = node.next.take();
                    removed += 1;
                }
                Some(node) => cur = &mut node.next,
            }
        }
        removed
    }
    fn reverse(&mut self) {
        let mut prev = None;
        let mut cur = self.head.take();
        while let Some(mut n) = cur {
            cur = n.next.take();
            n.next = prev;
            prev = Some(n);
        }
        self.head = prev;
    }
    fn to_vec(&self) -> Vec<&T> {
        let mut v = vec![];
        let mut c = &self.head;
        while let Some(n) = c {
            v.push(&n.v);
            c = &n.next;
        }
        v
    }
    fn nth_mut(&mut self, mut i: usize) -> Option<&mut T> {
        let mut c = self.head.as_mut();
        while let Some(n) = c {
            if i == 0 {
                return Some(&mut n.v);
            }
            i -= 1;
            c = n.next.as_mut();
        }
        None
    }
}

impl<T> Drop for List<T> {
    fn drop(&mut self) {
        let mut c = self.head.take();
        while let Some(mut n) = c {
            c = n.next.take();
        }
    }
}

// ---------------- LRU cache ----------------
struct Lru<V> {
    cap: usize,
    map: HashMap<String, usize>,
    slots: Vec<(String, V, Option<usize>, Option<usize>)>, // key, val, prev, next
    head: Option<usize>,
    tail: Option<usize>,
    free: Vec<usize>,
    hits: u32,
    misses: u32,
}

impl<V: Clone + Debug> Lru<V> {
    fn new(cap: usize) -> Self {
        Lru { cap, map: HashMap::new(), slots: vec![], head: None, tail: None, free: vec![], hits: 0, misses: 0 }
    }
    fn unlink(&mut self, i: usize) {
        let (p, n) = (self.slots[i].2, self.slots[i].3);
        match p {
            Some(p) => self.slots[p].3 = n,
            None => self.head = n,
        }
        match n {
            Some(n) => self.slots[n].2 = p,
            None => self.tail = p,
        }
    }
    fn push_front(&mut self, i: usize) {
        self.slots[i].2 = None;
        self.slots[i].3 = self.head;
        if let Some(h) = self.head {
            self.slots[h].2 = Some(i);
        }
        self.head = Some(i);
        if self.tail.is_none() {
            self.tail = Some(i);
        }
    }
    fn get(&mut self, k: &str) -> Option<&V> {
        match self.map.get(k).copied() {
            Some(i) => {
                self.hits += 1;
                self.unlink(i);
                self.push_front(i);
                Some(&self.slots[i].1)
            }
            None => {
                self.misses += 1;
                None
            }
        }
    }
    fn put(&mut self, k: &str, v: V) -> Option<(String, V)> {
        if let Some(&i) = self.map.get(k) {
            self.slots[i].1 = v;
            self.unlink(i);
            self.push_front(i);
            return None;
        }
        let mut evicted = None;
        if self.map.len() == self.cap {
            let t = self.tail.unwrap();
            self.unlink(t);
            let key = std::mem::take(&mut self.slots[t].0);
            self.map.remove(&key);
            evicted = Some((key, self.slots[t].1.clone()));
            self.free.push(t);
        }
        let i = match self.free.pop() {
            Some(i) => {
                self.slots[i] = (k.to_string(), v, None, None);
                i
            }
            None => {
                self.slots.push((k.to_string(), v, None, None));
                self.slots.len() - 1
            }
        };
        self.map.insert(k.to_string(), i);
        self.push_front(i);
        evicted
    }
    fn order(&self) -> Vec<&str> {
        let mut out = vec![];
        let mut c = self.head;
        while let Some(i) = c {
            out.push(self.slots[i].0.as_str());
            c = self.slots[i].3;
        }
        out
    }
}

// ---------------- trie ----------------
#[derive(Default, Debug)]
struct Trie {
    children: Vec<(char, Trie)>,
    count: u32,
}

impl Trie {
    fn child_mut(&mut self, c: char) -> &mut Trie {
        let pos = match self.children.iter().position(|(ch, _)| *ch == c) {
            Some(p) => p,
            None => {
                self.children.push((c, Trie::default()));
                self.children.len() - 1
            }
        };
        &mut self.children[pos].1
    }
    fn insert(&mut self, w: &str) {
        let mut node = self;
        for c in w.chars() {
            node = node.child_mut(c);
        }
        node.count += 1;
    }
    fn find(&self, w: &str) -> Option<&Trie> {
        let mut node = self;
        for c in w.chars() {
            node = &node.children.iter().find(|(ch, _)| *ch == c)?.1;
        }
        Some(node)
    }
    fn words(&self, prefix: &mut String, out: &mut Vec<(String, u32)>) {
        if self.count > 0 {
            out.push((prefix.clone(), self.count));
        }
        let mut kids: Vec<&(char, Trie)> = self.children.iter().collect();
        kids.sort_by_key(|(c, _)| *c);
        for (c, t) in kids {
            prefix.push(*c);
            t.words(prefix, out);
            prefix.pop();
        }
    }
}

struct Stats {
    items: Vec<i64>,
    offset: i64,
    log: Vec<String>,
    counts: HashMap<i64, usize>,
}

impl Stats {
    fn bump(&mut self, x: i64) -> usize {
        let c = self.counts.entry(x).or_insert(0);
        *c += 1;
        *c
    }
    fn process(&mut self) {
        // disjoint field borrows inside a method
        for x in self.items.iter_mut() {
            *x += self.offset;
        }
        let log = &mut self.log;
        self.items.iter().filter(|x| **x % 2 == 0).for_each(|x| log.push(format!("even {}", x)));
        // two-phase borrows
        self.items.push(self.items.len() as i64);
        let n = self.items[0];
        let c = self.bump(n);
        self.log.push(format!("bumped {} -> {}", n, c));
        let first_big = self.items.iter().position(|&x| x > 25).map(|i| self.items.remove(i));
        self.log.push(format!("removed {:?}", first_big));
        // closure capturing one field mutably while another is read (edition 2021)
        let offset = &self.offset;
        let mut add = |v: i64| self.items.push(v + *offset);
        add(1);
        add(2);
        let mid = self.items.len() / 2;
        self.items.insert(mid, -1);
        std::mem::swap(&mut self.offset, &mut self.items[0]);
    }
}

fn main() {
    let mut t = Bst::new();
    let keys = [50, 30, 70, 20, 40, 60, 80, 35, 45, 65, 75, 85, 10, 25];
    for (i, k) in keys.iter().enumerate() {
        t.insert(*k, format!("v{}", i));
    }
    let replaced = t.insert(40, "forty".into());
    println!("len {} height {} replace {:?}", t.len, Bst::height(&t.root), replaced);
    if let Some(v) = t.get_mut(&35) {
        v.push_str("-edited");
    }
    t.for_each_mut(|k, v| {
        if k % 20 == 0 {
            v.make_ascii_uppercase();
        }
    });
    println!("{:?}", t.in_order().iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>());
    for k in [30, 50, 85, 99, 10, 70] {
        let r = t.remove(&k);
        println!("remove {} -> {:?}; len {} height {} get(45)={:?}", k, r, t.len, Bst::height(&t.root), t.get(&45));
    }
    println!("{:?}", t.in_order().iter().map(|(k, _)| **k).collect::<Vec<_>>());

    let mut l = List { head: None };
    for x in [3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5] {
        l.push_back(x);
    }
    let removed = l.remove_all(&5);
    if let Some(x) = l.nth_mut(2) {
        *x *= 100;
    }
    l.reverse();
    println!("list {:?} removed {}", l.to_vec(), removed);
    let mut big = List { head: None };
    for i in 0..3000 {
        big.push_back(i % 7);
    }
    println!("big removed {} remaining {}", big.remove_all(&3), big.to_vec().len());

    let mut c = Lru::new(3);
    let ops = [("put", "a", 1), ("put", "b", 2), ("get", "a", 0), ("put", "c", 3), ("put", "d", 4), ("get", "b", 0), ("get", "c", 0), ("put", "a", 10), ("put", "e", 5), ("get", "d", 0)];
    for (op, k, v) in ops {
        match op {
            "put" => {
                let ev = c.put(k, v);
                println!("put {}={} evicted {:?} order {:?}", k, v, ev, c.order());
            }
            _ => {
                let got = c.get(k).cloned();
                println!("get {} -> {:?} order {:?}", k, got, c.order());
            }
        }
    }
    println!("hits {} misses {} slots {}", c.hits, c.misses, c.slots.len());

    let mut trie = Trie::default();
    for w in "the then there their they them a an and ant the theory".split(' ') {
        trie.insert(w);
    }
    let mut out = vec![];
    trie.find("the").unwrap().words(&mut "the".to_string(), &mut out);
    println!("{:?}", out);
    println!("{:?} {:?}", trie.find("an").map(|t| (t.count, t.children.len())), trie.find("x").is_none());

    let mut st = Stats { items: vec![1, 2, 3, 30, 4], offset: 10, log: vec![], counts: HashMap::new() };
    st.process();
    println!("{:?} offset {} log {:?}", st.items, st.offset, st.log);
}
