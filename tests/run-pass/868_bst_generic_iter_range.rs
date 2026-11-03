use std::cmp::Ordering;
use std::fmt::Debug;
use std::ops::{Bound, RangeBounds};

#[derive(Debug)]
struct Node<K, V> { key: K, val: V, left: Tree<K, V>, right: Tree<K, V> }
#[derive(Debug)]
struct Tree<K, V>(Option<Box<Node<K, V>>>);

impl<K: Ord + Debug, V: Debug> Tree<K, V> {
    fn new() -> Self { Tree(None) }
    fn insert(&mut self, key: K, val: V) -> Option<V> {
        match self.0 {
            None => { self.0 = Some(Box::new(Node { key, val, left: Tree(None), right: Tree(None) })); None }
            Some(ref mut node) => match key.cmp(&node.key) {
                Ordering::Less => node.left.insert(key, val),
                Ordering::Greater => node.right.insert(key, val),
                Ordering::Equal => Some(std::mem::replace(&mut node.val, val)),
            },
        }
    }
    fn get(&self, key: &K) -> Option<&V> {
        let mut cur = &self.0;
        while let Some(n) = cur { match key.cmp(&n.key) { Ordering::Less => cur = &n.left.0, Ordering::Greater => cur = &n.right.0, Ordering::Equal => return Some(&n.val) } }
        None
    }
    fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let mut cur = &mut self.0;
        while let Some(n) = cur { match key.cmp(&n.key) { Ordering::Less => cur = &mut n.left.0, Ordering::Greater => cur = &mut n.right.0, Ordering::Equal => return Some(&mut n.val) } }
        None
    }
    fn pop_min(&mut self) -> Option<(K, V)> {
        if self.0.as_ref()?.left.0.is_some() { return self.0.as_mut().unwrap().left.pop_min(); }
        let node = self.0.take()?;
        let Node { key, val, right, .. } = *node;
        *self = right;
        Some((key, val))
    }
    fn remove(&mut self, key: &K) -> Option<V> {
        let node = self.0.as_mut()?;
        match key.cmp(&node.key) {
            Ordering::Less => node.left.remove(key),
            Ordering::Greater => node.right.remove(key),
            Ordering::Equal => {
                let mut boxed = self.0.take().unwrap();
                match (boxed.left.0.is_some(), boxed.right.0.is_some()) {
                    (false, _) => { *self = std::mem::replace(&mut boxed.right, Tree(None)); Some(boxed.val) }
                    (true, false) => { *self = std::mem::replace(&mut boxed.left, Tree(None)); Some(boxed.val) }
                    (true, true) => {
                        let (sk, sv) = boxed.right.pop_min().unwrap();
                        let old = std::mem::replace(&mut boxed.val, sv);
                        boxed.key = sk;
                        self.0 = Some(boxed);
                        Some(old)
                    }
                }
            }
        }
    }
    fn height(&self) -> usize { self.0.as_ref().map_or(0, |n| 1 + n.left.height().max(n.right.height())) }
    fn len(&self) -> usize { self.iter().count() }
    fn iter(&self) -> Iter<'_, K, V> { let mut it = Iter { stack: vec![] }; it.push_left(&self.0); it }
    fn range<R: RangeBounds<K>>(&self, r: R) -> Vec<(&K, &V)> {
        let mut out = vec![];
        fn walk<'t, K: Ord, V, R: RangeBounds<K>>(t: &'t Option<Box<Node<K, V>>>, r: &R, out: &mut Vec<(&'t K, &'t V)>) {
            let Some(n) = t else { return };
            let above_start = match r.start_bound() { Bound::Included(s) => n.key >= *s, Bound::Excluded(s) => n.key > *s, Bound::Unbounded => true };
            let below_end = match r.end_bound() { Bound::Included(e) => n.key <= *e, Bound::Excluded(e) => n.key < *e, Bound::Unbounded => true };
            if above_start { walk(&n.left.0, r, out); }
            if above_start && below_end { out.push((&n.key, &n.val)); }
            if below_end { walk(&n.right.0, r, out); }
        }
        walk(&self.0, &r, &mut out);
        out
    }
}

struct Iter<'a, K, V> { stack: Vec<&'a Node<K, V>> }
impl<'a, K, V> Iter<'a, K, V> { fn push_left(&mut self, mut t: &'a Option<Box<Node<K, V>>>) { while let Some(n) = t { self.stack.push(n); t = &n.left.0; } } }
impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> { let n = self.stack.pop()?; self.push_left(&n.right.0); Some((&n.key, &n.val)) }
}
impl<'a, K: Ord + Debug, V: Debug> IntoIterator for &'a Tree<K, V> { type Item = (&'a K, &'a V); type IntoIter = Iter<'a, K, V>; fn into_iter(self) -> Iter<'a, K, V> { self.iter() } }

fn main() {
    let mut t = Tree::new();
    for (i, k) in [50, 30, 70, 20, 40, 60, 80, 35, 45, 65, 30].iter().enumerate() {
        if let Some(old) = t.insert(*k, format!("v{}", i)) { println!("replaced {} (old {})", k, old); }
    }
    println!("len {} height {}", t.len(), t.height());
    println!("{:?}", t.iter().map(|(k, _)| *k).collect::<Vec<_>>());
    println!("get 45={:?} 99={:?}", t.get(&45), t.get(&99));
    if let Some(v) = t.get_mut(&60) { v.push_str("!"); }
    println!("range 33..=65 {:?}", t.range(33..=65));
    println!("range ..40 {:?}", t.range(..40).iter().map(|p| p.0).collect::<Vec<_>>());
    println!("range 66.. {:?}", t.range(66..).iter().map(|p| p.0).collect::<Vec<_>>());
    for k in [30, 50, 99, 20, 70] { println!("remove {} -> {:?}; now {:?}", k, t.remove(&k), t.iter().map(|(k, _)| *k).collect::<Vec<_>>()); }
    println!("pop_min {:?} height {}", t.pop_min(), t.height());
    let mut words: Tree<&str, usize> = Tree::new();
    for w in "the quick brown fox jumps over the lazy dog the end".split(' ') {
        match words.get_mut(&w) { Some(c) => *c += 1, None => { words.insert(w, 1); } }
    }
    for (w, c) in &words { print!("{}:{} ", w, c); }
    println!();
    let mut sorted: Tree<i64, ()> = Tree::new();
    let mut x: i64 = 7;
    for _ in 0..200 { x = (x * 1103515245 + 12345) % 2147483648; sorted.insert(x % 1000, ()); }
    let keys: Vec<i64> = sorted.iter().map(|(k, _)| *k).collect();
    println!("unique {} sorted {} first {:?} last {:?} height {}", keys.len(), keys.windows(2).all(|w| w[0] < w[1]), keys.first(), keys.last(), sorted.height());
    while sorted.len() > 5 { sorted.pop_min(); }
    println!("{:?}", sorted.iter().map(|p| *p.0).collect::<Vec<_>>());
}
