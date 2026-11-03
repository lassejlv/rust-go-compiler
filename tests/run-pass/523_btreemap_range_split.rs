// BTreeMap / BTreeSet: range with all bound kinds, split_off, append, retain, pop_first/last, entry
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound::{Excluded, Included, Unbounded};

fn main() {
    let mut m: BTreeMap<i32, String> = (1..=10).map(|i| (i * 10, format!("v{}", i))).collect();
    println!("{:?}", m.range(25..55).collect::<Vec<_>>());
    println!("{:?}", m.range(..=30).map(|(k, _)| *k).collect::<Vec<_>>());
    println!("{:?}", m.range(90..).map(|(k, _)| k).collect::<Vec<_>>());
    println!("{:?}", m.range((Excluded(20), Included(50))).map(|(k, _)| k).collect::<Vec<_>>());
    println!("{:?}", m.range((Unbounded, Excluded(20))).collect::<Vec<_>>());
    println!("{:?}", m.range(40..=40).next());
    println!("{:?}", m.range(41..45).next());
    println!("{:?}", m.range(..).rev().take(2).collect::<Vec<_>>());
    for (_, v) in m.range_mut(30..=50) {
        v.push('!');
    }
    println!("{:?}", m.get(&40));
    let floor = m.range(..=67).next_back().map(|(k, _)| *k);
    let ceil = m.range(67..).next().map(|(k, _)| *k);
    println!("floor={:?} ceil={:?}", floor, ceil);

    let hi = m.split_off(&60);
    println!("lo={:?}", m.keys().collect::<Vec<_>>());
    println!("hi={:?}", hi.keys().collect::<Vec<_>>());
    let mut hi = hi;
    m.append(&mut hi);
    println!("after append {} {}", m.len(), hi.len());
    m.retain(|k, v| k % 20 == 0 || v.ends_with('!'));
    println!("{:?}", m);
    println!("{:?} {:?}", m.first_key_value(), m.last_key_value());
    println!("{:?}", m.pop_first());
    println!("{:?}", m.pop_last());
    if let Some(mut e) = m.last_entry() {
        e.get_mut().push_str("-last");
    }
    if let Some(e) = m.first_entry() {
        println!("removing first {:?}", e.remove_entry());
    }
    println!("{:?}", m);
    *m.entry(45).or_insert_with(String::new) += "new";
    m.entry(40).and_modify(|s| s.make_ascii_uppercase());
    println!("{:?}", m);

    let mut s: BTreeSet<&str> = ["kiwi", "apple", "fig", "banana", "cherry", "date"].into_iter().collect();
    println!("{:?}", s.range("b".."d").collect::<Vec<_>>());
    println!("{:?}", s.range::<str, _>((Excluded("cherry"), Unbounded)).collect::<Vec<_>>());
    let tail = s.split_off("date");
    println!("{:?} | {:?}", s, tail);
    println!("{:?} {:?}", s.first(), tail.last());
    let a: BTreeSet<i32> = (1..=8).collect();
    let b: BTreeSet<i32> = (5..=12).step_by(2).collect();
    println!("{:?}", a.intersection(&b).collect::<Vec<_>>());
    println!("{:?}", a.difference(&b).collect::<Vec<_>>());
    println!("{:?}", a.symmetric_difference(&b).collect::<Vec<_>>());
    println!("{:?}", a.union(&b).collect::<Vec<_>>());
    println!("{:?}", &a & &b);
    println!("{:?}", &a - &b);
    println!("{} {}", a.is_subset(&b), b.iter().filter(|x| a.contains(x)).count());
    let mut c = a.clone();
    c.retain(|x| x % 3 != 0);
    let pl = c.pop_last();
    println!("{:?} {:?}", c, pl);
    let mut by_len: BTreeMap<(usize, &str), ()> = BTreeMap::new();
    for w in ["aa", "b", "ccc", "ab", "a"] {
        by_len.insert((w.len(), w), ());
    }
    println!("{:?}", by_len.keys().collect::<Vec<_>>());
    println!("{:?}", by_len.range((2, "")..(3, "")).map(|(k, _)| k.1).collect::<Vec<_>>());
    let mut counts = BTreeMap::new();
    for ch in "mississippi".chars() {
        *counts.entry(ch).or_insert(0usize) += 1;
    }
    let top = counts.iter().max_by_key(|(c, n)| (**n, std::cmp::Reverse(**c))).unwrap();
    println!("{:?} top={:?}", counts, top);
    let into_vals: Vec<usize> = counts.into_values().collect();
    println!("{:?}", into_vals);
    let mut inv: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    for (ln, line) in ["a b c", "b c d", "c d e", "a e"].iter().enumerate() {
        for w in line.split(' ') {
            inv.entry(w.to_string()).or_default().insert(ln);
        }
    }
    for (w, lines) in inv.range("b".to_string()..="d".to_string()) {
        println!("{} -> {:?}", w, lines);
    }
    let both: BTreeSet<usize> = inv["a"].intersection(&inv["e"]).copied().collect();
    println!("{:?}", both);
    let mut ages: BTreeMap<u32, &str> = [(30, "c"), (10, "a"), (20, "b"), (40, "d")].into_iter().collect();
    let young: Vec<_> = ages.extract_if(.., |k, _| *k < 25).collect();
    println!("{:?} {:?}", young, ages);
    let iter_mut_sum: u32 = ages.iter_mut().map(|(k, v)| { *v = "z"; *k }).sum();
    println!("{} {:?}", iter_mut_sum, ages.values().collect::<Vec<_>>());
}
