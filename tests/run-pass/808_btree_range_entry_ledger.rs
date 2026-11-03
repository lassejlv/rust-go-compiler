use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound::{Excluded, Included, Unbounded};

#[derive(Debug, Default, Clone)]
struct Day { deposits: u64, withdrawals: u64, txns: Vec<i64> }

fn main() {
    let mut ledger: BTreeMap<u32, Day> = BTreeMap::new();
    let events: [(u32, i64); 14] = [(20240105, 500), (20240103, -20), (20240105, -100), (20240110, 75), (20240101, 1000), (20240115, -300), (20240110, -5), (20240120, 60), (20240103, 40), (20240201, -999), (20240115, 12), (20240131, 7), (20240201, 1), (20240105, 3)];
    for &(day, amt) in &events {
        let e = ledger.entry(day).or_default();
        if amt >= 0 { e.deposits += amt as u64 } else { e.withdrawals += (-amt) as u64 }
        e.txns.push(amt);
    }
    for (d, day) in &ledger {
        println!("{} +{} -{} {:?}", d, day.deposits, day.withdrawals, day.txns);
    }
    println!("-- range [0105, 0115]");
    for (d, day) in ledger.range(20240105..=20240115) { println!("{} n={}", d, day.txns.len()); }
    println!("-- range (0105, 0131)");
    for (d, _) in ledger.range((Excluded(20240105), Excluded(20240131))) { print!("{} ", d); }
    println!();
    println!("-- range ..0104 rev");
    for (d, _) in ledger.range(..20240104).rev() { print!("{} ", d); }
    println!();
    println!("-- range (Unbounded, Included 0110)");
    let total: i64 = ledger.range((Unbounded, Included(20240110))).flat_map(|(_, d)| d.txns.iter()).sum();
    println!("net through 0110 = {}", total);
    println!("first {:?} last {:?}", ledger.first_key_value().map(|(k, _)| k), ledger.last_key_value().map(|(k, v)| (k, v.txns.len())));
    let next_after = ledger.range(20240111..).next().map(|(k, _)| *k);
    let prev_before = ledger.range(..20240111).next_back().map(|(k, _)| *k);
    println!("next after 0111 {:?}, prev before {:?}", next_after, prev_before);

    // entry API variants
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for w in "the cat and the hat and the bat sat".split_whitespace() {
        *counts.entry(w.to_string()).or_insert(0) += 1;
    }
    counts.entry("cat".into()).and_modify(|c| *c += 10).or_insert(99);
    counts.entry("dog".into()).and_modify(|c| *c += 10).or_insert(99);
    let k = counts.entry("eel".to_string()).or_insert_with_key(|k| k.len());
    *k += 1;
    use std::collections::btree_map::Entry;
    match counts.entry("hat".into()) {
        Entry::Occupied(o) => { let (k, v) = o.remove_entry(); println!("removed {}={}", k, v); }
        Entry::Vacant(v) => { v.insert(0); }
    }
    match counts.entry("zebra".into()) {
        Entry::Occupied(_) => {}
        Entry::Vacant(v) => { println!("vacant key {}", v.key()); v.insert(26); }
    }
    println!("{:?}", counts);
    let popped = counts.pop_first();
    let popped_last = counts.pop_last();
    println!("pop_first {:?} pop_last {:?}", popped, popped_last);
    counts.retain(|k, v| k.len() == 3 && *v < 50);
    println!("retained {:?}", counts);
    let mut hi = counts.split_off("eel");
    println!("lo {:?} hi {:?}", counts, hi);
    hi.append(&mut counts);
    println!("appended {:?} lo now empty={}", hi, counts.is_empty());
    if let Some(v) = hi.get_mut("the") { *v *= 7; }
    println!("keys {:?} values {:?}", hi.keys().collect::<Vec<_>>(), hi.values().copied().collect::<Vec<_>>());
    let mut vm: Vec<_> = hi.values_mut().collect();
    for v in vm.iter_mut() { **v += 1; }
    println!("{:?}", hi);
    let inverted: BTreeMap<usize, Vec<&String>> = hi.iter().fold(BTreeMap::new(), |mut m, (k, v)| { m.entry(*v).or_insert_with(Vec::new).push(k); m });
    println!("inverted {:?}", inverted);

    // BTreeSet ops
    let a: BTreeSet<i32> = (1..20).filter(|x| x % 2 == 0).collect();
    let b: BTreeSet<i32> = (1..20).filter(|x| x % 3 == 0).collect();
    println!("union {:?}", a.union(&b).collect::<Vec<_>>());
    println!("inter {:?}", a.intersection(&b).collect::<Vec<_>>());
    println!("diff {:?}", a.difference(&b).collect::<Vec<_>>());
    println!("symdiff {:?}", a.symmetric_difference(&b).cloned().collect::<BTreeSet<_>>());
    println!("range 5..13 {:?}", a.range(5..13).collect::<Vec<_>>());
    println!("subset {} disjoint {}", [6, 12].iter().copied().collect::<BTreeSet<_>>().is_subset(&b), a.is_disjoint(&b));
    let mut s = a.clone();
    println!("insert dup {} remove {} take {:?} replace {:?}", s.insert(4), s.remove(&4), s.take(&6), s.replace(8));
    println!("first {:?} last {:?} {:?}", s.first(), s.last(), &s | &b);
    let ceil = s.range(11..).next();
    let floor = s.range(..=11).next_back();
    println!("ceil(11) {:?} floor(11) {:?}", ceil, floor);
    let strs: BTreeSet<&str> = ["pear", "apple", "fig", "apricot", "banana", "avocado"].into_iter().collect();
    let a_words: Vec<_> = strs.range("a".."b").collect();
    println!("a-words {:?}", a_words);
}
