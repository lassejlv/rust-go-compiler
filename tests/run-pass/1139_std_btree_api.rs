// Breadth test of BTreeMap and BTreeSet APIs.
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound::{Excluded, Included, Unbounded};

fn main() {
    let mut m: BTreeMap<i32, String> = (1..=10).map(|i| (i * 10, format!("v{}", i))).collect();
    println!("first {:?} last {:?}", m.first_key_value(), m.last_key_value());
    println!("range 25..=60: {:?}", m.range(25..=60).map(|(k, _)| *k).collect::<Vec<_>>());
    println!("range ..30: {:?}", m.range(..30).collect::<Vec<_>>());
    println!("range (Excluded 50, Unbounded): {:?}", m.range((Excluded(50), Unbounded)).map(|(k, _)| k).collect::<Vec<_>>());
    println!("range (Included 35, Excluded 70) rev: {:?}", m.range((Included(35), Excluded(70))).rev().map(|(k, _)| k).collect::<Vec<_>>());
    println!("floor of 47: {:?}  ceiling of 47: {:?}", m.range(..=47).next_back().map(|(k, _)| k), m.range(47..).next().map(|(k, _)| k));
    for (_, v) in m.range_mut(40..=60) {
        v.push('!');
    }
    println!("range_mut: {:?}", m.range(30..=70).map(|(_, v)| v.as_str()).collect::<Vec<_>>());
    println!("pop_first {:?} pop_last {:?}", m.pop_first(), m.pop_last());
    if let Some(mut e) = m.first_entry() {
        e.get_mut().push_str("-first");
    }
    if let Some(e) = m.last_entry() {
        let k = *e.key();
        println!("last_entry key {} removed {:?}", k, e.remove());
    }
    let mut hi = m.split_off(&55);
    println!("split_off(55): lo {:?}\n  hi {:?}", m.keys().collect::<Vec<_>>(), hi.keys().collect::<Vec<_>>());
    hi.insert(5, "five".into());
    m.append(&mut hi);
    println!("append: {:?} hi empty {}", m.keys().collect::<Vec<_>>(), hi.is_empty());
    m.retain(|k, v| k % 20 != 0 || v.ends_with('!'));
    println!("retain: {:?}", m);
    let taken: BTreeMap<i32, String> = m.extract_if(.., |k, _| *k < 40).collect();
    println!("extract_if: {:?} left {:?}", taken, m.keys().collect::<Vec<_>>());
    *m.entry(1000).or_insert_with(String::new) += "new";
    m.entry(50).and_modify(|v| v.make_ascii_uppercase()).or_insert_with(|| "missing".into());
    println!("entry: {:?}", m);
    println!("into_keys {:?} into_values {:?}", m.clone().into_keys().collect::<Vec<_>>(), m.clone().into_values().collect::<Vec<_>>());
    println!("iter rev nth(1): {:?}", m.iter().rev().nth(1));
    println!("len {} contains 50 {} get 51 {:?}", m.len(), m.contains_key(&50), m.get(&51));

    // char frequency ordered
    let mut freq: BTreeMap<char, usize> = BTreeMap::new();
    for c in "hello wonderful world".chars().filter(|c| c.is_alphabetic()) {
        *freq.entry(c).or_default() += 1;
    }
    let top = freq.iter().max_by_key(|&(c, n)| (*n, std::cmp::Reverse(*c))).unwrap();
    println!("freq {:?} top {:?}", freq, top);

    // composite keys and range over tuples
    let mut events: BTreeMap<(u32, u32), &str> = BTreeMap::new();
    for (d, h, e) in [(1, 9, "standup"), (1, 14, "review"), (2, 10, "deploy"), (2, 9, "standup"), (3, 16, "retro")] {
        events.insert((d, h), e);
    }
    println!("day 2: {:?}", events.range((2, 0)..(3, 0)).map(|((_, h), e)| format!("{}h {}", h, e)).collect::<Vec<_>>());
    println!("after (1,12): {:?}", events.range((1, 12)..).next());

    // BTreeSet
    let a: BTreeSet<u32> = [5, 1, 9, 3, 7, 3].into_iter().collect();
    let b: BTreeSet<u32> = (4..=8).collect();
    println!("a {:?} first {:?} last {:?}", a, a.first(), a.last());
    println!("union {:?}", a.union(&b).collect::<Vec<_>>());
    println!("intersection {:?}", a.intersection(&b).collect::<Vec<_>>());
    println!("difference {:?}", a.difference(&b).collect::<Vec<_>>());
    println!("sym diff {:?}", a.symmetric_difference(&b).collect::<Vec<_>>());
    println!("ops {:?} {:?}", &a | &b, &a - &b);
    println!("range 2..8 {:?}", a.range(2..8).collect::<Vec<_>>());
    println!("subset {} superset {} disjoint {}", BTreeSet::from([3, 5]).is_subset(&a), a.is_superset(&BTreeSet::from([1])), a.is_disjoint(&BTreeSet::from([2, 4])));
    let mut s = a.clone();
    println!("pop_first {:?} pop_last {:?} -> {:?}", s.pop_first(), s.pop_last(), s);
    let hi = s.split_off(&5);
    println!("split_off {:?} {:?}", s, hi);
    s.extend(hi.iter().map(|x| x * 10));
    s.retain(|x| *x != 50);
    println!("extend/retain {:?}", s);
    println!("replace {:?} take {:?} get {:?}", s.replace(70), s.take(&3), s.get(&70));
    let removed: Vec<u32> = s.extract_if(.., |x| *x > 60).collect();
    println!("extract_if {:?} left {:?}", removed, s);
    let words: BTreeSet<&str> = "the quick brown fox jumps over the lazy dog".split(' ').collect();
    println!("{:?}", words);
    println!("words starting with t..: {:?}", words.range("t".."u").collect::<Vec<_>>());
    let nested: BTreeSet<Vec<u8>> = [vec![2], vec![1, 9], vec![1], vec![]].into_iter().collect();
    println!("{:?}", nested);
    let opt: BTreeSet<Option<i8>> = [Some(3), None, Some(-1)].into();
    println!("{:?} cmp {:?}", opt, BTreeSet::from([1, 2]).cmp(&BTreeSet::from([1, 3])));
    let m2: BTreeMap<&str, Vec<i32>> = BTreeMap::from([("b", vec![2]), ("a", vec![1, 1])]);
    println!("{:?} {}", m2, m2 < BTreeMap::from([("a", vec![1, 2])]));
}
