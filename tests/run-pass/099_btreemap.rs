use std::collections::{BTreeMap, BTreeSet};

fn main() {
    let mut m = BTreeMap::new();
    for (k, v) in [("pear", 3), ("apple", 7), ("fig", 1), ("banana", 4), ("cherry", 9)] {
        m.insert(k, v);
    }
    println!("{:?}", m);
    for (k, v) in &m {
        println!("  {} = {}", k, v);
    }
    println!("first = {:?}", m.first_key_value());
    println!("last = {:?}", m.last_key_value());
    println!("get fig = {:?}", m.get("fig"));
    let range: Vec<(&&str, &i32)> = m.range("b".."f").collect();
    println!("range b..f = {:?}", range);
    *m.entry("fig").or_insert(0) += 10;
    m.entry("grape").or_insert(2);
    println!("keys = {:?}", m.keys().collect::<Vec<_>>());
    println!("values = {:?}", m.values().collect::<Vec<_>>());
    m.remove("apple");
    println!("after remove = {:?}", m);
    let mut years: BTreeMap<u32, Vec<&str>> = BTreeMap::new();
    let events = [(1969, "moon landing"), (1989, "web proposed"), (1969, "woodstock"), (1991, "linux"), (1989, "berlin wall")];
    for (y, e) in events {
        years.entry(y).or_default().push(e);
    }
    for (y, es) in &years {
        println!("{}: {}", y, es.join(", "));
    }
    let after_1980: Vec<&u32> = years.range(1980..).map(|(k, _)| k).collect();
    println!("after 1980: {:?}", after_1980);
    let rev: Vec<&u32> = years.keys().rev().collect();
    println!("reversed keys: {:?}", rev);
    let mut counts: BTreeMap<char, usize> = BTreeMap::new();
    for c in "hello world".chars().filter(|c| c.is_alphabetic()) {
        *counts.entry(c).or_insert(0) += 1;
    }
    let line: Vec<String> = counts.iter().map(|(c, n)| format!("{}{}", c, n)).collect();
    println!("counts: {}", line.join(" "));
    let mut set = BTreeSet::new();
    for x in [5, 1, 9, 3, 7, 3, 1] {
        set.insert(x);
    }
    println!("set = {:?}", set);
    println!("set range 3..=7 = {:?}", set.range(3..=7).collect::<Vec<_>>());
    println!("first = {:?}, last = {:?}", set.first(), set.last());
    let other: BTreeSet<i32> = [3, 4, 5].into_iter().collect();
    println!("intersection = {:?}", set.intersection(&other).collect::<Vec<_>>());
    println!("union = {:?}", set.union(&other).collect::<Vec<_>>());
    println!("pop_first = {:?}, now {:?}", set.pop_first(), set);
    let mut scores: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    for (n, s) in [("zed", 70), ("amy", 95), ("zed", 80), ("amy", 85), ("max", 60)] {
        scores.entry(n.to_string()).or_insert_with(Vec::new).push(s);
    }
    for (name, s) in &scores {
        let avg = s.iter().sum::<u32>() as f64 / s.len() as f64;
        println!("{:<4} {:?} avg {:.1}", name, s, avg);
    }
}
