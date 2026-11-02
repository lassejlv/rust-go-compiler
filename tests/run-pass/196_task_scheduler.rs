use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Task {
    id: u32,
    name: String,
    priority: Priority,
    duration: u32,
    deps: Vec<u32>,
    submitted: u32,
}

#[derive(Debug, PartialEq, Eq)]
struct Ready {
    priority: Priority,
    submitted: u32,
    id: u32,
}

impl Ord for Ready {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.submitted.cmp(&self.submitted))
            .then_with(|| other.id.cmp(&self.id))
    }
}
impl PartialOrd for Ready {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct Scheduler {
    tasks: BTreeMap<u32, Task>,
    workers: usize,
}

#[derive(Debug)]
struct Run {
    id: u32,
    worker: usize,
    start: u32,
    end: u32,
}

impl Scheduler {
    fn schedule(&self) -> Result<Vec<Run>, String> {
        let mut remaining_deps: HashMap<u32, usize> = HashMap::new();
        let mut dependents: HashMap<u32, Vec<u32>> = HashMap::new();
        for t in self.tasks.values() {
            for d in &t.deps {
                if !self.tasks.contains_key(d) {
                    return Err(format!("task {} depends on missing {}", t.id, d));
                }
                dependents.entry(*d).or_default().push(t.id);
            }
            remaining_deps.insert(t.id, t.deps.len());
        }
        let mut ready = BinaryHeap::new();
        for t in self.tasks.values() {
            if t.deps.is_empty() {
                ready.push(Ready { priority: t.priority, submitted: t.submitted, id: t.id });
            }
        }
        let mut worker_free = vec![0u32; self.workers];
        let mut finish_time: HashMap<u32, u32> = HashMap::new();
        let mut runs = Vec::new();
        // in-flight: (end time, id)
        let mut inflight: Vec<(u32, u32)> = Vec::new();
        let mut now = 0;
        while runs.len() < self.tasks.len() {
            // assign ready tasks to free workers
            while let Some(w) = (0..self.workers).find(|&w| worker_free[w] <= now) {
                let Some(r) = ready.pop() else { break };
                let t = &self.tasks[&r.id];
                let start = now.max(t.deps.iter().map(|d| finish_time[d]).max().unwrap_or(0));
                let end = start + t.duration;
                worker_free[w] = end;
                inflight.push((end, t.id));
                runs.push(Run { id: t.id, worker: w, start, end });
            }
            if inflight.is_empty() {
                if ready.is_empty() {
                    return Err("cycle detected".to_string());
                }
                continue;
            }
            // advance to next completion
            inflight.sort();
            let (t_end, done) = inflight.remove(0);
            now = t_end;
            finish_time.insert(done, t_end);
            if let Some(ds) = dependents.get(&done) {
                for &dep in ds {
                    let c = remaining_deps.get_mut(&dep).unwrap();
                    *c -= 1;
                    if *c == 0 {
                        let t = &self.tasks[&dep];
                        ready.push(Ready { priority: t.priority, submitted: t.submitted, id: dep });
                    }
                }
            }
        }
        Ok(runs)
    }
}

fn task(id: u32, name: &str, priority: Priority, duration: u32, deps: &[u32], submitted: u32) -> Task {
    Task { id, name: name.to_string(), priority, duration, deps: deps.to_vec(), submitted }
}

fn main() {
    use Priority::*;
    let mut s = Scheduler { tasks: BTreeMap::new(), workers: 2 };
    for t in [
        task(1, "fetch", Normal, 3, &[], 0),
        task(2, "compile", High, 5, &[1], 1),
        task(3, "lint", Low, 2, &[1], 2),
        task(4, "docs", Low, 4, &[], 3),
        task(5, "test", Critical, 4, &[2], 4),
        task(6, "package", High, 2, &[2, 3], 5),
        task(7, "hotfix", Critical, 1, &[], 6),
        task(8, "deploy", Critical, 1, &[5, 6], 7),
    ] {
        s.tasks.insert(t.id, t);
    }
    match s.schedule() {
        Ok(runs) => {
            for r in &runs {
                println!("{:<8} worker {} [{:>2}, {:>2})", s.tasks[&r.id].name, r.worker, r.start, r.end);
            }
            println!("makespan {}", runs.iter().map(|r| r.end).max().unwrap());
            let mut by_worker: BTreeMap<usize, Vec<&str>> = BTreeMap::new();
            for r in &runs {
                by_worker.entry(r.worker).or_default().push(&s.tasks[&r.id].name);
            }
            println!("{:?}", by_worker);
        }
        Err(e) => println!("error: {}", e),
    }
    s.workers = 3;
    let runs = s.schedule().unwrap();
    println!("3 workers makespan {}", runs.iter().map(|r| r.end).max().unwrap());
    s.tasks.get_mut(&1).unwrap().deps.push(8);
    println!("{:?}", s.schedule().err());
    s.tasks.get_mut(&1).unwrap().deps = vec![42];
    println!("{:?}", s.schedule().err());
    let mut heap: BinaryHeap<Ready> = BinaryHeap::new();
    heap.push(Ready { priority: Normal, submitted: 5, id: 1 });
    heap.push(Ready { priority: Normal, submitted: 2, id: 2 });
    heap.push(Ready { priority: Low, submitted: 0, id: 3 });
    heap.push(Ready { priority: High, submitted: 9, id: 4 });
    let order: Vec<u32> = std::iter::from_fn(|| heap.pop().map(|r| r.id)).collect();
    println!("{:?}", order);
}
