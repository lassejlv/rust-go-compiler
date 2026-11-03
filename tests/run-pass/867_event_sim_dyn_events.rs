use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};
use std::fmt::Debug;

#[derive(Debug, Default)]
struct World { queue_len: usize, served: u32, balked: u32, busy_until: u64, waits: Vec<u64>, arrivals: BTreeMap<u32, u64>, rng: u64, log: Vec<String> }

impl World {
    fn rand(&mut self, m: u64) -> u64 { self.rng ^= self.rng << 13; self.rng ^= self.rng >> 7; self.rng ^= self.rng << 17; self.rng % m }
}

trait Event: Debug { fn fire(&self, now: u64, w: &mut World) -> Vec<(u64, Box<dyn Event>)>; fn kind(&self) -> &'static str; }

#[derive(Debug)] struct Arrival { id: u32 }
#[derive(Debug)] struct StartService { id: u32 }
#[derive(Debug)] struct Departure { id: u32 }
#[derive(Debug)] struct Report;

impl Event for Arrival {
    fn kind(&self) -> &'static str { "arrive" }
    fn fire(&self, now: u64, w: &mut World) -> Vec<(u64, Box<dyn Event>)> {
        let mut next: Vec<(u64, Box<dyn Event>)> = vec![];
        if self.id < 25 { let gap = 1 + w.rand(6); next.push((now + gap, Box::new(Arrival { id: self.id + 1 }))); }
        if w.queue_len >= 4 { w.balked += 1; w.log.push(format!("t={} #{} balked", now, self.id)); return next; }
        w.queue_len += 1;
        w.arrivals.insert(self.id, now);
        let start = now.max(w.busy_until);
        let dur = 2 + w.rand(5);
        w.busy_until = start + dur;
        next.push((start, Box::new(StartService { id: self.id })));
        next.push((start + dur, Box::new(Departure { id: self.id })));
        next
    }
}
impl Event for StartService {
    fn kind(&self) -> &'static str { "start" }
    fn fire(&self, now: u64, w: &mut World) -> Vec<(u64, Box<dyn Event>)> { let a = w.arrivals[&self.id]; w.waits.push(now - a); vec![] }
}
impl Event for Departure {
    fn kind(&self) -> &'static str { "depart" }
    fn fire(&self, _now: u64, w: &mut World) -> Vec<(u64, Box<dyn Event>)> { w.queue_len -= 1; w.served += 1; w.arrivals.remove(&self.id); vec![] }
}
impl Event for Report {
    fn kind(&self) -> &'static str { "report" }
    fn fire(&self, now: u64, w: &mut World) -> Vec<(u64, Box<dyn Event>)> {
        w.log.push(format!("t={} report: queue={} served={} balked={}", now, w.queue_len, w.served, w.balked));
        if now < 60 { vec![(now + 20, Box::new(Report))] } else { vec![] }
    }
}

fn main() {
    let mut w = World { rng: 0x9E3779B97F4A7C15, ..Default::default() };
    let mut heap: BinaryHeap<Reverse<(u64, u64)>> = BinaryHeap::new();
    let mut events: BTreeMap<u64, Box<dyn Event>> = BTreeMap::new();
    let mut seq = 0u64;
    let mut schedule = |heap: &mut BinaryHeap<Reverse<(u64, u64)>>, events: &mut BTreeMap<u64, Box<dyn Event>>, t: u64, e: Box<dyn Event>| { heap.push(Reverse((t, seq))); events.insert(seq, e); seq += 1; };
    schedule(&mut heap, &mut events, 0, Box::new(Arrival { id: 0 }));
    schedule(&mut heap, &mut events, 20, Box::new(Report));
    let mut counts: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut last_t = 0;
    while let Some(Reverse((t, id))) = heap.pop() {
        let e = events.remove(&id).expect("event");
        *counts.entry(e.kind()).or_default() += 1;
        last_t = t;
        for (nt, ne) in e.fire(t, &mut w) { schedule(&mut heap, &mut events, nt, ne); }
    }
    for l in &w.log { println!("{}", l); }
    let avg = w.waits.iter().sum::<u64>() as f64 / w.waits.len() as f64;
    println!("end t={} served={} balked={} avg_wait={:.3} max_wait={:?}", last_t, w.served, w.balked, avg, w.waits.iter().max());
    println!("{:?}", counts);
    println!("pending events {} queue {}", events.len(), w.queue_len);
}
