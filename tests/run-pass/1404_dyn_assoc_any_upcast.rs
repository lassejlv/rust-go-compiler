use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::thread;

trait Source {
    type Item;
    fn next_item(&mut self) -> Option<Self::Item>;
    fn take_n(&mut self, n: usize) -> Vec<Self::Item> where Self: Sized {
        let mut v = Vec::new();
        for _ in 0..n {
            match self.next_item() {
                Some(x) => v.push(x),
                None => break,
            }
        }
        v
    }
    fn map_items<F, U>(self, f: F) -> Mapped<Self, F> where Self: Sized, F: FnMut(Self::Item) -> U {
        Mapped { s: self, f }
    }
}

struct Mapped<S, F> { s: S, f: F }
impl<S: Source, U, F: FnMut(S::Item) -> U> Source for Mapped<S, F> {
    type Item = U;
    fn next_item(&mut self) -> Option<U> {
        self.s.next_item().map(&mut self.f)
    }
}

struct Counter(u32);
impl Source for Counter {
    type Item = u32;
    fn next_item(&mut self) -> Option<u32> {
        if self.0 < 5 { self.0 += 1; Some(self.0) } else { None }
    }
}

fn drain(s: &mut dyn Source<Item = u32>) -> u32 {
    let mut t = 0;
    while let Some(x) = s.next_item() { t += x; }
    t
}

fn drain_box<T: Debug>(mut s: Box<dyn Source<Item = T> + '_>) -> Vec<String> {
    let mut out = vec![];
    while let Some(x) = s.next_item() { out.push(format!("{:?}", x)); }
    out
}

trait Animal: Any + Debug {
    fn speak(&self) -> String;
    fn as_any(&self) -> &dyn Any;
}
#[derive(Debug)] struct Cat { lives: u8 }
#[derive(Debug)] struct Cow;
impl Animal for Cat {
    fn speak(&self) -> String { "meow".into() }
    fn as_any(&self) -> &dyn Any { self }
}
impl Animal for Cow {
    fn speak(&self) -> String { "moo".into() }
    fn as_any(&self) -> &dyn Any { self }
}

trait Base { fn base(&self) -> i32; }
trait Derived: Base { fn derived(&self) -> i32 { self.base() * 10 } }
struct Thing(i32);
impl Base for Thing { fn base(&self) -> i32 { self.0 } }
impl Derived for Thing {}

fn use_base(b: &dyn Base) -> i32 { b.base() + 1 }

type Job = Box<dyn FnOnce() -> String + Send + 'static>;

fn run_jobs(jobs: Vec<Job>) -> Vec<String> {
    let handles: Vec<_> = jobs.into_iter().map(|j| thread::spawn(j)).collect();
    handles.into_iter().map(|h| h.join().unwrap()).collect()
}

fn describe_any(v: &dyn Any) -> String {
    if let Some(i) = v.downcast_ref::<i32>() {
        format!("i32 {}", i)
    } else if let Some(s) = v.downcast_ref::<String>() {
        format!("String {:?}", s)
    } else if let Some(s) = v.downcast_ref::<&str>() {
        format!("&str {:?}", s)
    } else {
        "unknown".to_string()
    }
}

fn main() {
    let mut c = Counter(0);
    println!("{:?}", c.take_n(2));
    println!("{}", drain(&mut c));
    let mut c2 = Counter(1);
    let r: &mut dyn Source<Item = u32> = &mut c2;
    println!("{:?}", r.next_item());
    println!("{:?}", drain_box(Box::new(Counter(2).map_items(|x| x * x))));
    println!("{:?}", drain_box(Box::new(Counter(3).map_items(|x| format!("#{}", x)))));

    let zoo: Vec<Box<dyn Animal>> = vec![Box::new(Cat { lives: 9 }), Box::new(Cow)];
    for a in &zoo {
        print!("{} ", a.speak());
        if let Some(cat) = a.as_any().downcast_ref::<Cat>() {
            print!("(cat with {} lives) ", cat.lives);
        }
        println!("{:?}", a);
    }
    let anys: Vec<Box<dyn Any>> = vec![Box::new(5i32), Box::new(String::from("s")), Box::new("lit"), Box::new(1.5f64)];
    for a in &anys {
        println!("{}", describe_any(a.as_ref()));
    }
    let b: Box<dyn Any> = Box::new(vec![1, 2, 3]);
    match b.downcast::<Vec<i32>>() {
        Ok(v) => println!("vec len {}", v.len()),
        Err(_) => println!("not vec"),
    }

    let t = Thing(4);
    let d: &dyn Derived = &t;
    println!("{} {}", d.derived(), use_base(d));
    let bd: Box<dyn Derived> = Box::new(Thing(7));
    let bb: Box<dyn Base> = bd;
    println!("{}", bb.base());
    let ad: Arc<dyn Derived + Send + Sync> = Arc::new(Thing(2));
    let ab: Arc<dyn Base + Send + Sync> = ad.clone();
    println!("{} {}", ab.base(), Arc::strong_count(&ad));

    let shared = Arc::new(Mutex::new(HashMap::new()));
    let mut jobs: Vec<Job> = Vec::new();
    for i in 0..3 {
        let m = Arc::clone(&shared);
        jobs.push(Box::new(move || {
            m.lock().unwrap().insert(i, i * i);
            format!("job{}", i)
        }));
    }
    println!("{:?}", run_jobs(jobs));
    let mut keys: Vec<_> = shared.lock().unwrap().iter().map(|(k, v)| (*k, *v)).collect();
    keys.sort();
    println!("{:?}", keys);
}
