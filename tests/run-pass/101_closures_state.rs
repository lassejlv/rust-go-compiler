use std::collections::HashMap;

struct EventBus {
    handlers: Vec<(String, Box<dyn Fn(&str) -> String>)>,
}

impl EventBus {
    fn new() -> Self {
        EventBus { handlers: Vec::new() }
    }
    fn on(&mut self, name: &str, f: Box<dyn Fn(&str) -> String>) {
        self.handlers.push((name.to_string(), f));
    }
    fn emit(&self, name: &str, payload: &str) {
        for (n, h) in &self.handlers {
            if n == name {
                println!("  [{}] {}", n, h(payload));
            }
        }
    }
}

struct Memo<F: Fn(u64) -> u64> {
    f: F,
    cache: HashMap<u64, u64>,
    hits: u32,
}

impl<F: Fn(u64) -> u64> Memo<F> {
    fn new(f: F) -> Self {
        Memo { f, cache: HashMap::new(), hits: 0 }
    }
    fn get(&mut self, x: u64) -> u64 {
        if let Some(&v) = self.cache.get(&x) {
            self.hits += 1;
            return v;
        }
        let v = (self.f)(x);
        self.cache.insert(x, v);
        v
    }
}

fn retry<F: FnMut() -> Result<u32, String>>(mut op: F, max: u32) -> Result<u32, String> {
    let mut last = String::new();
    for attempt in 1..=max {
        match op() {
            Ok(v) => {
                println!("  succeeded on attempt {}", attempt);
                return Ok(v);
            }
            Err(e) => {
                println!("  attempt {} failed: {}", attempt, e);
                last = e;
            }
        }
    }
    Err(last)
}

fn main() {
    let mut bus = EventBus::new();
    let prefix = String::from(">>");
    bus.on("greet", Box::new(move |p| format!("{} hello {}", prefix, p)));
    bus.on("greet", Box::new(|p| format!("length {}", p.len())));
    bus.on("bye", Box::new(|p| p.to_uppercase()));
    println!("emit greet:");
    bus.emit("greet", "world");
    println!("emit bye:");
    bus.emit("bye", "see you");
    println!("emit none:");
    bus.emit("nothing", "");

    let mut m = Memo::new(|x| (1..=x).map(|i| i * i).sum());
    for x in [10, 20, 10, 10, 30, 20] {
        println!("sum squares to {} = {}", x, m.get(x));
    }
    println!("cache hits = {}", m.hits);

    let mut calls = 0;
    let r = retry(
        || {
            calls += 1;
            if calls < 3 { Err(format!("timeout #{}", calls)) } else { Ok(calls * 100) }
        },
        5,
    );
    println!("retry result = {:?}", r);
    let r2 = retry(|| Err("always".to_string()), 2);
    println!("retry result = {:?}", r2);

    let mut history: Vec<i32> = Vec::new();
    {
        let mut record = |x: i32| {
            history.push(x);
            history.len()
        };
        record(5);
        record(7);
        println!("recorded count = {}", record(9));
    }
    println!("history = {:?}", history);

    let mut sorters: Vec<(&str, Box<dyn Fn(&mut Vec<i32>)>)> = Vec::new();
    sorters.push(("asc", Box::new(|v| v.sort())));
    sorters.push(("desc", Box::new(|v| v.sort_by(|a, b| b.cmp(a)))));
    sorters.push(("abs", Box::new(|v| v.sort_by_key(|x| x.abs()))));
    for (name, s) in &sorters {
        let mut data = vec![3, -7, 1, -2, 5];
        s(&mut data);
        println!("{}: {:?}", name, data);
    }
}
