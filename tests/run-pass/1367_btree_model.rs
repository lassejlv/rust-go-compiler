// Model-based randomized testing of BTreeMap/BTreeSet against a sorted Vec
// reference: inserts, removes, entry API, ranges (both ends, all bound kinds),
// range_mut, pop_first/last, split_off, append, retain, extract_if, iterator
// double-endedness and ExactSizeIterator lengths, at sizes large enough to
// span many internal nodes; plus an order-book simulation and a Dijkstra
// priority queue built on BTreeSet.
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound::{self, Excluded, Included, Unbounded};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

#[derive(Default)]
struct Model {
    v: Vec<(u32, u64)>,
}

impl Model {
    fn pos(&self, k: u32) -> Result<usize, usize> {
        self.v.binary_search_by_key(&k, |e| e.0)
    }
    fn insert(&mut self, k: u32, val: u64) -> Option<u64> {
        match self.pos(k) {
            Ok(i) => Some(std::mem::replace(&mut self.v[i].1, val)),
            Err(i) => {
                self.v.insert(i, (k, val));
                None
            }
        }
    }
    fn remove(&mut self, k: u32) -> Option<u64> {
        self.pos(k).ok().map(|i| self.v.remove(i).1)
    }
    fn range(&self, lo: Bound<u32>, hi: Bound<u32>) -> Vec<(u32, u64)> {
        self.v
            .iter()
            .filter(|(k, _)| {
                (match lo {
                    Included(l) => *k >= l,
                    Excluded(l) => *k > l,
                    Unbounded => true,
                }) && (match hi {
                    Included(h) => *k <= h,
                    Excluded(h) => *k < h,
                    Unbounded => true,
                })
            })
            .cloned()
            .collect()
    }
}

fn bound(r: &mut Rng, keyspace: u64) -> Bound<u32> {
    match r.below(3) {
        0 => Included(r.below(keyspace) as u32),
        1 => Excluded(r.below(keyspace) as u32),
        _ => Unbounded,
    }
}

fn valid_range(lo: Bound<u32>, hi: Bound<u32>) -> bool {
    match (lo, hi) {
        (Included(a) | Excluded(a), Included(b) | Excluded(b)) if a > b => false,
        (Excluded(a), Excluded(b)) if a == b => false,
        _ => true,
    }
}

fn check(m: &BTreeMap<u32, u64>, model: &Model, ctx: &str) {
    assert_eq!(m.len(), model.v.len(), "{}: len", ctx);
    let got: Vec<(u32, u64)> = m.iter().map(|(k, v)| (*k, *v)).collect();
    assert!(got == model.v, "{}: contents differ", ctx);
    let rev: Vec<(u32, u64)> = m.iter().rev().map(|(k, v)| (*k, *v)).collect();
    assert!(rev.iter().rev().eq(model.v.iter()), "{}: reverse differs", ctx);
    assert_eq!(m.first_key_value().map(|(k, v)| (*k, *v)), model.v.first().copied());
    assert_eq!(m.last_key_value().map(|(k, v)| (*k, *v)), model.v.last().copied());
}

fn main() {
    let mut r = Rng(12345);
    let mut total_ops = 0;
    for (round, &(ops, keyspace)) in [(3000u64, 50u64), (20000, 5000), (20000, 400_000), (6000, 20)].iter().enumerate() {
        let mut m: BTreeMap<u32, u64> = BTreeMap::new();
        let mut model = Model::default();
        let mut range_checks = 0;
        for op in 0..ops {
            let k = r.below(keyspace) as u32;
            match r.below(16) {
                0..=5 => assert_eq!(m.insert(k, op), model.insert(k, op)),
                6..=7 => assert_eq!(m.remove(&k), model.remove(k)),
                8 => {
                    let e = m.entry(k).or_insert(0);
                    *e += 7;
                    let cur = model.pos(k).map(|i| model.v[i].1).unwrap_or(0) + 7;
                    model.insert(k, cur);
                }
                9 => {
                    let got = m.pop_first();
                    let want = if model.v.is_empty() { None } else { Some(model.v.remove(0)) };
                    assert_eq!(got, want);
                }
                10 => {
                    let got = m.pop_last();
                    assert_eq!(got, model.v.pop());
                }
                11 | 12 => {
                    let (lo, hi) = (bound(&mut r, keyspace), bound(&mut r, keyspace));
                    if valid_range(lo, hi) {
                        let want = model.range(lo, hi);
                        let it = m.range((lo, hi));
                        assert_eq!(it.clone().count(), want.len(), "range len {:?} {:?}", lo, hi);
                        let got: Vec<(u32, u64)> = it.map(|(k, v)| (*k, *v)).collect();
                        assert!(got == want, "range {:?} {:?}", lo, hi);
                        // alternate from both ends
                        let mut it = m.range((lo, hi));
                        let mut front = vec![];
                        let mut back = vec![];
                        loop {
                            match it.next() {
                                Some((k, _)) => front.push(*k),
                                None => break,
                            }
                            match it.next_back() {
                                Some((k, _)) => back.push(*k),
                                None => break,
                            }
                        }
                        back.reverse();
                        front.extend(back);
                        assert!(front.iter().eq(want.iter().map(|e| &e.0)), "zigzag {:?} {:?}", lo, hi);
                        range_checks += 1;
                    }
                }
                13 => {
                    if let Some(mut e) = m.first_entry() {
                        *e.get_mut() += 1;
                        model.v[0].1 += 1;
                    }
                    if r.below(4) == 0 {
                        if let Some(e) = m.last_entry() {
                            let (k, v) = e.remove_entry();
                            assert_eq!(Some((k, v)), model.v.pop());
                        }
                    }
                }
                14 => {
                    let lo = r.below(keyspace) as u32;
                    let hi = lo.saturating_add(r.below(keyspace / 4 + 1) as u32);
                    for (k, v) in m.range_mut(lo..=hi) {
                        *v = v.wrapping_mul(3) ^ *k as u64;
                    }
                    for e in model.v.iter_mut().filter(|e| e.0 >= lo && e.0 <= hi) {
                        e.1 = e.1.wrapping_mul(3) ^ e.0 as u64;
                    }
                }
                _ => {
                    assert_eq!(m.get(&k), model.pos(k).ok().map(|i| &model.v[i].1));
                    assert_eq!(m.contains_key(&k), model.pos(k).is_ok());
                }
            }
            if op % 997 == 0 {
                check(&m, &model, "periodic");
            }
        }
        check(&m, &model, "end of round");
        // bulk operations
        let pivot = r.below(keyspace) as u32;
        let mut hi = m.split_off(&pivot);
        let split_at = model.pos(pivot).unwrap_or_else(|i| i);
        let model_hi = model.v.split_off(split_at);
        assert!(hi.iter().map(|(k, v)| (*k, *v)).eq(model_hi.iter().copied()));
        check(&m, &model, "after split_off");
        m.append(&mut hi);
        assert!(hi.is_empty());
        model.v.extend(model_hi);
        check(&m, &model, "after append");
        m.retain(|k, v| (*k as u64 + *v) % 3 != 0);
        model.v.retain(|(k, v)| (*k as u64 + *v) % 3 != 0);
        check(&m, &model, "after retain");
        let taken: Vec<(u32, u64)> = m.extract_if(.., |k, _| k % 5 == 0).collect();
        let (gone, kept): (Vec<_>, Vec<_>) = model.v.iter().partition(|(k, _)| k % 5 == 0);
        model.v = kept;
        assert!(taken == gone);
        check(&m, &model, "after extract_if");
        for (_, v) in m.iter_mut().rev().step_by(2) {
            *v += 1;
        }
        let n = model.v.len();
        for (i, e) in model.v.iter_mut().enumerate() {
            if (n - 1 - i) % 2 == 0 {
                e.1 += 1;
            }
        }
        check(&m, &model, "after iter_mut");
        let sum: u64 = m.values().fold(0u64, |a, v| a.wrapping_add(*v));
        let keys_rev_first = m.keys().rev().next().copied();
        let into: Vec<(u32, u64)> = m.clone().into_iter().rev().take(3).collect();
        let cloned = m.clone();
        assert!(cloned == m && !(cloned < m));
        total_ops += ops;
        println!(
            "round {}: len {} range checks {} sum {} last key {:?} tail {:?} into_keys {}",
            round,
            m.len(),
            range_checks,
            sum,
            keys_rev_first,
            into,
            m.clone().into_keys().filter(|k| k % 2 == 0).count()
        );
    }
    println!("total ops {}", total_ops);

    // BTreeSet as a priority queue (Dijkstra on a grid with weights)
    let (w, h) = (60usize, 40usize);
    let cost: Vec<u64> = (0..w * h).map(|_| 1 + r.below(9)).collect();
    let mut dist = vec![u64::MAX; w * h];
    let mut pq = BTreeSet::new();
    dist[0] = 0;
    pq.insert((0u64, 0usize));
    let mut pops = 0;
    while let Some((d, u)) = pq.pop_first() {
        pops += 1;
        if d > dist[u] {
            continue;
        }
        let (x, y) = (u % w, u / w);
        let mut nbrs = vec![];
        if x > 0 {
            nbrs.push(u - 1);
        }
        if x + 1 < w {
            nbrs.push(u + 1);
        }
        if y > 0 {
            nbrs.push(u - w);
        }
        if y + 1 < h {
            nbrs.push(u + w);
        }
        for v in nbrs {
            let nd = d + cost[v];
            if nd < dist[v] {
                if dist[v] != u64::MAX {
                    pq.remove(&(dist[v], v));
                }
                dist[v] = nd;
                pq.insert((nd, v));
            }
        }
    }
    println!("dijkstra: corner dist {} pops {} max {}", dist[w * h - 1], pops, dist.iter().max().unwrap());

    // order book: price levels in a BTreeMap, best bid = last, best ask = first
    let mut bids: BTreeMap<u32, u32> = BTreeMap::new();
    let mut asks: BTreeMap<u32, u32> = BTreeMap::new();
    let mut traded = 0u64;
    let mut trades = 0;
    for _ in 0..20000 {
        let buy = r.below(2) == 0;
        let price = 1000 + r.below(200) as u32 - if buy { 20 } else { 0 };
        let mut qty = 1 + r.below(50) as u32;
        if buy {
            while qty > 0 {
                let Some(mut best) = asks.first_entry() else { break };
                if *best.key() > price {
                    break;
                }
                let take = qty.min(*best.get());
                qty -= take;
                *best.get_mut() -= take;
                traded += take as u64 * *best.key() as u64;
                trades += 1;
                if *best.get() == 0 {
                    best.remove();
                }
            }
            if qty > 0 {
                *bids.entry(price).or_default() += qty;
            }
        } else {
            while qty > 0 {
                let Some(mut best) = bids.last_entry() else { break };
                if *best.key() < price {
                    break;
                }
                let take = qty.min(*best.get());
                qty -= take;
                *best.get_mut() -= take;
                traded += take as u64 * *best.key() as u64;
                trades += 1;
                if *best.get() == 0 {
                    best.remove();
                }
            }
            if qty > 0 {
                *asks.entry(price).or_default() += qty;
            }
        }
    }
    let spread = match (bids.last_key_value(), asks.first_key_value()) {
        (Some((b, _)), Some((a, _))) => Some(a - b),
        _ => None,
    };
    println!("orderbook: trades {} notional {} bid levels {} ask levels {} spread {:?}", trades, traded, bids.len(), asks.len(), spread);
    println!("top bids {:?}", bids.iter().rev().take(3).collect::<Vec<_>>());
    println!("asks 1050..1060 {:?}", asks.range(1050..1060).collect::<Vec<_>>());

    // sets
    let a: BTreeSet<u32> = (0..2000).map(|i| i * 3 % 1999).collect();
    let b: BTreeSet<u32> = (0..1500).map(|i| i * 7 % 1777).collect();
    println!(
        "sets: |a|={} |b|={} |a&b|={} |a|b|={} |a-b|={} |a^b|={} subset {} first {:?} last {:?}",
        a.len(),
        b.len(),
        a.intersection(&b).count(),
        a.union(&b).count(),
        a.difference(&b).count(),
        a.symmetric_difference(&b).count(),
        b.is_subset(&a),
        a.first(),
        b.last()
    );
    let mut s: BTreeSet<String> = ["pear", "apple", "fig", "kiwi", "banana"].iter().map(|s| s.to_string()).collect();
    let tail = s.split_off("fig");
    println!("{:?} {:?} {:?}", s, tail, tail.range::<str, _>((Excluded("fig"), Unbounded)).collect::<Vec<_>>());
}
