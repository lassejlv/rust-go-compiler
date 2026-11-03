// HashMap entry API in depth: or_insert_with, or_insert_with_key, and_modify, Entry matching, remove_entry, retain
use std::collections::hash_map::Entry;
use std::collections::HashMap;

fn sorted<K: Ord + Clone, V: Clone>(m: &HashMap<K, V>) -> Vec<(K, V)> {
    let mut v: Vec<(K, V)> = m.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

#[derive(Debug, Clone, Default)]
struct Stats {
    count: u32,
    sum: i64,
    min: Option<i64>,
}

fn main() {
    let text = "one two three two three three four";
    let mut freq: HashMap<&str, u32> = HashMap::new();
    for w in text.split_whitespace() {
        freq.entry(w).and_modify(|c| *c += 1).or_insert(1);
    }
    println!("{:?}", sorted(&freq));

    let mut lens: HashMap<String, usize> = HashMap::new();
    let mut computed = 0;
    for w in ["alpha", "beta", "alpha", "gamma", "beta"] {
        lens.entry(w.to_string()).or_insert_with(|| {
            computed += 1;
            w.len() * 10
        });
    }
    println!("{:?} computed={}", sorted(&lens), computed);
    let mut keyed: HashMap<u32, String> = HashMap::new();
    for k in [3, 1, 3, 2] {
        keyed.entry(k).or_insert_with_key(|k| format!("key{}", k * k));
    }
    println!("{:?}", sorted(&keyed));

    let mut stats: HashMap<char, Stats> = HashMap::new();
    for (c, v) in [('a', 5i64), ('b', -2), ('a', 3), ('c', 10), ('b', 7), ('a', -1)] {
        let s = stats.entry(c).or_default();
        s.count += 1;
        s.sum += v;
        s.min = Some(s.min.map_or(v, |m| m.min(v)));
    }
    for (k, s) in sorted(&stats) {
        println!("{} {:?}", k, s);
    }

    let mut inv: HashMap<&str, i32> = HashMap::new();
    let ops = [("x", 5), ("y", -3), ("x", -5), ("z", 1), ("y", 3), ("w", 0)];
    for (k, d) in ops {
        match inv.entry(k) {
            Entry::Occupied(mut o) => {
                *o.get_mut() += d;
                if *o.get() == 0 {
                    let (k, v) = o.remove_entry();
                    println!("removed {} (was {})", k, v);
                }
            }
            Entry::Vacant(v) => {
                if d != 0 {
                    println!("inserting {} at key {}", d, v.key());
                    v.insert(d);
                } else {
                    println!("skip zero for {}", v.into_key());
                }
            }
        }
    }
    println!("{:?}", sorted(&inv));

    let mut m: HashMap<i32, Vec<i32>> = (0..10).map(|i| (i % 4, vec![])).collect();
    for i in 0..10 {
        m.get_mut(&(i % 4)).unwrap().push(i);
    }
    m.retain(|k, v| {
        v.retain(|x| x % 3 != 0);
        *k != 2 && !v.is_empty()
    });
    println!("{:?}", sorted(&m));
    let e = m.entry(1).or_default();
    e.push(100);
    let ins = *m.entry(9).or_insert_with(|| vec![9, 9]).first().unwrap();
    println!("{} {:?}", ins, sorted(&m));
    println!("{:?} {:?}", m.remove(&9), m.remove(&9));
    println!("{:?}", m.remove_entry(&0));
    println!("{} {}", m.contains_key(&1), m.len());
    let old = m.insert(1, vec![]);
    println!("{:?}", old);
    if let Some(v) = m.get_mut(&3) {
        v.clear();
        v.push(-1);
    }
    println!("{:?}", sorted(&m));
    let mut counts: HashMap<bool, usize> = HashMap::new();
    (1..=20).for_each(|n| *counts.entry(n % 3 == 0).or_insert(0) += 1);
    println!("{:?} {:?}", counts.get(&true), counts[&false]);
    let mut cache: HashMap<u64, u64> = HashMap::new();
    fn fibm(n: u64, c: &mut HashMap<u64, u64>) -> u64 {
        if n < 2 {
            return n;
        }
        if let Some(&v) = c.get(&n) {
            return v;
        }
        let r = fibm(n - 1, c) + fibm(n - 2, c);
        c.insert(n, r);
        r
    }
    println!("{} cache={}", fibm(90, &mut cache), cache.len());
    let drained: Vec<(u64, u64)> = {
        let mut d: Vec<_> = cache.drain().filter(|(k, _)| *k > 87).collect();
        d.sort();
        d
    };
    println!("{:?} {}", drained, cache.is_empty());
}
