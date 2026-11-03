use std::collections::{BTreeSet, VecDeque};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Dir { Up, Down, Idle }

#[derive(Debug, Clone)]
struct Request { at: u32, from: u8, to: u8, id: usize }

trait Strategy: fmt::Debug {
    fn name(&self) -> &str;
    fn pick(&self, car: &Car, pending: &[Request]) -> Option<usize>;
}

#[derive(Debug)] struct Fifo;
#[derive(Debug)] struct Nearest { penalty: u32 }

#[derive(Debug, Clone)]
struct Car { floor: u8, dir: Dir, riders: Vec<Request>, stops: BTreeSet<u8>, moved: u32 }

impl Strategy for Fifo {
    fn name(&self) -> &str { "fifo" }
    fn pick(&self, _car: &Car, pending: &[Request]) -> Option<usize> { if pending.is_empty() { None } else { Some(0) } }
}
impl Strategy for Nearest {
    fn name(&self) -> &str { "nearest" }
    fn pick(&self, car: &Car, pending: &[Request]) -> Option<usize> {
        pending.iter().enumerate().min_by_key(|(_, r)| {
            let d = (r.from as i32 - car.floor as i32).unsigned_abs();
            let wrong_way = match car.dir { Dir::Up => r.from < car.floor, Dir::Down => r.from > car.floor, Dir::Idle => false };
            (d + if wrong_way { self.penalty } else { 0 }, r.id)
        }).map(|(i, _)| i)
    }
}

struct Stats { waits: Vec<u32>, rides: Vec<u32> }
impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let avg = |v: &[u32]| if v.is_empty() { 0.0 } else { v.iter().sum::<u32>() as f64 / v.len() as f64 };
        write!(f, "served={} avg_wait={:.2} max_wait={} avg_ride={:.2}", self.rides.len(), avg(&self.waits), self.waits.iter().max().unwrap_or(&0), avg(&self.rides))
    }
}

fn simulate(strategy: &dyn Strategy, reqs: &[Request], verbose: bool) -> (Stats, u32) {
    let mut incoming: VecDeque<Request> = reqs.iter().cloned().collect();
    let mut pending: Vec<Request> = vec![];
    let mut car = Car { floor: 0, dir: Dir::Idle, riders: vec![], stops: BTreeSet::new(), moved: 0 };
    let mut waiting_target: Option<usize> = None;
    let mut stats = Stats { waits: vec![], rides: vec![] };
    let mut boarded_at: Vec<(usize, u32)> = vec![];
    let mut t = 0;
    while t < 500 {
        while incoming.front().map_or(false, |r| r.at <= t) { pending.push(incoming.pop_front().unwrap()); }
        // unload
        let floor = car.floor;
        let (out, stay): (Vec<Request>, Vec<Request>) = car.riders.drain(..).partition(|r| r.to == floor);
        car.riders = stay;
        for r in out {
            let b = boarded_at.iter().position(|(id, _)| *id == r.id).unwrap();
            let (_, bt) = boarded_at.swap_remove(b);
            stats.rides.push(t - bt);
            if verbose { println!("  t={:>3} drop #{} at {}", t, r.id, floor); }
        }
        car.stops.remove(&floor);
        // load everyone waiting here
        let mut i = 0;
        while i < pending.len() {
            if pending[i].from == floor {
                let r = pending.remove(i);
                stats.waits.push(t - r.at);
                boarded_at.push((r.id, t));
                car.stops.insert(r.to);
                if verbose { println!("  t={:>3} pick #{} at {} -> {}", t, r.id, floor, r.to); }
                if waiting_target == Some(r.id) { waiting_target = None; }
                car.riders.push(r);
            } else { i += 1; }
        }
        if waiting_target.is_none() && car.riders.is_empty() {
            if let Some(idx) = strategy.pick(&car, &pending) { waiting_target = Some(pending[idx].id); car.stops.insert(pending[idx].from); }
        }
        if let Some(id) = waiting_target { if let Some(r) = pending.iter().find(|r| r.id == id) { car.stops.insert(r.from); } }
        let target = match car.dir {
            Dir::Up | Dir::Idle => car.stops.range(car.floor..).next().or_else(|| car.stops.iter().next_back()).copied(),
            Dir::Down => car.stops.range(..=car.floor).next_back().or_else(|| car.stops.iter().next()).copied(),
        };
        match target {
            Some(f) if f > car.floor => { car.floor += 1; car.dir = Dir::Up; car.moved += 1; }
            Some(f) if f < car.floor => { car.floor -= 1; car.dir = Dir::Down; car.moved += 1; }
            Some(_) => {}
            None => { car.dir = Dir::Idle; if incoming.is_empty() && pending.is_empty() { break; } }
        }
        t += 1;
    }
    (stats, car.moved)
}

fn main() {
    let mut seed = 12345u32;
    let mut rnd = |m: u32| { seed ^= seed << 13; seed ^= seed >> 17; seed ^= seed << 5; seed % m };
    let mut reqs = vec![];
    let mut at = 0;
    for id in 0..18 {
        at += rnd(6);
        let from = rnd(10) as u8;
        let mut to = rnd(10) as u8;
        if to == from { to = (to + 3) % 10; }
        reqs.push(Request { at, from, to, id });
    }
    for r in &reqs[..5] { println!("{:?}", r); }
    let strategies: Vec<Box<dyn Strategy>> = vec![Box::new(Fifo), Box::new(Nearest { penalty: 0 }), Box::new(Nearest { penalty: 5 })];
    for (i, s) in strategies.iter().enumerate() {
        let (stats, moved) = simulate(s.as_ref(), &reqs, i == 2);
        println!("{:<8} {:?}: {} moved={}", s.name(), s, stats, moved);
    }
}
