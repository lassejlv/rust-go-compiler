use std::ops::Add;
use std::fmt::Debug;

trait Convert<T = String> {
    fn convert(&self) -> T;
}
struct N(i32);
impl Convert for N { fn convert(&self) -> String { format!("N{}", self.0) } }
impl Convert<f64> for N { fn convert(&self) -> f64 { self.0 as f64 / 2.0 } }
impl Convert<Vec<i32>> for N { fn convert(&self) -> Vec<i32> { (0..self.0).collect() } }

fn conv_default<C: Convert>(c: &C) -> String { c.convert() }
fn conv_to<T, C: Convert<T>>(c: &C) -> T { c.convert() }

#[derive(Debug, Clone, Copy, PartialEq)]
struct Meters(f64);
#[derive(Debug, Clone, Copy, PartialEq)]
struct Millis(f64);
impl Add for Meters { type Output = Meters; fn add(self, o: Meters) -> Meters { Meters(self.0 + o.0) } }
impl Add<Millis> for Meters { type Output = Meters; fn add(self, o: Millis) -> Meters { Meters(self.0 + o.0 / 1000.0) } }

struct Container<T, A = Vec<T>> { items: A, _t: std::marker::PhantomData<T> }
impl<T: Debug> Container<T> {
    fn new_vec() -> Self { Container { items: Vec::new(), _t: std::marker::PhantomData } }
    fn add(&mut self, t: T) { self.items.push(t) }
    fn show(&self) -> String { format!("{:?}", self.items) }
}
impl<T: Debug + Ord> Container<T, std::collections::BTreeSet<T>> {
    fn new_set() -> Self { Container { items: std::collections::BTreeSet::new(), _t: std::marker::PhantomData } }
    fn add_s(&mut self, t: T) { self.items.insert(t); }
    fn show_s(&self) -> String { format!("{:?}", self.items) }
}

struct Wrapper<T = i32> { v: T }
impl Wrapper { fn default_kind(&self) -> &'static str { "i32 wrapper" } }
impl Wrapper<&str> { fn default_kind(&self) -> &'static str { "str wrapper" } }

fn sum_generic<T: Add<Output = T> + Copy>(a: T, b: T) -> T { a + b }
fn add_mixed<L: Add<R>, R>(l: L, r: R) -> L::Output { l + r }

fn main() {
    let n = N(4);
    println!("{}", conv_default(&n));
    let f: f64 = n.convert();
    let v: Vec<i32> = n.convert();
    let s: String = n.convert();
    println!("{} {:?} {}", f, v, s);
    println!("{}", conv_to::<f64, _>(&N(7)));
    println!("{:?}", conv_to::<Vec<i32>, N>(&N(2)));
    println!("{:?}", Meters(1.0) + Meters(2.5));
    println!("{:?}", Meters(1.0) + Millis(250.0));
    println!("{:?}", sum_generic(Meters(0.5), Meters(0.25)));
    println!("{:?} {:?} {}", add_mixed(Meters(2.0), Millis(1.0)), add_mixed(1u8, 2u8), add_mixed(String::from("a"), "b"));
    let mut c = Container::new_vec();
    c.add(3); c.add(1); c.add(3);
    println!("{}", c.show());
    let mut cs = Container::new_set();
    cs.add_s(3); cs.add_s(1); cs.add_s(3);
    println!("{}", cs.show_s());
    let w: Wrapper = Wrapper { v: 5 };
    let ws = Wrapper { v: "x" };
    println!("{} {} {} {}", w.default_kind(), w.v, ws.default_kind(), ws.v);
}
