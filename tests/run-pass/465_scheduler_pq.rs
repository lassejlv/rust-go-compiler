// A priority-queue scheduler simulation: BinaryHeap with Reverse and custom Ord (priority, then
// arrival seq for FIFO tie-breaking), tasks with remaining work and time slices (round-robin within
// priority), aging, deadlines, and a final report with per-task stats in a BTreeMap.
use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BinaryHeap};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Task {
    name: String,
    priority: u8, // higher runs first
    seq: u64,     // lower = earlier
    remaining: u32,
    deadline: Option<u32>,
}

impl Ord for Task {
    fn cmp(&self, o: &Self) -> Ordering {
        self.priority.cmp(&o.priority).then_with(|| o.seq.cmp(&self.seq))
    }
}
impl PartialOrd for Task {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

#[derive(Default, Debug)]
struct Stats {
    first_run: Option<u32>,
    finished: Option<u32>,
    slices: u32,
    missed: bool,
}

struct Scheduler {
    heap: BinaryHeap<Task>,
    seq: u64,
    clock: u32,
    slice: u32,
    stats: BTreeMap<String, Stats>,
    timeline: Vec<String>,
    arrivals: BinaryHeap<Reverse<(u32, u64, String, u8, u32, Option<u32>)>>,
}

impl Scheduler {
    fn new(slice: u32) -> Self {
        Scheduler { heap: BinaryHeap::new(), seq: 0, clock: 0, slice, stats: BTreeMap::new(), timeline: vec![], arrivals: BinaryHeap::new() }
    }
    fn submit_at(&mut self, at: u32, name: &str, prio: u8, work: u32, deadline: Option<u32>) {
        self.seq += 1;
        self.arrivals.push(Reverse((at, self.seq, name.to_string(), prio, work, deadline)));
    }
    fn admit(&mut self) {
        while let Some(Reverse((at, ..))) = self.arrivals.peek() {
            if *at > self.clock {
                break;
            }
            let Reverse((_, _, name, prio, work, dl)) = self.arrivals.pop().unwrap();
            self.seq += 1;
            self.stats.entry(name.clone()).or_default();
            self.heap.push(Task { name, priority: prio, seq: self.seq, remaining: work, deadline: dl });
        }
    }
    fn age(&mut self) {
        // every waiting task gains priority 1 (capped) each time slice
        let old = std::mem::take(&mut self.heap);
        self.heap = old.into_iter().map(|mut t| {
            t.priority = t.priority.saturating_add(1).min(9);
            t
        }).collect();
    }
    fn run(&mut self) {
        loop {
            self.admit();
            let Some(mut t) = self.heap.pop() else {
                match self.arrivals.peek() {
                    Some(Reverse((at, ..))) => {
                        self.timeline.push(format!("t={} idle->{}", self.clock, at));
                        self.clock = *at;
                        continue;
                    }
                    None => break,
                }
            };
            let clock = self.clock;
            let st = self.stats.get_mut(&t.name).unwrap();
            st.first_run.get_or_insert(clock);
            st.slices += 1;
            let ran = t.remaining.min(self.slice);
            self.timeline.push(format!("t={} {}(p{})x{}", self.clock, t.name, t.priority, ran));
            self.clock += ran;
            t.remaining -= ran;
            self.age();
            if t.remaining == 0 {
                let st = self.stats.get_mut(&t.name).unwrap();
                st.finished = Some(self.clock);
                st.missed = t.deadline.map_or(false, |d| self.clock > d);
            } else {
                self.seq += 1;
                t.seq = self.seq;
                self.admit();
                self.heap.push(t);
            }
        }
    }
}

fn main() {
    let mut s = Scheduler::new(3);
    s.submit_at(0, "compile", 5, 7, Some(20));
    s.submit_at(0, "lint", 2, 2, None);
    s.submit_at(1, "test", 5, 4, Some(12));
    s.submit_at(2, "deploy", 8, 2, Some(6));
    s.submit_at(30, "late", 1, 1, Some(30));
    s.submit_at(4, "docs", 0, 5, None);
    s.run();
    for line in &s.timeline {
        println!("{}", line);
    }
    println!("{:<8} {:>5} {:>6} {:>6} {:>6}", "task", "first", "finish", "slices", "missed");
    for (name, st) in &s.stats {
        println!("{:<8} {:>5} {:>6} {:>6} {:>6}", name, st.first_run.unwrap(), st.finished.unwrap(), st.slices, st.missed);
    }
    let total_wait: u32 = s.stats.values().map(|st| st.first_run.unwrap()).sum();
    println!("clock={} total_first_wait={}", s.clock, total_wait);

    // heap ordering sanity: equal priority pops FIFO
    let mut h = BinaryHeap::new();
    for (i, p) in [3u8, 1, 3, 2, 3].iter().enumerate() {
        h.push(Task { name: format!("t{}", i), priority: *p, seq: i as u64, remaining: 0, deadline: None });
    }
    let order: Vec<String> = std::iter::from_fn(|| h.pop().map(|t| t.name)).collect();
    println!("{:?}", order);
    let mut mins: BinaryHeap<Reverse<(u32, &str)>> = vec![Reverse((5, "e")), Reverse((1, "a")), Reverse((3, "c"))].into();
    mins.push(Reverse((2, "b")));
    println!("{:?} len={}", mins.peek(), mins.len());
    let drained: Vec<&str> = std::iter::from_fn(|| mins.pop().map(|Reverse((_, n))| n)).collect();
    println!("{:?}", drained);
}
