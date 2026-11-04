// Discrete-event simulation of a bank of service counters and an elevator
// using BinaryHeap as the event queue, with trait-based event handlers,
// deterministic randomness, statistics, and histograms.
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::fmt;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
struct Time(f64);

impl Eq for Time {}
impl Ord for Time {
    fn cmp(&self, o: &Self) -> Ordering { self.0.total_cmp(&o.0) }
}
impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mins = (self.0 / 60.0).floor() as u64;
        let secs = self.0 - mins as f64 * 60.0;
        write!(f, "{:02}:{:05.2}", mins, secs)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Ev {
    Arrive { customer: u32 },
    ServiceDone { counter: usize, customer: u32 },
    Break { counter: usize },
    Resume { counter: usize },
    Close,
}

#[derive(PartialEq, Eq)]
struct Scheduled { at: Time, seq: u64, ev: Ev }

impl Ord for Scheduled {
    fn cmp(&self, o: &Self) -> Ordering {
        o.at.cmp(&self.at).then_with(|| o.seq.cmp(&self.seq))
    }
}
impl PartialOrd for Scheduled {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) }
}

struct Rng(u64);
impl Rng {
    fn uniform(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn exp(&mut self, mean: f64) -> f64 { -mean * (1.0 - self.uniform()).ln() }
}

#[derive(Default, Debug, Clone)]
struct Counter { busy_with: Option<u32>, on_break: bool, served: u32, busy_time: f64, last_start: f64 }

struct Customer { arrived: f64, started: Option<f64>, done: Option<f64>, vip: bool, balked: bool }

struct Sim {
    now: Time,
    queue: BinaryHeap<Scheduled>,
    seq: u64,
    rng: Rng,
    counters: Vec<Counter>,
    line: VecDeque<u32>,
    customers: HashMap<u32, Customer>,
    next_customer: u32,
    open: bool,
    log: Vec<String>,
    max_line: usize,
    area_line: f64,
    last_change: f64,
}

impl Sim {
    fn schedule(&mut self, delay: f64, ev: Ev) {
        self.seq += 1;
        self.queue.push(Scheduled { at: Time(self.now.0 + delay), seq: self.seq, ev });
    }

    fn note(&mut self, s: String) {
        if self.log.len() < 25 {
            self.log.push(format!("[{}] {}", self.now, s));
        }
    }

    fn track_line(&mut self) {
        self.area_line += self.line.len() as f64 * (self.now.0 - self.last_change);
        self.last_change = self.now.0;
    }

    fn try_start(&mut self) {
        for c in 0..self.counters.len() {
            if self.counters[c].busy_with.is_some() || self.counters[c].on_break {
                continue;
            }
            // VIPs jump the line
            let pos = self.line.iter().position(|id| self.customers[id].vip).unwrap_or(0);
            let Some(id) = self.line.remove(pos) else { return };
            self.track_line();
            let now = self.now.0;
            let cust = self.customers.get_mut(&id).unwrap();
            cust.started = Some(now);
            let vip = cust.vip;
            let ctr = &mut self.counters[c];
            ctr.busy_with = Some(id);
            ctr.last_start = now;
            let dur = self.rng.exp(if vip { 90.0 } else { 150.0 }) + 20.0;
            self.schedule(dur, Ev::ServiceDone { counter: c, customer: id });
            self.note(format!("counter {} starts #{}{} ({:.1}s)", c, id, if vip { " VIP" } else { "" }, dur));
        }
    }

    fn handle(&mut self, ev: Ev) {
        match ev {
            Ev::Arrive { customer } => {
                let vip = self.rng.uniform() < 0.1;
                let balk = self.line.len() >= 8 && self.rng.uniform() < 0.5;
                self.customers.insert(customer, Customer { arrived: self.now.0, started: None, done: None, vip, balked: balk });
                if balk {
                    self.note(format!("#{} balks at line of {}", customer, self.line.len()));
                } else {
                    self.track_line();
                    self.line.push_back(customer);
                    self.max_line = self.max_line.max(self.line.len());
                }
                if self.open {
                    self.next_customer += 1;
                    let gap = self.rng.exp(40.0);
                    self.schedule(gap, Ev::Arrive { customer: self.next_customer });
                }
                self.try_start();
            }
            Ev::ServiceDone { counter, customer } => {
                let now = self.now.0;
                let c = &mut self.counters[counter];
                c.busy_with = None;
                c.served += 1;
                c.busy_time += now - c.last_start;
                self.customers.get_mut(&customer).unwrap().done = Some(now);
                self.try_start();
            }
            Ev::Break { counter } => {
                self.counters[counter].on_break = true;
                self.note(format!("counter {} goes on break", counter));
                self.schedule(600.0, Ev::Resume { counter });
            }
            Ev::Resume { counter } => {
                self.counters[counter].on_break = false;
                self.note(format!("counter {} back from break (line {})", counter, self.line.len()));
                self.try_start();
            }
            Ev::Close => {
                self.open = false;
                self.note(format!("doors close; {} waiting", self.line.len()));
            }
        }
    }

    fn run(&mut self, limit: usize) -> usize {
        let mut n = 0;
        while let Some(Scheduled { at, ev, .. }) = self.queue.pop() {
            assert!(at >= self.now, "time went backwards");
            self.now = at;
            self.handle(ev);
            n += 1;
            if n >= limit {
                break;
            }
        }
        n
    }
}

fn histogram(values: &[f64], bucket: f64, width: usize) -> Vec<String> {
    let mut counts: Vec<usize> = Vec::new();
    for &v in values {
        let b = (v / bucket) as usize;
        if b >= counts.len() {
            counts.resize(b + 1, 0);
        }
        counts[b] += 1;
    }
    let max = counts.iter().copied().max().unwrap_or(1).max(1);
    counts.iter().enumerate().map(|(i, &c)| {
        format!("{:>5.0}-{:<5.0}|{:<w$}| {}", i as f64 * bucket, (i + 1) as f64 * bucket, "*".repeat(c * width / max), c, w = width)
    }).collect()
}

// ---- elevator (second model, with a trait for policies) ----
trait Policy {
    fn name(&self) -> &'static str;
    fn next_floor(&mut self, at: i32, calls: &[(i32, u32)]) -> Option<i32>;
}

struct Nearest;
struct Sweep { up: bool }

impl Policy for Nearest {
    fn name(&self) -> &'static str { "nearest" }
    fn next_floor(&mut self, at: i32, calls: &[(i32, u32)]) -> Option<i32> {
        calls.iter().min_by_key(|(f, id)| ((f - at).abs(), *id)).map(|c| c.0)
    }
}
impl Policy for Sweep {
    fn name(&self) -> &'static str { "sweep" }
    fn next_floor(&mut self, at: i32, calls: &[(i32, u32)]) -> Option<i32> {
        if calls.is_empty() { return None; }
        for _ in 0..2 {
            let cand = if self.up {
                calls.iter().filter(|c| c.0 >= at).map(|c| c.0).min()
            } else {
                calls.iter().filter(|c| c.0 <= at).map(|c| c.0).max()
            };
            if cand.is_some() { return cand; }
            self.up = !self.up;
        }
        None
    }
}

fn elevator(policy: &mut dyn Policy, requests: &[(f64, i32)]) -> (f64, f64, i32) {
    let mut time = 0.0;
    let mut at = 0;
    let mut pending: Vec<(i32, u32)> = Vec::new();
    let mut waits = Vec::new();
    let mut travelled = 0;
    let mut idx = 0;
    let mut arrival: HashMap<u32, f64> = HashMap::new();
    while idx < requests.len() || !pending.is_empty() {
        while idx < requests.len() && requests[idx].0 <= time {
            pending.push((requests[idx].1, idx as u32));
            arrival.insert(idx as u32, requests[idx].0);
            idx += 1;
        }
        match policy.next_floor(at, &pending) {
            None => time = requests[idx].0,
            Some(f) => {
                let d = (f - at).abs();
                travelled += d;
                time += d as f64 * 2.5 + 4.0;
                at = f;
                pending.retain(|&(fl, id)| {
                    if fl == f { waits.push(time - arrival[&id]); false } else { true }
                });
            }
        }
    }
    let avg = waits.iter().sum::<f64>() / waits.len() as f64;
    let max = waits.iter().cloned().fold(0.0, f64::max);
    (avg, max, travelled)
}

fn main() {
    let mut sim = Sim {
        now: Time(0.0),
        queue: BinaryHeap::new(),
        seq: 0,
        rng: Rng(0x9E3779B97F4A7C15),
        counters: vec![Counter::default(); 3],
        line: VecDeque::new(),
        customers: HashMap::new(),
        next_customer: 1,
        open: true,
        log: vec![],
        max_line: 0,
        area_line: 0.0,
        last_change: 0.0,
    };
    sim.schedule(0.0, Ev::Arrive { customer: 1 });
    sim.schedule(1800.0, Ev::Break { counter: 1 });
    sim.schedule(2400.0, Ev::Break { counter: 2 });
    sim.schedule(3.0 * 3600.0, Ev::Close);
    let events = sim.run(100_000);
    println!("processed {} events, final time {}", events, sim.now);
    for l in &sim.log {
        println!("  {}", l);
    }
    let served: Vec<&Customer> = sim.customers.values().filter(|c| c.done.is_some()).collect();
    let mut waits: Vec<f64> = served.iter().map(|c| c.started.unwrap() - c.arrived).collect();
    waits.sort_by(f64::total_cmp);
    let balked = sim.customers.values().filter(|c| c.balked).count();
    let vips = served.iter().filter(|c| c.vip).count();
    println!("customers: {} arrived, {} served ({} VIP), {} balked", sim.customers.len(), served.len(), vips, balked);
    let mean = waits.iter().sum::<f64>() / waits.len() as f64;
    println!("wait: mean {:.2}s median {:.2}s p95 {:.2}s max {:.2}s", mean, waits[waits.len() / 2], waits[waits.len() * 95 / 100], waits.last().unwrap());
    let vip_wait: Vec<f64> = served.iter().filter(|c| c.vip).map(|c| c.started.unwrap() - c.arrived).collect();
    println!("vip mean wait {:.2}s", vip_wait.iter().sum::<f64>() / vip_wait.len().max(1) as f64);
    println!("max line {}, avg line {:.3}", sim.max_line, sim.area_line / sim.now.0);
    for (i, c) in sim.counters.iter().enumerate() {
        println!("counter {}: served {:>3}, utilisation {:>5.1}%", i, c.served, c.busy_time / sim.now.0 * 100.0);
    }
    for line in histogram(&waits, 120.0, 30) {
        println!("{}", line);
    }
    let sys: Vec<f64> = served.iter().map(|c| c.done.unwrap() - c.arrived).collect();
    let total: f64 = sys.iter().sum();
    println!("time in system: total {:.1}s mean {:.2}s", total, total / sys.len() as f64);

    let mut rng = Rng(12345);
    let mut t = 0.0;
    let reqs: Vec<(f64, i32)> = (0..60).map(|_| { t += rng.exp(8.0); (t, (rng.uniform() * 20.0) as i32) }).collect();
    let mut policies: Vec<Box<dyn Policy>> = vec![Box::new(Nearest), Box::new(Sweep { up: true })];
    for p in policies.iter_mut() {
        let (avg, max, dist) = elevator(p.as_mut(), &reqs);
        println!("elevator {:<8} avg wait {:>7.2} max wait {:>7.2} floors travelled {}", p.name(), avg, max, dist);
    }
    println!("{} {} {}", Time(0.0), Time(59.999), Time(3725.5));
    let mut times = vec![Time(3.0), Time(-0.0), Time(0.0), Time(f64::INFINITY), Time(1.5)];
    times.sort();
    println!("{:?}", times);
}
