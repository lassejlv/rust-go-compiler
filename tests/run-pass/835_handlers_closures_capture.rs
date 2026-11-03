use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;
use std::cell::RefCell;
trait Handler<E> { fn handle(&mut self, e: &E) -> bool; }
impl<E, F: FnMut(&E) -> bool> Handler<E> for F { fn handle(&mut self, e: &E) -> bool { self(e) } }
struct Bus<'a, E> { hs: Vec<Box<dyn Handler<E> + 'a>> }
fn apply<F>(f: F) -> usize where F: for<'a> Fn(&'a str) -> &'a str { f("hello world").len() }
fn pick<'a>(xs: &'a [String], f: &dyn Fn(&String) -> bool) -> Vec<&'a str> { xs.iter().filter(|s| f(s)).map(|s| s.as_str()).collect() }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)] enum K { A, B }
fn main() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let l2 = log.clone();
    let mut count = 0;
    {
        let mut bus: Bus<i32> = Bus { hs: vec![] };
        bus.hs.push(Box::new(move |e: &i32| { l2.borrow_mut().push(*e); true }));
        bus.hs.push(Box::new(|e: &i32| { count += *e; *e > 2 }));
        for e in 1..5 { let r: Vec<bool> = bus.hs.iter_mut().map(|h| h.handle(&e)).collect(); println!("{:?}", r); }
    }
    println!("{:?} {}", log.borrow(), count);
    println!("{}", apply(|s| s.split(' ').next().unwrap()));
    let xs = vec!["ab".to_string(), "abc".into(), "x".into()];
    println!("{:?}", pick(&xs, &|s| s.len() > 1));
    let mut hm: HashMap<K, Vec<u8>> = HashMap::new();
    hm.entry(K::B).or_default().push(1);
    hm.entry(K::A).or_default().extend([2, 3]);
    let mut keys: Vec<_> = hm.iter().collect(); keys.sort();
    println!("{:?}", keys);
    let hs: HashSet<char> = "hello".chars().collect();
    let mut v: Vec<_> = hs.into_iter().collect(); v.sort_unstable(); println!("{:?}", v);
    let bm: BTreeMap<_, _> = hm.into_iter().collect();
    println!("{:?}", bm);
    let f: fn(i32) -> i32 = |x| x + 1;
    let g: &dyn Fn(i32) -> i32 = &f;
    println!("{}", g(g(1)));
    let fs: Vec<(&str, fn(f64) -> f64)> = vec![("sin", f64::sin), ("neg", std::ops::Neg::neg)];
    for (n, f) in &fs { println!("{} {:.3}", n, f(1.0)); }
    let s = String::from("abc");
    let c = || s.len();
    let d = c;
    println!("{} {}", c(), d());
    let mut v2 = vec![1, 2, 3];
    v2.push(v2.len());
    let i = v2.len() - 1;
    v2[i] += v2[0];
    println!("{:?}", v2);
    let t = (String::from("a"), String::from("b"));
    let cl = move || t.0.len();
    println!("{}", cl());
    let mut st = (vec![1], vec![2]);
    let mut push0 = |x| st.0.push(x);
    push0(5);
    st.1.push(9);
    println!("{:?}", st);
}
