use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};
use std::cmp::Reverse;
use std::ops::Bound::{Excluded, Included, Unbounded};

fn main() {
    let mut m: BTreeMap<i32, String> = BTreeMap::new();
    for i in [5, 1, 9, 3, 7, 11, 2] {
        m.insert(i, format!("v{}", i));
    }
    println!("{:?}", m.range(3..8).collect::<Vec<_>>());
    println!("{:?}", m.range(..=3).map(|(k, _)| *k).collect::<Vec<_>>());
    println!("{:?}", m.range((Excluded(3), Included(9))).map(|(k, _)| k).collect::<Vec<_>>());
    println!("{:?}", m.range((Unbounded, Excluded(5))).rev().next());
    println!("{:?} {:?}", m.first_key_value(), m.last_key_value());
    for (_, v) in m.range_mut(7..) {
        v.push('!');
    }
    println!("{:?}", m.values().cloned().collect::<Vec<_>>());
    match m.entry(4) {
        Entry::Vacant(e) => {
            e.insert("four".into());
        }
        Entry::Occupied(_) => unreachable!(),
    }
    match m.entry(5) {
        Entry::Occupied(mut e) => {
            e.get_mut().push_str("-mod");
            println!("occupied key {}", e.key());
        }
        Entry::Vacant(_) => {}
    }
    if let Entry::Occupied(e) = m.entry(1) {
        println!("removed {:?}", e.remove_entry());
    }
    *m.entry(100).or_insert_with(|| "h".into()) += "undred";
    m.entry(2).and_modify(|v| v.make_ascii_uppercase()).or_default();
    m.entry(50).or_default();
    println!("{:?}", m);
    let mut counts: BTreeMap<char, usize> = BTreeMap::new();
    for c in "mississippi".chars() {
        *counts.entry(c).or_insert(0) += 1;
    }
    println!("{:?}", counts);
    println!("{:?} {:?}", m.pop_first(), m.pop_last());
    let hi = m.split_off(&7);
    println!("{:?} | {:?}", m.keys().collect::<Vec<_>>(), hi.keys().collect::<Vec<_>>());
    m.retain(|k, _| k % 2 == 1);
    println!("{:?}", m);
    let mut m2: BTreeMap<&str, i32> = [("b", 2), ("a", 1)].into();
    m2.extend([("c", 3), ("a", 10)]);
    println!("{:?} {} {:?}", m2, m2.len(), m2.get("a"));
    println!("{:?}", m2.iter().next_back());
    println!("{:?}", m2.into_iter().rev().collect::<Vec<_>>());

    let s1: BTreeSet<i32> = (1..10).filter(|x| x % 2 == 0).collect();
    let s2: BTreeSet<i32> = (1..10).filter(|x| x % 3 == 0).collect();
    println!("{:?}", s1.intersection(&s2).collect::<Vec<_>>());
    println!("{:?}", s1.union(&s2).collect::<Vec<_>>());
    println!("{:?}", s1.difference(&s2).collect::<Vec<_>>());
    println!("{:?}", s1.symmetric_difference(&s2).collect::<Vec<_>>());
    println!("{:?} {:?}", s1.range(3..).next(), s1.iter().rev().nth(1));
    println!("{} {}", s1.is_subset(&s2), s1.is_disjoint(&BTreeSet::from([1, 3])));
    println!("{:?}", &s1 | &s2);
    println!("{:?}", &s1 - &s2);
    println!("{:?} {:?}", s1.first(), s1.last());

    let mut dq: VecDeque<i32> = (1..=6).collect();
    dq.rotate_left(2);
    println!("{:?}", dq);
    dq.rotate_right(4);
    println!("{:?}", dq);
    dq.push_front(0);
    dq.push_back(7);
    println!("{:?} {:?}", dq.front(), dq.back());
    let (a, b) = dq.as_slices();
    println!("{}", a.len() + b.len());
    dq.make_contiguous().sort_by(|a, b| b.cmp(a));
    println!("{:?}", dq);
    println!("{:?}", dq.as_slices().0);
    dq.retain(|&x| x != 3);
    println!("{:?} {:?}", dq.pop_front(), dq.pop_back());
    println!("{:?} {:?}", dq.get(1), dq.iter().rev().collect::<Vec<_>>());
    dq.insert(2, 99);
    let rm0 = dq.remove(0);
    println!("{:?} {:?}", dq, rm0);
    let drained: Vec<i32> = dq.drain(1..3).collect();
    println!("{:?} {:?}", drained, dq);
    dq.extend([1, 2]);
    dq.truncate(3);
    let other = dq.split_off(1);
    println!("{:?} {:?}", dq, other);
    let mut dq2: VecDeque<_> = VecDeque::with_capacity(2);
    for i in 0..10 {
        if i % 3 == 0 {
            dq2.push_front(i);
        } else {
            dq2.push_back(i);
        }
        if dq2.len() > 4 {
            dq2.pop_front();
        }
    }
    println!("{:?} {}", dq2, dq2.contains(&8));
    dq2.swap(0, 3);
    for x in dq2.iter_mut() {
        *x *= 2;
    }
    println!("{:?} {:?}", dq2, dq2.range(1..3).collect::<Vec<_>>());
    let v: Vec<i32> = dq2.into();
    println!("{:?}", v);

    let mut heap = BinaryHeap::new();
    for x in [5, 1, 8, 3, 9, 2] {
        heap.push(Reverse(x));
    }
    let mut out = vec![];
    while let Some(Reverse(x)) = heap.pop() {
        out.push(x);
        if out.len() == 3 {
            break;
        }
    }
    println!("{:?} {:?}", out, heap.peek());
    let mut h2: BinaryHeap<(u32, &str)> = vec![(2, "b"), (5, "e"), (1, "a")].into();
    h2.push((3, "c"));
    println!("{:?}", h2.clone().into_sorted_vec());
    if let Some(mut top) = h2.peek_mut() {
        top.0 = 0;
    }
    println!("{:?}", h2.pop());
    println!("{}", h2.len());
    let mut v = h2.into_vec();
    v.sort();
    println!("{:?}", v);
}
