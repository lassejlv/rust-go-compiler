// Discrete-event simulation of a multi-server queueing network (M/M/c-ish with
// deterministic pseudo-random draws): event heap ordered by (time, seq),
// pluggable scheduling policies as trait objects, statistics collectors via
// generic traits, histograms, and a report with formatted tables.
use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap, HashMap, VecDeque};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Time(f64);

impl Eq for Time {}
impl PartialOrd for Time {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Time {
    fn cmp(&self, o: &Self) -> Ordering {
        self.0.total_cmp(&o.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EventKind {
    Arrival { customer: u32, class: Class },
    Departure { server: usize, customer: u32 },
    Breakdown { server: usize },
    Repair { server: usize },
    Snapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Class {
    Vip,
    Regular,
    Bulk,
}

#[derive(Debug, PartialEq, Eq)]
struct Event {
    at: Time,
    seq: u64,
    kind: EventKind,
}

impl Ord for Event {
    fn cmp(&self, o: &Self) -> Ordering {
        // min-heap on (time, seq)
        o.at.cmp(&self.at).then_with(|| o.seq.cmp(&self.seq))
    }
}
impl PartialOrd for Event {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

struct Rng(u64);
impl Rng {
    fn uniform(&mut self) -> f64 {
        self.0 ^= self.0 << 7;
        self.0 ^= self.0 >> 9;
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
    fn exp(&mut self, rate: f64) -> f64 {
        -(1.0 - self.uniform()).ln() / rate
    }
}

#[derive(Debug, Clone)]
struct Customer {
    id: u32,
    class: Class,
    arrived: f64,
    work: f64,
}

trait Policy {
    fn name(&self) -> String;
    fn push(&mut self, c: Customer);
    fn pop(&mut self, now: f64) -> Option<Customer>;
    fn len(&self) -> usize;
}

#[derive(Default)]
struct Fifo(VecDeque<Customer>);
impl Policy for Fifo {
    fn name(&self) -> String {
        "FIFO".into()
    }
    fn push(&mut self, c: Customer) {
        self.0.push_back(c)
    }
    fn pop(&mut self, _now: f64) -> Option<Customer> {
        self.0.pop_front()
    }
    fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Default)]
struct Priority(BTreeMap<(Class, u32), Customer>);
impl Policy for Priority {
    fn name(&self) -> String {
        "PRIORITY".to_string()
    }
    fn push(&mut self, c: Customer) {
        self.0.insert((c.class, c.id), c);
    }
    fn pop(&mut self, _now: f64) -> Option<Customer> {
        self.0.pop_first().map(|(_, c)| c)
    }
    fn len(&self) -> usize {
        self.0.len()
    }
}

struct ShortestJob {
    q: Vec<Customer>,
    aging: f64,
}
impl Policy for ShortestJob {
    fn name(&self) -> String {
        format!("SJF(aging={})", self.aging)
    }
    fn push(&mut self, c: Customer) {
        self.q.push(c)
    }
    fn pop(&mut self, now: f64) -> Option<Customer> {
        let aging = self.aging;
        let idx = self
            .q
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let ka = a.work - aging * (now - a.arrived);
                let kb = b.work - aging * (now - b.arrived);
                ka.total_cmp(&kb).then(a.id.cmp(&b.id))
            })
            .map(|(i, _)| i)?;
        Some(self.q.swap_remove(idx))
    }
    fn len(&self) -> usize {
        self.q.len()
    }
}

trait Stat {
    fn record(&mut self, x: f64);
    fn summary(&self) -> String;
}

#[derive(Default, Clone)]
struct Moments {
    n: u64,
    mean: f64,
    m2: f64,
    min: f64,
    max: f64,
}

impl Stat for Moments {
    fn record(&mut self, x: f64) {
        // Welford
        self.n += 1;
        if self.n == 1 {
            self.min = x;
            self.max = x;
        }
        self.min = self.min.min(x);
        self.max = self.max.max(x);
        let d = x - self.mean;
        self.mean += d / self.n as f64;
        self.m2 += d * (x - self.mean);
    }
    fn summary(&self) -> String {
        let sd = if self.n > 1 { (self.m2 / (self.n - 1) as f64).sqrt() } else { 0.0 };
        format!("n={:<5} mean={:>8.3} sd={:>8.3} min={:>7.3} max={:>8.3}", self.n, self.mean, sd, self.min, self.max)
    }
}

struct Histogram<const B: usize> {
    width: f64,
    bins: [u32; B],
    overflow: u32,
}

impl<const B: usize> Histogram<B> {
    fn new(width: f64) -> Self {
        Histogram { width, bins: [0; B], overflow: 0 }
    }
    fn render(&self) -> String {
        let max = *self.bins.iter().max().unwrap_or(&1).max(&1);
        let mut s = String::new();
        for (i, &b) in self.bins.iter().enumerate() {
            let bar = "#".repeat((b as usize * 30 + max as usize - 1) / max as usize);
            s += &format!("  [{:>5.1},{:>5.1}) {:>4} {}\n", i as f64 * self.width, (i + 1) as f64 * self.width, b, bar);
        }
        s + &format!("  overflow {}", self.overflow)
    }
}

impl<const B: usize> Stat for Histogram<B> {
    fn record(&mut self, x: f64) {
        let i = (x / self.width) as usize;
        match self.bins.get_mut(i) {
            Some(b) => *b += 1,
            None => self.overflow += 1,
        }
    }
    fn summary(&self) -> String {
        format!("{} bins, overflow {}", B, self.overflow)
    }
}

struct Server {
    busy: Option<(Customer, f64)>,
    up: bool,
    busy_time: f64,
    served: u32,
    speed: f64,
}

struct Sim {
    now: f64,
    seq: u64,
    heap: BinaryHeap<Event>,
    servers: Vec<Server>,
    queue: Box<dyn Policy>,
    rng: Rng,
    waits: HashMap<Class, Moments>,
    sojourn: Histogram<12>,
    all_waits: Moments,
    qlen_samples: Vec<usize>,
    next_id: u32,
    log: Vec<String>,
    interrupted: u32,
}

impl Sim {
    fn new(policy: Box<dyn Policy>, nservers: usize, seed: u64) -> Sim {
        let servers = (0..nservers).map(|i| Server { busy: None, up: true, busy_time: 0.0, served: 0, speed: 1.0 + i as f64 * 0.25 }).collect();
        Sim {
            now: 0.0,
            seq: 0,
            heap: BinaryHeap::new(),
            servers,
            queue: policy,
            rng: Rng(seed),
            waits: HashMap::new(),
            sojourn: Histogram::new(2.0),
            all_waits: Moments::default(),
            qlen_samples: vec![],
            next_id: 0,
            log: vec![],
            interrupted: 0,
        }
    }
    fn schedule(&mut self, dt: f64, kind: EventKind) {
        self.seq += 1;
        self.heap.push(Event { at: Time(self.now + dt), seq: self.seq, kind });
    }
    fn new_arrival(&mut self) {
        let r = self.rng.uniform();
        let class = if r < 0.1 { Class::Vip } else if r < 0.85 { Class::Regular } else { Class::Bulk };
        let dt = self.rng.exp(1.6);
        self.next_id += 1;
        let id = self.next_id;
        self.schedule(dt, EventKind::Arrival { customer: id, class });
    }
    fn try_start(&mut self) {
        for i in 0..self.servers.len() {
            if self.servers[i].up && self.servers[i].busy.is_none() {
                let Some(c) = self.queue.pop(self.now) else { return };
                let wait = self.now - c.arrived;
                self.waits.entry(c.class).or_default().record(wait);
                self.all_waits.record(wait);
                let dur = c.work / self.servers[i].speed;
                let id = c.id;
                self.servers[i].busy = Some((c, self.now));
                self.schedule(dur, EventKind::Departure { server: i, customer: id });
            }
        }
    }
    fn run(&mut self, until: f64) {
        self.new_arrival();
        self.schedule(5.0, EventKind::Snapshot);
        self.schedule(30.0, EventKind::Breakdown { server: 0 });
        while let Some(ev) = self.heap.pop() {
            if ev.at.0 > until {
                break;
            }
            self.now = ev.at.0;
            match ev.kind {
                EventKind::Arrival { customer, class } => {
                    let work = match class {
                        Class::Vip => self.rng.exp(1.0),
                        Class::Regular => self.rng.exp(0.8),
                        Class::Bulk => 2.0 + self.rng.exp(0.3),
                    };
                    self.queue.push(Customer { id: customer, class, arrived: self.now, work });
                    self.new_arrival();
                    self.try_start();
                }
                EventKind::Departure { server, customer } => {
                    let s = &mut self.servers[server];
                    match s.busy.take() {
                        Some((c, started)) if c.id == customer => {
                            s.busy_time += self.now - started;
                            s.served += 1;
                            self.sojourn.record(self.now - c.arrived);
                        }
                        other => {
                            // stale departure after a breakdown re-queued the customer
                            s.busy = other;
                        }
                    }
                    self.try_start();
                }
                EventKind::Breakdown { server } => {
                    let s = &mut self.servers[server];
                    s.up = false;
                    if let Some((c, started)) = s.busy.take() {
                        s.busy_time += self.now - started;
                        self.interrupted += 1;
                        self.log.push(format!("t={:.2} server {} broke while serving #{}", self.now, server, c.id));
                        self.queue.push(c);
                    } else {
                        self.log.push(format!("t={:.2} server {} broke (idle)", self.now, server));
                    }
                    let rep = 3.0 + self.rng.uniform() * 4.0;
                    self.schedule(rep, EventKind::Repair { server });
                    self.try_start();
                }
                EventKind::Repair { server } => {
                    self.servers[server].up = true;
                    self.log.push(format!("t={:.2} server {} repaired", self.now, server));
                    let next = 25.0 + self.rng.uniform() * 20.0;
                    self.schedule(next, EventKind::Breakdown { server: (server + 1) % self.servers.len() });
                    self.try_start();
                }
                EventKind::Snapshot => {
                    self.qlen_samples.push(self.queue.len());
                    self.schedule(5.0, EventKind::Snapshot);
                }
            }
        }
    }
    fn report(&self) -> String {
        let mut out = format!("policy {} | t={:.1} customers={} interrupted={}\n", self.queue.name(), self.now, self.next_id, self.interrupted);
        let mut classes: Vec<_> = self.waits.iter().collect();
        classes.sort_by_key(|(c, _)| **c);
        for (c, m) in classes {
            out += &format!("  wait {:<8} {}\n", format!("{:?}", c), m.summary());
        }
        out += &format!("  wait {:<8} {}\n", "ALL", self.all_waits.summary());
        for (i, s) in self.servers.iter().enumerate() {
            out += &format!("  server {} speed {:.2} served {:>3} util {:>5.1}%\n", i, s.speed, s.served, 100.0 * s.busy_time / self.now);
        }
        let avg_q = self.qlen_samples.iter().sum::<usize>() as f64 / self.qlen_samples.len().max(1) as f64;
        out += &format!("  avg queue length {:.3} (max {:?})\n", avg_q, self.qlen_samples.iter().max());
        out += &format!("  sojourn {}\n{}", self.sojourn.summary(), self.sojourn.render());
        out
    }
}

impl fmt::Display for Sim {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.report())
    }
}

fn main() {
    let policies: Vec<Box<dyn Fn() -> Box<dyn Policy>>> = vec![
        Box::new(|| Box::new(Fifo::default())),
        Box::new(|| Box::new(Priority::default())),
        Box::new(|| Box::new(ShortestJob { q: vec![], aging: 0.0 })),
        Box::new(|| Box::new(ShortestJob { q: vec![], aging: 0.5 })),
    ];
    let mut summary = vec![];
    for (i, mk) in policies.iter().enumerate() {
        let mut sim = Sim::new(mk(), 3, 0x2545F4914F6CDD1D);
        sim.run(200.0);
        print!("{}", sim);
        if i == 0 {
            for l in sim.log.iter().take(6) {
                println!("  log: {}", l);
            }
        }
        summary.push((sim.queue.name(), sim.all_waits.mean, sim.waits.get(&Class::Vip).map(|m| m.mean)));
    }
    summary.sort_by(|a, b| a.1.total_cmp(&b.1));
    for (name, w, vip) in &summary {
        println!("{:<16} mean wait {:>7.3} vip {:>7.3}", name, w, vip.unwrap_or(f64::NAN));
    }
    let mut h: Histogram<4> = Histogram::new(0.25);
    for x in [0.1, 0.3, 0.3, 0.9, 1.5, -0.0] {
        h.record(x);
    }
    println!("{}\n{}", h.summary(), h.render());
    let mut ev = BinaryHeap::new();
    for (i, t) in [3.0, 1.0, 2.0, 1.0, f64::INFINITY, 0.5].iter().enumerate() {
        ev.push(Event { at: Time(*t), seq: i as u64, kind: EventKind::Snapshot });
    }
    let order: Vec<(f64, u64)> = std::iter::from_fn(|| ev.pop()).map(|e| (e.at.0, e.seq)).collect();
    println!("{:?}", order);
}
