// An insertion-ordered map (indexmap-like): Vec of entries + HashMap from key to index.
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

struct IndexMap<K, V> {
    entries: Vec<(K, V)>,
    indices: HashMap<K, usize>,
}

enum Entry<'a, K, V> {
    Occupied(&'a mut V),
    Vacant(&'a mut IndexMap<K, V>, K),
}

impl<'a, K: Hash + Eq + Clone, V> Entry<'a, K, V> {
    fn or_insert(self, v: V) -> &'a mut V {
        self.or_insert_with(|| v)
    }
    fn or_insert_with<F: FnOnce() -> V>(self, f: F) -> &'a mut V {
        match self {
            Entry::Occupied(v) => v,
            Entry::Vacant(map, k) => {
                let (i, _) = map.insert_full(k, f());
                &mut map.entries[i].1
            }
        }
    }
    fn and_modify<F: FnOnce(&mut V)>(self, f: F) -> Self {
        match self {
            Entry::Occupied(v) => {
                f(v);
                Entry::Occupied(v)
            }
            e => e,
        }
    }
}

impl<K: Hash + Eq + Clone, V> IndexMap<K, V> {
    fn new() -> Self {
        IndexMap { entries: Vec::new(), indices: HashMap::new() }
    }
    fn len(&self) -> usize {
        self.entries.len()
    }
    fn insert_full(&mut self, k: K, v: V) -> (usize, Option<V>) {
        if let Some(&i) = self.indices.get(&k) {
            let old = std::mem::replace(&mut self.entries[i].1, v);
            (i, Some(old))
        } else {
            let i = self.entries.len();
            self.indices.insert(k.clone(), i);
            self.entries.push((k, v));
            (i, None)
        }
    }
    fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.insert_full(k, v).1
    }
    fn get<Q>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.indices.get(k).map(|&i| &self.entries[i].1)
    }
    fn get_index_of<Q: Hash + Eq + ?Sized>(&self, k: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
    {
        self.indices.get(k).copied()
    }
    fn get_index(&self, i: usize) -> Option<(&K, &V)> {
        self.entries.get(i).map(|(k, v)| (k, v))
    }
    fn entry(&mut self, k: K) -> Entry<'_, K, V> {
        match self.indices.get(&k) {
            Some(&i) => Entry::Occupied(&mut self.entries[i].1),
            None => Entry::Vacant(self, k),
        }
    }
    // O(1) removal that moves the last element into the hole
    fn swap_remove<Q: Hash + Eq + ?Sized>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        let i = self.indices.remove(k)?;
        let (_, v) = self.entries.swap_remove(i);
        if i < self.entries.len() {
            let moved = self.entries[i].0.clone();
            *self.indices.get_mut::<K>(&moved).unwrap() = i;
        }
        Some(v)
    }
    // O(n) removal preserving order
    fn shift_remove<Q: Hash + Eq + ?Sized>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        let i = self.indices.remove(k)?;
        let (_, v) = self.entries.remove(i);
        for (j, (key, _)) in self.entries.iter().enumerate().skip(i) {
            *self.indices.get_mut::<K>(key).unwrap() = j;
        }
        Some(v)
    }
    fn iter(&self) -> impl DoubleEndedIterator<Item = (&K, &V)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }
    fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.iter().map(|e| &e.0)
    }
    fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.entries.iter_mut().map(|e| &mut e.1)
    }
    fn sort_by<F: FnMut(&K, &V, &K, &V) -> std::cmp::Ordering>(&mut self, mut f: F) {
        self.entries.sort_by(|a, b| f(&a.0, &a.1, &b.0, &b.1));
        for (i, (k, _)) in self.entries.iter().enumerate() {
            *self.indices.get_mut(k).unwrap() = i;
        }
    }
    fn check(&self) -> bool {
        self.entries.iter().enumerate().all(|(i, (k, _))| self.indices[k] == i) && self.indices.len() == self.entries.len()
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for IndexMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self.entries.iter().map(|(k, v)| (k, v))).finish()
    }
}

fn main() {
    let mut m: IndexMap<String, i32> = IndexMap::new();
    for w in "the quick brown fox jumps over the lazy dog the end".split(' ') {
        *m.entry(w.to_string()).or_insert(0) += 1;
    }
    println!("{:?}", m);
    println!("len={} the={:?} cat={:?} idx(fox)={:?}", m.len(), m.get("the"), m.get("cat"), m.get_index_of("fox"));
    println!("{:?}", m.get_index(4));
    m.entry("quick".into()).and_modify(|v| *v *= 10).or_insert(99);
    m.entry("new".into()).and_modify(|v| *v *= 10).or_insert_with(|| -1);
    println!("{:?}", m);
    println!("insert existing: {:?}", m.insert("fox".into(), 7));
    println!("swap_remove brown: {:?}", m.swap_remove("brown"));
    println!("{:?} ok={}", m.keys().collect::<Vec<_>>(), m.check());
    println!("shift_remove over: {:?}", m.shift_remove("over"));
    println!("shift_remove zzz: {:?}", m.shift_remove("zzz"));
    println!("{:?} ok={}", m.keys().collect::<Vec<_>>(), m.check());
    for v in m.values_mut() {
        *v += 100;
    }
    m.sort_by(|k1, v1, k2, v2| v2.cmp(v1).then(k1.cmp(k2)));
    println!("{:?} ok={}", m, m.check());
    let last_two: Vec<_> = m.iter().rev().take(2).collect();
    println!("{:?}", last_two);
    println!("idx(the)={:?} idx(dog)={:?}", m.get_index_of("the"), m.get_index_of("dog"));
}
