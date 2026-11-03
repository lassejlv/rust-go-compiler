use std::collections::BTreeMap;
use std::ops::Bound::{Excluded, Unbounded};

#[derive(Debug, Default)]
struct Calendar { booked: BTreeMap<u32, (u32, String)> }

impl Calendar {
    fn book(&mut self, start: u32, end: u32, who: &str) -> Result<(), String> {
        if start >= end { return Err(format!("empty interval [{}, {})", start, end)); }
        if let Some((&s, (e, w))) = self.booked.range(..end).next_back() {
            if *e > start { return Err(format!("[{}, {}) for {} overlaps [{}, {}) of {}", start, end, who, s, e, w)); }
        }
        self.booked.insert(start, (end, who.to_string()));
        Ok(())
    }
    fn free_slots(&self, lo: u32, hi: u32) -> Vec<(u32, u32)> {
        let mut out = vec![];
        let mut cur = lo;
        for (&s, &(e, _)) in self.booked.range(..hi) {
            if e <= cur { continue; }
            if s > cur { out.push((cur, s.min(hi))); }
            cur = cur.max(e);
        }
        if cur < hi { out.push((cur, hi)); }
        out
    }
    fn cancel_after(&mut self, t: u32) -> Vec<String> {
        let keys: Vec<u32> = self.booked.range((Excluded(t), Unbounded)).map(|(k, _)| *k).collect();
        keys.into_iter().filter_map(|k| self.booked.remove(&k)).map(|(_, w)| w).collect()
    }
}

fn merge(mut v: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    v.sort_unstable();
    let mut out: Vec<(i32, i32)> = vec![];
    for (s, e) in v {
        match out.last_mut() { Some(last) if s <= last.1 => last.1 = last.1.max(e), _ => out.push((s, e)) }
    }
    out
}

fn max_overlap(v: &[(i32, i32)]) -> (usize, i32) {
    let mut events: Vec<(i32, i32)> = v.iter().flat_map(|&(s, e)| [(s, 1), (e, -1)]).collect();
    events.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut cur = 0; let mut best = (0usize, 0);
    for (t, d) in events { cur += d; if cur as usize > best.0 { best = (cur as usize, t); } }
    best
}

fn skyline(buildings: &[(i32, i32, i32)]) -> Vec<(i32, i32)> {
    let mut xs: Vec<i32> = buildings.iter().flat_map(|b| [b.0, b.1]).collect();
    xs.sort(); xs.dedup();
    let mut out: Vec<(i32, i32)> = vec![];
    for &x in &xs {
        let h = buildings.iter().filter(|b| b.0 <= x && x < b.1).map(|b| b.2).max().unwrap_or(0);
        if out.last().map_or(true, |l| l.1 != h) { out.push((x, h)); }
    }
    out
}

fn main() {
    let mut cal = Calendar::default();
    let reqs = [(9, 10, "ann"), (13, 15, "bob"), (9, 11, "cat"), (10, 12, "dan"), (12, 13, "eve"), (14, 16, "fay"), (8, 9, "gus"), (16, 16, "hal"), (17, 20, "ivy")];
    for (s, e, w) in reqs { match cal.book(s, e, w) { Ok(()) => println!("booked {} [{}, {})", w, s, e), Err(m) => println!("rejected: {}", m) } }
    println!("free 7..21: {:?}", cal.free_slots(7, 21));
    println!("free 10..14: {:?}", cal.free_slots(10, 14));
    println!("cancelled after 12: {:?}", cal.cancel_after(12));
    println!("remaining {:?}", cal.booked);
    println!("{:?}", merge(vec![(1, 3), (2, 6), (8, 10), (15, 18), (17, 20), (10, 11), (-5, -1)]));
    println!("{:?}", merge(vec![]));
    println!("{:?}", max_overlap(&[(1, 5), (2, 6), (4, 7), (5, 8), (9, 10)]));
    println!("{:?}", skyline(&[(2, 9, 10), (3, 7, 15), (5, 12, 12), (15, 20, 10), (19, 24, 8)]));
}
