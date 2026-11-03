use std::fmt;

trait Named {
    fn name(&self) -> String;
}

trait Shape: Named + fmt::Debug {
    fn area(&self) -> f64;
    fn perimeter(&self) -> f64;
    fn describe(&self) -> String {
        format!("{} area={:.3} perim={:.3}", self.name(), self.area(), self.perimeter())
    }
}

#[derive(Debug, Clone)]
struct Circle { r: f64 }
#[derive(Debug, Clone)]
struct Rect { w: f64, h: f64 }
#[derive(Debug, Clone)]
struct Poly<const N: usize> { pts: [(f64, f64); N] }

impl Named for Circle { fn name(&self) -> String { "circle".to_string() } }
impl Named for Rect { fn name(&self) -> String { if self.w == self.h { "square".into() } else { "rect".into() } } }
impl<const N: usize> Named for Poly<N> { fn name(&self) -> String { format!("poly{}", N) } }

impl Shape for Circle {
    fn area(&self) -> f64 { std::f64::consts::PI * self.r * self.r }
    fn perimeter(&self) -> f64 { 2.0 * std::f64::consts::PI * self.r }
}
impl Shape for Rect {
    fn area(&self) -> f64 { self.w * self.h }
    fn perimeter(&self) -> f64 { 2.0 * (self.w + self.h) }
    fn describe(&self) -> String { format!("[{}x{}] {}", self.w, self.h, self.name()) }
}
impl<const N: usize> Shape for Poly<N> {
    fn area(&self) -> f64 {
        let mut s = 0.0;
        for i in 0..N {
            let (x1, y1) = self.pts[i];
            let (x2, y2) = self.pts[(i + 1) % N];
            s += x1 * y2 - x2 * y1;
        }
        (s / 2.0).abs()
    }
    fn perimeter(&self) -> f64 {
        (0..N).map(|i| {
            let (x1, y1) = self.pts[i];
            let (x2, y2) = self.pts[(i + 1) % N];
            ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt()
        }).sum()
    }
}

struct Registry<T: ?Sized> {
    items: Vec<Box<T>>,
}

impl<T: ?Sized + Shape> Registry<T> {
    fn total_area(&self) -> f64 { self.items.iter().map(|s| s.area()).sum() }
}

impl Registry<dyn Shape> {
    fn new() -> Self { Registry { items: Vec::new() } }
    fn add<S: Shape + 'static>(&mut self, s: S) -> &mut Self { self.items.push(Box::new(s)); self }
    fn largest(&self) -> Option<&dyn Shape> {
        self.items.iter().map(|b| b.as_ref()).max_by(|a, b| a.area().partial_cmp(&b.area()).unwrap())
    }
}

fn print_named(n: &dyn Named) { println!("named: {}", n.name()); }

fn upcast_all(v: &[Box<dyn Shape>]) {
    for s in v {
        let n: &dyn Named = s.as_ref();
        print_named(n);
    }
}

fn generic_sum<I, S>(it: I) -> f64 where I: IntoIterator<Item = S>, S: AsRef<dyn Shape> {
    it.into_iter().map(|s| s.as_ref().perimeter()).sum()
}

struct Wrap(Box<dyn Shape>);
impl AsRef<dyn Shape> for Wrap { fn as_ref(&self) -> &(dyn Shape + 'static) { &*self.0 } }

fn main() {
    let mut reg = Registry::new();
    reg.add(Circle { r: 1.5 })
        .add(Rect { w: 2.0, h: 3.0 })
        .add(Rect { w: 4.0, h: 4.0 })
        .add(Poly { pts: [(0.0, 0.0), (4.0, 0.0), (0.0, 3.0)] })
        .add(Poly { pts: [(0.0, 0.0), (2.0, 0.0), (3.0, 2.0), (1.0, 3.0), (-1.0, 1.0)] });
    for s in &reg.items {
        println!("{}", s.describe());
    }
    println!("total area = {:.4}", reg.total_area());
    if let Some(l) = reg.largest() {
        println!("largest: {:?}", l);
    }
    upcast_all(&reg.items);
    let wraps: Vec<Wrap> = vec![Wrap(Box::new(Circle { r: 0.5 })), Wrap(Box::new(Rect { w: 1.0, h: 0.5 }))];
    println!("perim sum = {:.5}", generic_sum(wraps));
    let mut sorted: Vec<&Box<dyn Shape>> = reg.items.iter().collect();
    sorted.sort_by(|a, b| b.perimeter().partial_cmp(&a.perimeter()).unwrap());
    let names: Vec<String> = sorted.iter().map(|s| s.name()).collect();
    println!("{}", names.join(" > "));
    println!("{:#?}", reg.items[1]);
}
