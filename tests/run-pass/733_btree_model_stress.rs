// Randomized model checking of BTreeMap/BTreeSet against a sorted Vec model:
// insert/remove/get/range/range_mut/split_off/append/retain/first/last/pop,
// entry API, and iteration in both directions. Prints checksums.
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound::{Excluded, Included, Unbounded};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 { self.next() % n }
}

#[derive(Default)]
struct Model { v: Vec<(u32, u64)> }
impl Model {
    fn find(&self, k: u32) -> Result<usize, usize> { self.v.binary_search_by_key(&k, |e| e.0) }
    fn insert(&mut self, k: u32, val: u64) -> Option<u64> {
        match self.find(k) {
            Ok(i) => Some(std::mem::replace(&mut self.v[i].1, val)),
            Err(i) => { self.v.insert(i, (k, val)); None }
        }
    }
    fn remove(&mut self, k: u32) -> Option<u64> { self.find(k).ok().map(|i| self.v.remove(i).1) }
    fn get(&self, k: u32) -> Option<u64> { self.find(k).ok().map(|i| self.v[i].1) }
}

fn checksum<'a>(it: impl Iterator<Item = (&'a u32, &'a u64)>) -> u64 {
    it.fold(17u64, |h, (k, v)| h.wrapping_mul(31).wrapping_add(*k as u64).wrapping_mul(131).wrapping_add(*v))
}

fn main() {
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let mut map: BTreeMap<u32, u64> = BTreeMap::new();
    let mut model = Model::default();
    let mut mismatches = 0;
    let mut ops = [0u32; 8];
    for step in 0..20000 {
        let k = rng.below(3000) as u32;
        let op = rng.below(8) as usize;
        ops[op] += 1;
        match op {
            0 | 1 | 2 => {
                let v = rng.next() >> 20;
                if map.insert(k, v) != model.insert(k, v) { mismatches += 1; }
            }
            3 | 4 => {
                if map.remove(&k) != model.remove(k) { mismatches += 1; }
            }
            5 => {
                if map.get(&k).copied() != model.get(k) { mismatches += 1; }
            }
            6 => {
                let hi = k + rng.below(200) as u32;
                let a: Vec<(u32, u64)> = map.range(k..hi).map(|(a, b)| (*a, *b)).collect();
                let b: Vec<(u32, u64)> = model.v.iter().filter(|e| e.0 >= k && e.0 < hi).copied().collect();
                if a != b { mismatches += 1; }
                let ar: Vec<u32> = map.range((Excluded(k), Included(hi))).rev().map(|(a, _)| *a).collect();
                let br: Vec<u32> = model.v.iter().rev().filter(|e| e.0 > k && e.0 <= hi).map(|e| e.0).collect();
                if ar != br { mismatches += 1; }
            }
            _ => {
                *map.entry(k).or_insert(0) += 1;
                match model.find(k) { Ok(i) => model.v[i].1 += 1, Err(i) => model.v.insert(i, (k, 1)) }
            }
        }
        if step % 5000 == 4999 {
            let same = map.iter().map(|(a, b)| (*a, *b)).eq(model.v.iter().copied());
            println!("step {:>5}: len {} model {} same={} first={:?} last={:?}", step + 1, map.len(), model.v.len(), same, map.first_key_value().map(|e| *e.0), map.last_key_value().map(|e| *e.0));
        }
    }
    println!("ops {:?} mismatches {}", ops, mismatches);
    println!("checksum fwd {} rev {}", checksum(map.iter()), checksum(map.iter().rev()));

    for (_, v) in map.range_mut(1000..1100) { *v = 7; }
    let sevens = map.values().filter(|&&v| v == 7).count();
    println!("range_mut set {} sevens", sevens);
    let mut upper = map.split_off(&1500);
    println!("split_off: lower {} upper {} lower.last={:?} upper.first={:?}", map.len(), upper.len(), map.keys().next_back(), upper.keys().next());
    upper.retain(|k, v| k % 3 == 0 && *v != 7);
    println!("retain upper -> {}", upper.len());
    let ulen = upper.len();
    map.append(&mut upper);
    println!("append: map {} upper {} (moved {})", map.len(), upper.len(), ulen);
    let mut popped = Vec::new();
    for _ in 0..3 { popped.push(map.pop_first().unwrap().0); popped.push(map.pop_last().unwrap().0); }
    println!("popped {:?}", popped);
    let unb: Vec<u32> = map.range((Unbounded, Excluded(40))).map(|(k, _)| *k).collect();
    println!("below 40: {:?}", unb);
    let mut it = map.iter();
    let a = it.next().map(|e| *e.0);
    let z = it.next_back().map(|e| *e.0);
    println!("double ended: {:?} {:?} remaining {}", a, z, it.len());
    let extracted: Vec<u32> = map.extract_if(.., |k, _| *k % 500 == 0).map(|(k, _)| k).collect();
    println!("extract_if multiples of 500: {:?}", extracted);

    let mut set: BTreeSet<u32> = BTreeSet::new();
    let mut other: BTreeSet<u32> = BTreeSet::new();
    for _ in 0..3000 {
        set.insert(rng.below(1000) as u32);
        other.insert(rng.below(1000) as u32 + 500);
    }
    let inter = set.intersection(&other).count();
    let uni = set.union(&other).count();
    let diff = set.difference(&other).count();
    let sym = set.symmetric_difference(&other).count();
    println!("set {} other {} inter {} union {} diff {} sym {} consistent={}", set.len(), other.len(), inter, uni, diff, sym, uni == inter + diff + (other.len() - inter) && sym == uni - inter);
    println!("set range 495..505: {:?}", set.range(495..505).collect::<Vec<_>>());
    println!("subset {} disjoint {}", set.range(..10).copied().collect::<BTreeSet<_>>().is_subset(&set), set.is_disjoint(&other));
    let firsts: Vec<_> = set.iter().take(5).chain(set.iter().rev().take(5)).collect();
    println!("set ends {:?}", firsts);
}
