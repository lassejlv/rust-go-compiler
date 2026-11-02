use std::collections::HashMap;

fn make_counter(start: i32, step: i32) -> Box<dyn FnMut() -> i32> {
    let mut cur = start;
    Box::new(move || {
        let r = cur;
        cur += step;
        r
    })
}

struct Cache<F: Fn(u64) -> u64> {
    func: F,
    store: HashMap<u64, u64>,
    hits: u32,
}

impl<F: Fn(u64) -> u64> Cache<F> {
    fn new(func: F) -> Self {
        Cache { func, store: HashMap::new(), hits: 0 }
    }
    fn get(&mut self, k: u64) -> u64 {
        if let Some(v) = self.store.get(&k) {
            self.hits += 1;
            return *v;
        }
        let v = (self.func)(k);
        self.store.insert(k, v);
        v
    }
}

struct Button<'a> {
    label: String,
    on_click: Option<Box<dyn Fn(&str) -> String + 'a>>,
}

fn fib_memo(n: u64, memo: &mut HashMap<u64, u64>) -> u64 {
    if n <= 1 {
        return n;
    }
    if let Some(&v) = memo.get(&n) {
        return v;
    }
    let v = fib_memo(n - 1, memo) + fib_memo(n - 2, memo);
    memo.insert(n, v);
    v
}

fn grid_paths(r: usize, c: usize, memo: &mut HashMap<(usize, usize), u64>) -> u64 {
    if r == 0 || c == 0 {
        return 1;
    }
    if let Some(&v) = memo.get(&(r, c)) {
        return v;
    }
    let v = grid_paths(r - 1, c, memo) + grid_paths(r, c - 1, memo);
    memo.insert((r, c), v);
    v
}

fn apply_all(fs: &mut [Box<dyn FnMut() -> i32>]) -> Vec<i32> {
    fs.iter_mut().map(|f| f()).collect()
}

fn call_once<F: FnOnce() -> String>(f: F) -> String {
    f()
}

fn main() {
    let mut c1 = make_counter(0, 1);
    let mut c2 = make_counter(100, -10);
    println!("{} {} {}", c1(), c1(), c1());
    println!("{} {}", c2(), c2());
    let mut fs = vec![c1, c2, make_counter(5, 5)];
    println!("{:?}", apply_all(&mut fs));
    println!("{:?}", apply_all(&mut fs));

    let offset = 3;
    let mut cache = Cache::new(move |x| x * x + offset);
    println!("{} {} {}", cache.get(4), cache.get(5), cache.get(4));
    println!("hits {} size {}", cache.hits, cache.store.len());

    let prefix = String::from(">> ");
    let b1 = Button { label: "ok".into(), on_click: Some(Box::new(|who| format!("{}{} clicked", prefix, who))) };
    let b2 = Button { label: "none".into(), on_click: None };
    for b in [&b1, &b2] {
        match &b.on_click {
            Some(f) => println!("{}: {}", b.label, f("alice")),
            None => println!("{}: no handler", b.label),
        }
    }

    let mut memo = HashMap::new();
    println!("fib(90) = {}", fib_memo(90, &mut memo));
    println!("memo size {}", memo.len());
    let mut gm = HashMap::new();
    println!("paths(16,16) = {}", grid_paths(16, 16, &mut gm));

    let words = vec!["apple".to_string(), "kiwi".to_string()];
    let consume = move || words.join("+");
    println!("{}", call_once(consume));

    let mut total = 0;
    let mut add = |x: i32| total += x;
    add(5);
    add(7);
    println!("total {}", total);

    let multipliers: Vec<Box<dyn Fn(i32) -> i32>> = (1..=3).map(|m| Box::new(move |x| x * m) as Box<dyn Fn(i32) -> i32>).collect();
    println!("{:?}", multipliers.iter().map(|f| f(10)).collect::<Vec<_>>());

    let compose_all = |fs: &[Box<dyn Fn(i32) -> i32>], x: i32| fs.iter().fold(x, |acc, f| f(acc));
    println!("{}", compose_all(&multipliers, 1));

    let mut history: Vec<String> = Vec::new();
    let mut record = |event: &str| {
        history.push(format!("{}:{}", history.len(), event));
        history.len()
    };
    record("start");
    record("middle");
    let n = record("end");
    println!("{} {:?}", n, history);
}
