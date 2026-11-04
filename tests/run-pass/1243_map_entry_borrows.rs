use std::collections::{BTreeMap, HashMap};
use std::collections::hash_map::Entry;
fn count<'a>(words: &[&'a str]) -> HashMap<&'a str, usize> {
    let mut m = HashMap::new();
    for w in words { *m.entry(*w).or_insert(0) += 1; }
    m
}
fn index<'a>(text: &'a str) -> BTreeMap<char, Vec<&'a str>> {
    let mut m: BTreeMap<char, Vec<&'a str>> = BTreeMap::new();
    for w in text.split(' ') {
        let tmp = w.to_uppercase();
        m.entry(w.chars().next().unwrap()).or_insert_with(Vec::new).push(w);
        m.entry('#').and_modify(|v| v.push(w)).or_insert_with(|| { let _ = tmp.len(); vec![w] });
    }
    m
}
fn main() {
    let text = String::from("apple avocado banana blue cherry");
    let words: Vec<&str> = text.split(' ').collect();
    let c = count(&words);
    let mut cv: Vec<_> = c.into_iter().collect();
    cv.sort();
    println!("{:?}", cv);
    println!("{:?}", index(&text));
    let mut m: HashMap<String, &str> = HashMap::new();
    let local = String::from("val");
    match m.entry("k".to_string()) { Entry::Vacant(e) => { e.insert(&local); } Entry::Occupied(mut e) => { e.insert("x"); } }
    let r = m.entry("k".to_string()).or_insert("default");
    println!("{}", r);
    let mut bm: BTreeMap<u8, String> = BTreeMap::new();
    bm.entry(1).or_default().push_str("one");
    let e = bm.entry(2).or_insert_with(|| "two".into());
    e.push('!');
    println!("{:?} {:?}", m, bm);
}
