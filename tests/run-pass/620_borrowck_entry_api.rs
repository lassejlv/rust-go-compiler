// HashMap/BTreeMap entry API and get_mut-or-insert patterns.
use std::collections::{BTreeMap, HashMap};

fn main() {
    let text = "a b a c b a d";
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for w in text.split(' ') {
        *counts.entry(w).or_insert(0) += 1;
    }
    let mut groups: HashMap<usize, Vec<&str>> = HashMap::new();
    for (w, c) in &counts {
        groups.entry(*c).or_default().push(w);
    }
    let mut keys: Vec<_> = groups.keys().copied().collect();
    keys.sort();
    for k in keys {
        let v = groups.get_mut(&k).unwrap();
        v.sort();
        println!("{} -> {:?}", k, v);
    }
    let mut m: HashMap<String, Vec<u32>> = HashMap::new();
    for i in 0..6u32 {
        let key = format!("k{}", i % 2);
        match m.get_mut(&key) {
            Some(v) => v.push(i),
            None => {
                m.insert(key, vec![i]);
            }
        }
    }
    println!("{:?} {:?}", m["k0"], m["k1"]);
    println!("{:?}", counts);
}
