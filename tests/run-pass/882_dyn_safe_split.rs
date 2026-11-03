use std::fmt::Debug;

// Object-safe core
trait Storage {
    fn get_raw(&self, key: &str) -> Option<String>;
    fn put_raw(&mut self, key: &str, val: String);
    fn keys(&self) -> Vec<String>;
}

// Generic extension methods (not object-safe on their own)
trait StorageExt: Storage {
    fn get<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.get_raw(key).and_then(|s| s.parse().ok())
    }
    fn put<T: ToString>(&mut self, key: &str, val: T) {
        self.put_raw(key, val.to_string());
    }
    fn get_or<T: std::str::FromStr>(&self, key: &str, d: T) -> T {
        self.get(key).unwrap_or(d)
    }
}
impl<S: Storage + ?Sized> StorageExt for S {}

#[derive(Default, Debug)]
struct VecStore { items: Vec<(String, String)> }
impl Storage for VecStore {
    fn get_raw(&self, key: &str) -> Option<String> {
        self.items.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }
    fn put_raw(&mut self, key: &str, val: String) {
        if let Some(e) = self.items.iter_mut().find(|(k, _)| k == key) {
            e.1 = val;
        } else {
            self.items.push((key.to_string(), val));
        }
    }
    fn keys(&self) -> Vec<String> { self.items.iter().map(|(k, _)| k.clone()).collect() }
}

struct Prefixed<S> { inner: S, prefix: String }
impl<S: Storage> Storage for Prefixed<S> {
    fn get_raw(&self, key: &str) -> Option<String> { self.inner.get_raw(&format!("{}{}", self.prefix, key)) }
    fn put_raw(&mut self, key: &str, val: String) { let k = format!("{}{}", self.prefix, key); self.inner.put_raw(&k, val) }
    fn keys(&self) -> Vec<String> { self.inner.keys() }
}

trait Visitor {
    fn visit_num(&mut self, n: i64);
    fn visit_str(&mut self, s: &str);
}
trait Visitable {
    fn accept(&self, v: &mut dyn Visitor);
    fn accept_generic<V: Visitor>(&self, v: &mut V) where Self: Sized { self.accept(v) }
}
impl Visitable for i64 { fn accept(&self, v: &mut dyn Visitor) { v.visit_num(*self) } }
impl Visitable for &str { fn accept(&self, v: &mut dyn Visitor) { v.visit_str(self) } }
#[derive(Debug, Default)]
struct Collect { nums: i64, strs: Vec<String> }
impl Visitor for Collect {
    fn visit_num(&mut self, n: i64) { self.nums += n }
    fn visit_str(&mut self, s: &str) { self.strs.push(s.to_uppercase()) }
}

fn use_dyn(s: &mut dyn Storage) {
    s.put("count", 5);
    s.put("pi", 3.25);
    let c: i32 = s.get("count").unwrap();
    s.put("count", c + 1);
}

fn main() {
    let mut vs = VecStore::default();
    use_dyn(&mut vs);
    println!("{:?}", vs);
    let pi: f64 = vs.get("pi").unwrap();
    println!("{} {:?} {}", pi, vs.get::<i32>("pi"), vs.get_or("missing", 9u8));
    let mut p = Prefixed { inner: vs, prefix: "app.".into() };
    use_dyn(&mut p);
    p.put("flag", true);
    println!("{:?} {:?}", p.get::<bool>("flag"), p.keys());
    let boxed: Box<dyn Storage> = Box::new(p);
    println!("{:?}", boxed.get::<i32>("count"));
    let items: Vec<Box<dyn Visitable>> = vec![Box::new(5i64), Box::new("ab"), Box::new(-2i64), Box::new("cd")];
    let mut c = Collect::default();
    for i in &items { i.accept(&mut c); }
    10i64.accept_generic(&mut c);
    println!("{:?}", c);
}
