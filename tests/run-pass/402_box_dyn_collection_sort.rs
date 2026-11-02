// Box<dyn Trait> in Vec, HashMap and BTreeMap; sorting by dyn method results,
// retain/drain on boxed objects, and trait objects with mutable state.
use std::collections::{BTreeMap, HashMap};

trait Task {
    fn id(&self) -> u32;
    fn priority(&self) -> i32;
    fn run(&mut self) -> Option<String>;
    fn done(&self) -> bool;
}

struct Counter {
    id: u32,
    left: u32,
}
struct Printer {
    id: u32,
    msgs: Vec<&'static str>,
}

impl Task for Counter {
    fn id(&self) -> u32 {
        self.id
    }
    fn priority(&self) -> i32 {
        self.left as i32
    }
    fn run(&mut self) -> Option<String> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        Some(format!("counter#{} -> {}", self.id, self.left))
    }
    fn done(&self) -> bool {
        self.left == 0
    }
}

impl Task for Printer {
    fn id(&self) -> u32 {
        self.id
    }
    fn priority(&self) -> i32 {
        10 - self.msgs.len() as i32
    }
    fn run(&mut self) -> Option<String> {
        if self.msgs.is_empty() {
            None
        } else {
            Some(format!("printer#{}: {}", self.id, self.msgs.remove(0)))
        }
    }
    fn done(&self) -> bool {
        self.msgs.is_empty()
    }
}

fn main() {
    let mut tasks: Vec<Box<dyn Task>> = Vec::new();
    tasks.push(Box::new(Counter { id: 1, left: 3 }));
    tasks.push(Box::new(Printer { id: 2, msgs: vec!["a", "b"] }));
    tasks.push(Box::new(Counter { id: 3, left: 1 }));
    tasks.push(Box::new(Printer { id: 4, msgs: vec!["x", "y", "z", "w"] }));

    tasks.sort_by_key(|t| std::cmp::Reverse(t.priority()));
    let order: Vec<u32> = tasks.iter().map(|t| t.id()).collect();
    println!("order by priority: {:?}", order);

    let mut round = 0;
    while !tasks.is_empty() {
        round += 1;
        let mut outs = Vec::new();
        for t in tasks.iter_mut() {
            if let Some(s) = t.run() {
                outs.push(s);
            }
        }
        println!("round {}: {}", round, outs.join(", "));
        let before = tasks.len();
        tasks.retain(|t| !t.done());
        if tasks.len() != before {
            println!("  removed {}", before - tasks.len());
        }
    }

    // Map of name -> boxed closure-like strategies
    let mut ops: HashMap<&str, Box<dyn Fn(i64, i64) -> i64>> = HashMap::new();
    ops.insert("add", Box::new(|a, b| a + b));
    ops.insert("mul", Box::new(|a, b| a * b));
    ops.insert("pow", Box::new(|a, b| a.pow(b as u32)));
    let offset = 7;
    ops.insert("off", Box::new(move |a, b| a - b + offset));
    let mut keys: Vec<_> = ops.keys().copied().collect();
    keys.sort();
    for k in keys {
        println!("{} => {}", k, ops[k](3, 4));
    }

    // BTreeMap of boxed tasks, drained in key order
    let mut bt: BTreeMap<u32, Box<dyn Task>> = BTreeMap::new();
    for i in (0..4).rev() {
        bt.insert(i * 10, Box::new(Counter { id: i, left: i }));
    }
    let mut total = 0;
    for (k, t) in bt.iter_mut() {
        let mut n = 0;
        while t.run().is_some() {
            n += 1;
        }
        total += n;
        println!("key {} ran {} times, done={}", k, n, t.done());
    }
    println!("total runs {}", total);
    let drained: Vec<(u32, u32)> = std::mem::take(&mut bt).into_iter().map(|(k, t)| (k, t.id())).collect();
    println!("drained {:?}, left {}", drained, bt.len());
}
