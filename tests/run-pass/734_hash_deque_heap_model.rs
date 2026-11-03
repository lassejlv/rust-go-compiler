// Randomized model checks for HashMap/HashSet (with resize, removal, entry
// API, retain, drain), VecDeque (wrap-around, rotate, insert/remove at
// positions, range, make_contiguous) and BinaryHeap (peek_mut, into_sorted_vec).
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::collections::binary_heap::PeekMut;

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
}

fn main() {
    let mut r = Lcg(42);
    // HashMap vs Vec<Option<u64>> model indexed by key
    let mut hm: HashMap<u32, u64> = HashMap::new();
    let mut model: Vec<Option<u64>> = vec![None; 5000];
    let mut bad = 0;
    for i in 0..30000u64 {
        let k = r.next() % 5000;
        match r.next() % 6 {
            0 | 1 => { if hm.insert(k, i) != model[k as usize].replace(i) { bad += 1; } }
            2 => { if hm.remove(&k) != model[k as usize].take() { bad += 1; } }
            3 => {
                let e = hm.entry(k).and_modify(|v| *v += 1).or_insert(1);
                let m = model[k as usize].get_or_insert(0);
                if model_prev_matches(*e, m) { bad += 1; }
            }
            4 => { if hm.get(&k).copied() != model[k as usize] { bad += 1; } }
            _ => { if hm.contains_key(&k) != model[k as usize].is_some() { bad += 1; } }
        }
    }
    let model_len = model.iter().filter(|x| x.is_some()).count();
    let mut keys: Vec<u32> = hm.keys().copied().collect();
    keys.sort_unstable();
    let model_keys: Vec<u32> = (0..5000).filter(|&k| model[k as usize].is_some()).collect();
    println!("hashmap len {} model {} keys equal {} bad {}", hm.len(), model_len, keys == model_keys, bad);
    let sum: u64 = hm.values().sum();
    let msum: u64 = model.iter().flatten().sum();
    println!("value sums equal: {}", sum == msum);
    hm.retain(|k, v| k % 7 == 0 || *v % 2 == 0);
    let expect = model.iter().enumerate().filter(|(k, v)| v.map_or(false, |v| k % 7 == 0 || v % 2 == 0)).count();
    println!("after retain {} expected {}", hm.len(), expect);
    let mut drained: Vec<(u32, u64)> = hm.drain().filter(|(k, _)| *k < 50).collect();
    drained.sort();
    println!("drained small keys {:?} now empty {}", drained.iter().map(|e| e.0).collect::<Vec<_>>(), hm.is_empty());
    hm.shrink_to_fit();
    hm.extend((0..10).map(|i| (i, i as u64 * 3)));
    let mut ex: Vec<_> = hm.iter().filter(|(k, _)| **k > 6).map(|(k, v)| (*k, *v)).collect();
    ex.sort();
    println!("extend then filter: {:?}", ex);

    let a: HashSet<u32> = (0..2000).map(|_| r.next() % 3000).collect();
    let b: HashSet<u32> = (0..2000).map(|_| r.next() % 3000).collect();
    let mut i: Vec<_> = a.intersection(&b).copied().collect();
    i.sort();
    let union = a.union(&b).count();
    println!("hashset a {} b {} inter {} union {} check {}", a.len(), b.len(), i.len(), union, union + i.len() == a.len() + b.len());
    println!("first inter {:?}", &i[..5]);

    // VecDeque vs Vec model
    let mut dq: VecDeque<u32> = VecDeque::with_capacity(4);
    let mut vm: Vec<u32> = Vec::new();
    let mut dbad = 0;
    for step in 0..20000u32 {
        match r.next() % 9 {
            0 | 1 => { dq.push_back(step); vm.push(step); }
            2 | 3 => { dq.push_front(step); vm.insert(0, step); }
            4 => { if dq.pop_front() != (if vm.is_empty() { None } else { Some(vm.remove(0)) }) { dbad += 1; } }
            5 => { if dq.pop_back() != vm.pop() { dbad += 1; } }
            6 if !vm.is_empty() => {
                let i = r.next() as usize % vm.len();
                dq.insert(i, step); vm.insert(i, step);
            }
            7 if !vm.is_empty() => {
                let i = r.next() as usize % vm.len();
                if dq.remove(i) != Some(vm.remove(i)) { dbad += 1; }
            }
            8 if !vm.is_empty() => {
                let n = r.next() as usize % vm.len();
                dq.rotate_left(n); vm.rotate_left(n);
            }
            _ => {}
        }
        if step % 4000 == 0 && !dq.iter().eq(vm.iter()) { dbad += 1; }
    }
    println!("deque len {} model {} equal {} bad {}", dq.len(), vm.len(), dq.iter().eq(vm.iter()), dbad);
    let (s1, s2) = dq.as_slices();
    println!("slices total {} front {:?} back {:?}", s1.len() + s2.len(), dq.front(), dq.back());
    let mid: Vec<u32> = dq.range(10..15).copied().collect();
    println!("range 10..15 {:?} model {:?}", mid, &vm[10..15]);
    dq.make_contiguous().sort();
    vm.sort();
    println!("sorted contiguous equal {} first5 {:?}", dq.iter().eq(vm.iter()), dq.iter().take(5).collect::<Vec<_>>());
    dq.retain(|x| x % 3 == 0);
    dq.truncate(5);
    println!("retain/truncate {:?} binary_search {:?}", dq, dq.binary_search(&dq[2]));

    // BinaryHeap
    let mut heap: BinaryHeap<(u32, u32)> = BinaryHeap::new();
    for i in 0..1000 { heap.push((r.next() % 100, i)); }
    let mut top = Vec::new();
    for _ in 0..5 {
        if let Some(mut p) = heap.peek_mut() {
            if p.0 > 95 { top.push(PeekMut::pop(p)); } else { p.0 += 50; }
        }
    }
    println!("peek_mut results {:?} len {}", top, heap.len());
    let sorted = heap.clone().into_sorted_vec();
    let ok = sorted.windows(2).all(|w| w[0] <= w[1]);
    let mut popped = Vec::new();
    while let Some(x) = heap.pop() { popped.push(x); }
    popped.reverse();
    println!("into_sorted_vec sorted {} matches pops {} max {:?}", ok, popped == sorted, sorted.last());
    let mut h2: BinaryHeap<i32> = vec![5, 1, 8, 3].into();
    h2.extend([10, -2]);
    h2.retain(|x| x % 2 == 0);
    println!("heap retain/into_vec sorted: {:?}", h2.into_sorted_vec());
}

fn model_prev_matches(e: u64, m: &mut u64) -> bool {
    // mirror entry().and_modify(+1).or_insert(1) on the model
    if *m == 0 { *m = 1; } else { *m += 1; }
    e != *m
}
