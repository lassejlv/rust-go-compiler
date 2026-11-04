// Keys and values of maps whose key and value types are the same type with
// different lifetimes: the borrow checker must follow each one's regions
// (by the iterator impls' `Item` types), not merge them by type.
use std::collections::{BTreeMap, BTreeSet, HashMap};

fn keys<'a>(m: &HashMap<&'static str, &'a str>) -> Vec<&'static str> {
    let mut v: Vec<&'static str> = m.keys().copied().collect();
    v.sort();
    v
}

fn values<'a>(m: &HashMap<&'a str, &'static str>) -> Vec<&'static str> {
    let mut v: Vec<&'static str> = m.values().copied().collect();
    v.sort();
    v
}

fn values_cloned<'a>(m: &BTreeMap<&'a str, &'static str>) -> Vec<&'static str> {
    m.values().cloned().collect()
}

fn into_keys<'a>(m: BTreeMap<&'static str, &'a str>) -> Vec<&'static str> {
    m.into_keys().collect()
}

fn into_values<'a>(m: BTreeMap<&'a str, &'static str>) -> Vec<&'static str> {
    m.into_values().collect()
}

fn into_iter_keys<'a>(m: BTreeMap<&'static str, &'a str>) -> Vec<&'static str> {
    m.into_iter().map(|(k, _)| k).collect()
}

fn iter_keys<'a>(m: &BTreeMap<&'static str, &'a str>) -> Vec<&'static str> {
    m.iter().map(|(k, _)| *k).collect()
}

fn key_set<'a>(m: &HashMap<&'static str, &'a str>) -> BTreeSet<&'static str> {
    m.keys().copied().collect()
}

fn first_key<'a>(m: &BTreeMap<&'static str, &'a str>) -> Option<&'static str> {
    m.keys().next().copied()
}

fn max_key<'a>(m: &BTreeMap<&'static str, &'a str>) -> Option<&'static str> {
    m.keys().copied().max()
}

fn last_value<'a>(m: &BTreeMap<&'a str, &'static str>) -> Option<&'static str> {
    m.values().last().copied()
}

fn loop_values<'a>(m: &BTreeMap<&'a str, &'static str>) -> Vec<&'static str> {
    let mut out = Vec::new();
    for (_k, v) in m {
        out.push(*v);
    }
    out
}

fn next_value<'a>(m: &HashMap<&'a str, &'static str>) -> Option<&'static str> {
    let mut it = m.values();
    let x = it.next()?;
    Some(*x)
}

fn range_keys<'a>(m: &BTreeMap<&'static str, &'a str>) -> Vec<&'static str> {
    m.range("a".."z").map(|(k, _)| *k).collect()
}

fn values_mut<'a>(m: &mut HashMap<&'a str, &'static str>) -> Vec<&'static str> {
    m.values_mut().map(|v| *v).collect()
}

fn main() {
    let owned = String::from("dynamic");
    let mut hm = HashMap::new();
    hm.insert("key", owned.as_str());
    let mut hm2 = HashMap::new();
    hm2.insert(owned.as_str(), "value");
    let mut bm = BTreeMap::new();
    bm.insert("k1", owned.as_str());
    bm.insert("k2", "x");
    let mut bm2 = BTreeMap::new();
    bm2.insert(owned.as_str(), "v1");

    let ks = keys(&hm);
    let vs = values(&hm2);
    let vc = values_cloned(&bm2);
    let ik = into_keys(bm.clone());
    let iv = into_values(bm2.clone());
    let it = into_iter_keys(bm.clone());
    let rk = iter_keys(&bm);
    let set = key_set(&hm);
    let fk = first_key(&bm);
    let mk = max_key(&bm);
    let lv = last_value(&bm2);
    let lp = loop_values(&bm2);
    let nv = next_value(&hm2);
    let rg = range_keys(&bm);
    let vm = values_mut(&mut hm2.clone());
    drop(hm);
    drop(hm2);
    drop(bm);
    drop(bm2);
    drop(owned);
    // all of these only hold 'static strings
    println!("{:?} {:?} {:?}", ks, vs, vc);
    println!("{:?} {:?} {:?} {:?}", ik, iv, it, rk);
    println!("{:?} {:?} {:?} {:?}", set, fk, mk, lv);
    println!("{:?} {:?} {:?} {:?}", lp, nv, rg, vm);
}
