use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

struct Registry<K, V> {
    items: HashMap<K, V>,
    order: Vec<K>,
}

impl<K: Hash + Eq + Clone + Debug, V: Debug> Registry<K, V> {
    fn new() -> Self {
        Registry { items: HashMap::new(), order: Vec::new() }
    }
    fn add(&mut self, k: K, v: V) -> &mut Self {
        if !self.items.contains_key(&k) {
            self.order.push(k.clone());
        }
        self.items.insert(k, v);
        self
    }
    fn get(&self, k: &K) -> Option<&V> {
        self.items.get(k)
    }
    fn map_values<W: Debug, F: Fn(&V) -> W>(&self, f: F) -> Registry<K, W> {
        let mut r = Registry::new();
        for k in &self.order {
            r.add(k.clone(), f(&self.items[k]));
        }
        r
    }
    fn dump(&self) -> String {
        self.order.iter().map(|k| format!("{:?}={:?}", k, self.items[k])).collect::<Vec<_>>().join(" ")
    }
}

struct Wrapper<T>(T);

impl<T: Debug> Wrapper<T> {
    fn show(&self) -> String {
        format!("W({:?})", self.0)
    }
    fn convert<T2: From<T>>(self) -> Wrapper<T2> {
        Wrapper(T2::from(self.0))
    }
}

impl Wrapper<i32> {
    fn only_for_i32(&self) -> i32 {
        self.0 * 1000
    }
}

fn outer<T: Debug + Clone>(x: T) -> String {
    fn inner<T: Debug>(y: T, depth: u32) -> String {
        format!("{}:{:?}", depth, y)
    }
    let a = inner(x.clone(), 1);
    let b = inner(vec![x], 2);
    format!("{} | {}", a, b)
}

trait Transformer<T> {
    fn apply(&self, x: T) -> T;
}

struct AddN(i32);
struct Repeat(usize);

impl Transformer<i32> for AddN {
    fn apply(&self, x: i32) -> i32 {
        x + self.0
    }
}

impl Transformer<String> for Repeat {
    fn apply(&self, x: String) -> String {
        x.repeat(self.0)
    }
}

impl Transformer<i32> for Repeat {
    fn apply(&self, x: i32) -> i32 {
        x * self.0 as i32
    }
}

fn run_all<T: Clone, X: Transformer<T>>(t: &X, inputs: &[T]) -> Vec<T> {
    inputs.iter().cloned().map(|i| t.apply(i)).collect()
}

fn pick<T>(cond: bool, a: T, b: T) -> T {
    if cond { a } else { b }
}

fn main() {
    let mut r: Registry<&str, u32> = Registry::new();
    r.add("b", 2).add("a", 1).add("c", 3).add("b", 20);
    println!("{}", r.dump());
    println!("get a = {:?}, get z = {:?}", r.get(&"a"), r.get(&"z"));
    let doubled = r.map_values(|v| format!("<{}>", v * 2));
    println!("{}", doubled.dump());
    let w = Wrapper(7i32);
    println!("{} {}", w.show(), w.only_for_i32());
    let w64: Wrapper<i64> = w.convert();
    println!("{}", w64.show());
    let wf: Wrapper<f64> = Wrapper(3u8).convert();
    println!("{}", wf.show());
    println!("{}", outer("hi"));
    println!("{}", outer(3.5));
    println!("{:?}", run_all(&AddN(10), &[1, 2, 3]));
    println!("{:?}", run_all(&Repeat(3), &["ab".to_string(), "x".to_string()]));
    println!("{:?}", run_all::<i32, _>(&Repeat(3), &[1, 2]));
    println!("{} {}", pick(true, "yes", "no"), pick(false, 1.5, 2.5));
    let t: Vec<(String, usize)> = pick(false, vec![], vec![("k".to_string(), 1)]);
    println!("{:?}", t);
}
