// Generic traits used as trait objects: dyn Visitor<T>, Box<dyn Trait<Out = X>>
// with associated types, dyn traits with generic supertraits, trait objects
// inside generic structs, blanket impls bridging generic and dyn worlds,
// and dyn Any + generics for heterogeneous typed maps.
use std::any::Any;
use std::collections::BTreeMap;
use std::fmt::Debug;

trait Visitor<T> { fn visit(&mut self, item: &T); fn result(&self) -> String; }

struct Sum(i64);
impl Visitor<i64> for Sum { fn visit(&mut self, x: &i64) { self.0 += x; } fn result(&self) -> String { format!("sum={}", self.0) } }
struct Longest(String);
impl Visitor<String> for Longest {
    fn visit(&mut self, s: &String) { if s.len() > self.0.len() { self.0 = s.clone(); } }
    fn result(&self) -> String { format!("longest={:?}", self.0) }
}
struct CountAll(usize);
impl<T> Visitor<T> for CountAll { fn visit(&mut self, _: &T) { self.0 += 1; } fn result(&self) -> String { format!("count={}", self.0) } }
struct DebugAll(Vec<String>);
impl<T: Debug> Visitor<T> for DebugAll { fn visit(&mut self, x: &T) { self.0.push(format!("{:?}", x)); } fn result(&self) -> String { self.0.join(";") } }

fn walk<T>(items: &[T], visitors: &mut [Box<dyn Visitor<T> + '_>]) -> Vec<String> {
    for it in items { for v in visitors.iter_mut() { v.visit(it); } }
    visitors.iter().map(|v| v.result()).collect()
}

trait Producer { type Out: Debug; fn produce(&mut self) -> Option<Self::Out>; }
struct Countdown(u8);
impl Producer for Countdown { type Out = u8; fn produce(&mut self) -> Option<u8> { if self.0 == 0 { None } else { self.0 -= 1; Some(self.0) } } }
struct Words<'a>(std::str::SplitWhitespace<'a>);
impl<'a> Producer for Words<'a> { type Out = &'a str; fn produce(&mut self) -> Option<&'a str> { self.0.next() } }

fn drain_all<O: Debug>(p: &mut dyn Producer<Out = O>) -> Vec<O> { let mut v = vec![]; while let Some(x) = p.produce() { v.push(x); } v }

trait Shape { fn area(&self) -> f64; }
trait Named<Tag> { fn tag(&self) -> Tag; }
trait Entity: Shape + Named<&'static str> + Debug {}
impl<T: Shape + Named<&'static str> + Debug> Entity for T {}
#[derive(Debug)] struct Circle(f64);
#[derive(Debug)] struct Square(f64);
impl Shape for Circle { fn area(&self) -> f64 { 3.0 * self.0 * self.0 } }
impl Shape for Square { fn area(&self) -> f64 { self.0 * self.0 } }
impl Named<&'static str> for Circle { fn tag(&self) -> &'static str { "circle" } }
impl Named<&'static str> for Square { fn tag(&self) -> &'static str { "square" } }

struct Registry<K: Ord, V: ?Sized> { items: BTreeMap<K, Box<V>> }
impl<K: Ord + Debug, V: ?Sized + Entity> Registry<K, V> {
    fn total(&self) -> f64 { self.items.values().map(|e| e.area()).sum() }
    fn tags(&self) -> Vec<String> { self.items.iter().map(|(k, v)| format!("{:?}:{}", k, v.tag())).collect() }
}

trait Transform<I> { type O; fn apply(&self, i: I) -> Self::O; }
impl<I, O, F: Fn(I) -> O> Transform<I> for F { type O = O; fn apply(&self, i: I) -> O { self(i) } }
fn run_all<I: Copy>(input: I, ts: &[&dyn Transform<I, O = String>]) -> Vec<String> { ts.iter().map(|t| t.apply(input)).collect() }

#[derive(Default)]
struct TypeMap { m: BTreeMap<&'static str, Box<dyn Any>> }
impl TypeMap {
    fn insert<T: Any>(&mut self, v: T) { self.m.insert(std::any::type_name::<T>(), Box::new(v)); }
    fn get<T: Any>(&self) -> Option<&T> { self.m.get(std::any::type_name::<T>())?.downcast_ref() }
    fn get_mut<T: Any>(&mut self) -> Option<&mut T> { self.m.get_mut(std::any::type_name::<T>())?.downcast_mut() }
}

fn main() {
    let nums = [3i64, -1, 10, 7];
    let mut vs: Vec<Box<dyn Visitor<i64>>> = vec![Box::new(Sum(0)), Box::new(CountAll(0)), Box::new(DebugAll(vec![]))];
    println!("{:?}", walk(&nums, &mut vs));
    let words: Vec<String> = "a quick brownish fox".split(' ').map(String::from).collect();
    let mut ws: Vec<Box<dyn Visitor<String>>> = vec![Box::new(Longest(String::new())), Box::new(CountAll(0)), Box::new(DebugAll(vec![]))];
    println!("{:?}", walk(&words, &mut ws));

    let mut cd = Countdown(4);
    let mut wd = Words("x yy zzz".split_whitespace());
    println!("{:?} {:?}", drain_all(&mut cd), drain_all(&mut wd));
    let mut producers: Vec<Box<dyn Producer<Out = u8>>> = vec![Box::new(Countdown(2)), Box::new(Countdown(3))];
    let all: Vec<u8> = producers.iter_mut().flat_map(|p| drain_all(p.as_mut())).collect();
    println!("boxed producers {:?}", all);

    let mut reg: Registry<u32, dyn Entity> = Registry { items: BTreeMap::new() };
    reg.items.insert(2, Box::new(Square(3.0)));
    reg.items.insert(1, Box::new(Circle(1.0)));
    println!("total {} tags {:?} first {:?}", reg.total(), reg.tags(), reg.items.values().next());

    let shout = |x: i32| format!("{}!", x);
    let neg = |x: i32| format!("{}", -x);
    let bin = |x: i32| format!("{:b}", x);
    println!("{:?}", run_all(6, &[&shout, &neg, &bin]));

    let mut tm = TypeMap::default();
    tm.insert(42u32);
    tm.insert(String::from("hello"));
    tm.insert(vec![1.5f64, 2.5]);
    if let Some(s) = tm.get_mut::<String>() { s.push_str(" world"); }
    println!("{:?} {:?} {:?} {:?}", tm.get::<u32>(), tm.get::<String>(), tm.get::<Vec<f64>>(), tm.get::<i64>());
    println!("keys {:?}", tm.m.keys().collect::<Vec<_>>());
}
