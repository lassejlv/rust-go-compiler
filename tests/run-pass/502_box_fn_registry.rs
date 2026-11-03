// Box<dyn Fn> stored in structs and HashMaps, called in deterministic order
use std::collections::{BTreeMap, HashMap};

type BinOp = Box<dyn Fn(i64, i64) -> Option<i64>>;

struct Calculator {
    ops: HashMap<&'static str, BinOp>,
    unary: BTreeMap<String, Box<dyn Fn(i64) -> i64>>,
    log: Vec<String>,
}

impl Calculator {
    fn new() -> Self {
        let mut ops: HashMap<&'static str, BinOp> = HashMap::new();
        ops.insert("+", Box::new(|a, b| a.checked_add(b)));
        ops.insert("-", Box::new(|a, b| a.checked_sub(b)));
        ops.insert("*", Box::new(|a, b| a.checked_mul(b)));
        ops.insert("/", Box::new(|a, b| if b == 0 { None } else { Some(a / b) }));
        ops.insert("%", Box::new(|a, b| a.checked_rem(b)));
        let offset = 100;
        ops.insert("off", Box::new(move |a, b| Some(a + b + offset)));
        let mut unary: BTreeMap<String, Box<dyn Fn(i64) -> i64>> = BTreeMap::new();
        unary.insert("neg".to_string(), Box::new(|x| -x));
        unary.insert("sq".to_string(), Box::new(|x| x * x));
        unary.insert("abs".to_string(), Box::new(i64::abs));
        Calculator { ops, unary, log: Vec::new() }
    }

    fn apply(&mut self, op: &str, a: i64, b: i64) -> Option<i64> {
        let r = self.ops.get(op).and_then(|f| f(a, b));
        self.log.push(format!("{} {} {} = {:?}", a, op, b, r));
        r
    }

    fn register(&mut self, name: &'static str, f: impl Fn(i64, i64) -> Option<i64> + 'static) {
        self.ops.insert(name, Box::new(f));
    }
}

struct EventBus {
    handlers: Vec<(String, Box<dyn FnMut(&str) -> String>)>,
}

impl EventBus {
    fn on<F: FnMut(&str) -> String + 'static>(&mut self, name: &str, f: F) {
        self.handlers.push((name.to_string(), Box::new(f)));
    }
    fn emit(&mut self, ev: &str) -> Vec<String> {
        let mut out = vec![];
        for (n, h) in self.handlers.iter_mut() {
            if ev.starts_with(n.as_str()) {
                out.push(h(ev));
            }
        }
        out
    }
}

fn make_adder(n: i32) -> Box<dyn Fn(i32) -> i32> {
    Box::new(move |x| x + n)
}

fn compose_all(fs: Vec<Box<dyn Fn(i32) -> i32>>) -> impl Fn(i32) -> i32 {
    move |x| fs.iter().fold(x, |acc, f| f(acc))
}

fn main() {
    let mut c = Calculator::new();
    println!("{:?}", c.apply("+", 2, 3));
    println!("{:?}", c.apply("/", 7, 0));
    println!("{:?}", c.apply("*", i64::MAX, 2));
    println!("{:?}", c.apply("off", 1, 2));
    println!("{:?}", c.apply("^", 1, 2));
    c.register("pow", |a, b| a.checked_pow(b as u32));
    println!("{:?}", c.apply("pow", 3, 4));
    let mut keys: Vec<_> = c.ops.keys().copied().collect();
    keys.sort();
    println!("ops: {:?}", keys);
    for (name, f) in &c.unary {
        println!("{}(-7) = {}", name, f(-7));
    }
    for l in &c.log {
        println!("log: {}", l);
    }

    let mut bus = EventBus { handlers: vec![] };
    let mut count = 0;
    bus.on("click", move |e| {
        count += 1;
        format!("click#{} {}", count, e)
    });
    let mut seen = Vec::new();
    bus.on("key", move |e| {
        seen.push(e.len());
        format!("keys so far {:?}", seen)
    });
    bus.on("", |e| format!("any:{}", e));
    for ev in ["click:a", "keydown", "click:b", "scroll", "keyup"] {
        println!("{:?}", bus.emit(ev));
    }

    let f = compose_all(vec![make_adder(1), Box::new(|x| x * 10), make_adder(-3)]);
    println!("{} {}", f(0), f(5));
    let table: Vec<(&str, fn(f64) -> f64)> = vec![("sqrt", f64::sqrt), ("floor", f64::floor), ("neg", |x| -x)];
    for (n, g) in &table {
        println!("{} {}", n, g(10.5));
    }
}
