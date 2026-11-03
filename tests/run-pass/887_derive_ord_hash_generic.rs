use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Key<A, B> {
    major: A,
    minor: B,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Level<T> {
    Low,
    Mid(T),
    High { v: T, boost: bool },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Tagged<T>(String, T);

fn uniq_sorted<T: Hash + Eq + Ord + Clone>(xs: &[T]) -> Vec<T> {
    let s: HashSet<T> = xs.iter().cloned().collect();
    let mut v: Vec<T> = s.into_iter().collect();
    v.sort();
    v
}

fn counts<T: Hash + Eq + Ord + Clone>(xs: &[T]) -> Vec<(T, usize)> {
    let mut m: HashMap<T, usize> = HashMap::new();
    for x in xs { *m.entry(x.clone()).or_insert(0) += 1; }
    let mut v: Vec<_> = m.into_iter().collect();
    v.sort();
    v
}

fn main() {
    let keys = vec![
        Key { major: 2, minor: "b" },
        Key { major: 1, minor: "z" },
        Key { major: 2, minor: "a" },
        Key { major: 1, minor: "z" },
    ];
    let mut bt: BTreeMap<Key<i32, &str>, usize> = BTreeMap::new();
    for (i, k) in keys.iter().enumerate() { bt.insert(k.clone(), i); }
    for (k, v) in &bt { println!("{:?} => {}", k, v); }
    println!("{:?}", uniq_sorted(&keys));
    let levels = vec![
        Level::High { v: 1, boost: false },
        Level::Mid(5),
        Level::Low,
        Level::Mid(2),
        Level::High { v: 1, boost: true },
        Level::High { v: 0, boost: true },
        Level::Mid(5),
    ];
    let set: BTreeSet<Level<u8>> = levels.iter().cloned().collect();
    println!("{:?}", set);
    println!("{:?}", counts(&levels));
    println!("{:?}", levels.iter().max());
    println!("{}", Level::Mid(100) < Level::High { v: 0, boost: false });
    println!("{:?}", Level::<i32>::Low.cmp(&Level::Low));
    let tags = vec![Tagged("x".to_string(), 3), Tagged("a".to_string(), 9), Tagged("x".to_string(), 1)];
    println!("{:?}", uniq_sorted(&tags));
    let mut sorted = tags.clone();
    sorted.sort_by(|a, b| b.cmp(a));
    println!("{:?}", sorted);
    let nested: HashSet<Key<Level<i8>, Tagged<bool>>> = vec![
        Key { major: Level::Low, minor: Tagged("p".into(), true) },
        Key { major: Level::Low, minor: Tagged("p".into(), true) },
        Key { major: Level::Mid(-1), minor: Tagged("q".into(), false) },
    ].into_iter().collect();
    let mut nv: Vec<_> = nested.into_iter().collect();
    nv.sort();
    println!("{} {:?}", nv.len(), nv);
    println!("{:?}", (Key { major: 1, minor: 2 }).partial_cmp(&Key { major: 1, minor: 3 }));
}
