// Breadth test of HashMap and HashSet APIs (outputs sorted for determinism).
use std::collections::{BTreeMap, HashMap, HashSet};

fn sorted<K: Ord + Clone, V: Clone>(m: &HashMap<K, V>) -> Vec<(K, V)> {
    let mut v: Vec<(K, V)> = m.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

fn sset<T: Ord + Clone>(s: &HashSet<T>) -> Vec<T> {
    let mut v: Vec<T> = s.iter().cloned().collect();
    v.sort();
    v
}

fn main() {
    let text = "one two three two three three four five five five five";
    let mut freq: HashMap<&str, usize> = HashMap::new();
    for w in text.split(' ') {
        *freq.entry(w).or_insert(0) += 1;
    }
    println!("freq: {:?}", sorted(&freq));

    let mut m: HashMap<String, Vec<u32>> = HashMap::new();
    for (i, w) in text.split(' ').enumerate() {
        m.entry(w.to_string())
            .and_modify(|v| v.push(i as u32))
            .or_insert_with(|| vec![i as u32 + 1000]);
    }
    println!("and_modify/or_insert_with: {:?}", sorted(&m));
    let mut lens: HashMap<String, usize> = HashMap::new();
    for w in ["alpha", "be", "alpha"] {
        lens.entry(w.to_string()).or_insert_with_key(|k| k.len() * 10);
    }
    println!("or_insert_with_key: {:?}", sorted(&lens));
    let mut defaults: HashMap<char, Vec<u8>> = HashMap::new();
    defaults.entry('x').or_default().push(1);
    defaults.entry('x').or_default().push(2);
    println!("or_default: {:?}", sorted(&defaults));

    use std::collections::hash_map::Entry;
    let mut inv: HashMap<&str, i32> = HashMap::from([("apple", 3), ("pear", 0)]);
    for (k, delta) in [("apple", -1), ("pear", -1), ("kiwi", 5), ("apple", -2)] {
        match inv.entry(k) {
            Entry::Occupied(mut o) => {
                let nv = *o.get() + delta;
                if nv <= 0 {
                    let (key, old) = o.remove_entry();
                    println!("  removed {} (was {})", key, old);
                } else {
                    *o.get_mut() = nv;
                }
            }
            Entry::Vacant(v) => {
                println!("  inserting {}", v.key());
                v.insert(delta);
            }
        }
    }
    println!("entry match: {:?}", sorted(&inv));

    let mut hm: HashMap<u32, &str> = (1..=8).map(|i| (i, if i % 2 == 0 { "even" } else { "odd" })).collect();
    println!("get {:?} get_key_value {:?} contains {} {}", hm.get(&3), hm.get_key_value(&4), hm.contains_key(&8), hm.contains_key(&9));
    if let Some(v) = hm.get_mut(&1) {
        *v = "ONE";
    }
    println!("insert existing returns {:?}, new returns {:?}", hm.insert(2, "TWO"), hm.insert(9, "nine"));
    println!("remove {:?} {:?} remove_entry {:?}", hm.remove(&3), hm.remove(&3), hm.remove_entry(&5));
    hm.retain(|k, v| *k < 8 && !v.is_empty());
    println!("retain: {:?}", sorted(&hm));
    let mut taken: Vec<(u32, &str)> = hm.extract_if(|k, _| k % 2 == 0).collect();
    taken.sort();
    println!("extract_if even: {:?} left {:?}", taken, sorted(&hm));
    let mut keys: Vec<_> = hm.keys().copied().collect();
    keys.sort();
    let mut vals: Vec<_> = hm.values().copied().collect();
    vals.sort();
    println!("keys {:?} values {:?} len {}", keys, vals, hm.len());
    for v in hm.values_mut() {
        *v = "x";
    }
    let mut into_keys: Vec<u32> = hm.clone().into_keys().collect();
    into_keys.sort();
    println!("values_mut -> {:?} into_keys {:?}", sorted(&hm), into_keys);
    let mut drained: Vec<(u32, &str)> = hm.drain().collect();
    drained.sort();
    println!("drain {:?} now empty {}", drained, hm.is_empty());
    let a: HashMap<i32, i32> = [(1, 2), (3, 4)].into_iter().collect();
    let b: HashMap<i32, i32> = [(3, 4), (1, 2)].into_iter().collect();
    println!("map eq ignoring order: {}", a == b);
    let mut ext = a.clone();
    ext.extend([(5, 6), (1, 20)]);
    println!("extend: {:?}", sorted(&ext));
    let by_val: BTreeMap<i32, i32> = ext.iter().map(|(k, v)| (*v, *k)).collect();
    println!("inverted: {:?}", by_val);
    let mut cap: HashMap<u8, u8> = HashMap::with_capacity(16);
    cap.insert(1, 1);
    cap.shrink_to_fit();
    println!("capacity >= len: {}", cap.capacity() >= cap.len());
    let mut arr = HashMap::from([(1, 'a'), (2, 'b'), (3, 'c')]);
    if let [Some(x), Some(y)] = arr.get_disjoint_mut([&1, &3]) {
        std::mem::swap(x, y);
    }
    println!("get_disjoint_mut swap: {:?}", sorted(&arr));

    // HashSet
    let s1: HashSet<i32> = (1..=6).collect();
    let s2: HashSet<i32> = (4..=9).collect();
    println!("union {:?}", sset(&s1.union(&s2).copied().collect()));
    println!("intersection {:?}", sset(&s1.intersection(&s2).copied().collect()));
    println!("difference {:?}", sset(&s1.difference(&s2).copied().collect()));
    println!("symmetric_difference {:?}", sset(&s1.symmetric_difference(&s2).copied().collect()));
    println!("operators: {:?} {:?} {:?} {:?}", sset(&(&s1 | &s2)), sset(&(&s1 & &s2)), sset(&(&s1 - &s2)), sset(&(&s1 ^ &s2)));
    let small: HashSet<i32> = [4, 5].into();
    println!("subset {} superset {} disjoint {} {}", small.is_subset(&s1), s1.is_superset(&small), s1.is_disjoint(&s2), s1.is_disjoint(&[100].into()));
    let mut hs: HashSet<String> = HashSet::new();
    println!("insert {} {} {}", hs.insert("a".into()), hs.insert("b".into()), hs.insert("a".into()));
    println!("contains {} get {:?} take {:?} take again {:?}", hs.contains("a"), hs.get("b").cloned(), hs.take("a"), hs.take("a"));
    println!("replace {:?} -> {:?}", hs.replace("b".to_string()), sset(&hs));
    hs.extend(["c", "dd", "eee"].map(String::from));
    hs.retain(|s| s.len() != 2);
    println!("extend+retain: {:?}", sset(&hs));
    let mut gone: Vec<String> = hs.extract_if(|s| s.starts_with('e')).collect();
    gone.sort();
    println!("extract_if: {:?} left {:?}", gone, sset(&hs));
    println!("remove {} {}", hs.remove("c"), hs.remove("zz"));
    let words: HashSet<&str> = "the cat and the hat".split(' ').collect();
    println!("unique words {}: {:?}", words.len(), sset(&words));
    let mut uniq_order = Vec::new();
    let mut seen = HashSet::new();
    for c in "mississippi".chars() {
        if seen.insert(c) {
            uniq_order.push(c);
        }
    }
    println!("first-seen order: {:?}", uniq_order);
    let pairs: HashSet<(i32, i32)> = [(1, 2), (2, 1), (1, 2)].into_iter().collect();
    println!("tuple set {}", pairs.len());
    let nested: HashMap<&str, HashSet<u8>> = HashMap::from([("a", HashSet::from([1, 2])), ("b", HashSet::new())]);
    let mut nk: Vec<(&str, Vec<u8>)> = nested.iter().map(|(k, v)| (*k, sset(v))).collect();
    nk.sort();
    println!("nested: {:?}", nk);
}
