// Trait objects whose traits carry generic methods gated by `where Self: Sized`,
// plus object-safe methods that are still callable through `dyn`.
use std::fmt::Debug;

trait Shape {
    fn name(&self) -> String;
    fn area(&self) -> f64;

    // generic method: not callable on dyn, but trait stays object safe
    fn describe_with<F: Fn(&str, f64) -> String>(&self, f: F) -> String
    where
        Self: Sized,
    {
        f(&self.name(), self.area())
    }

    fn scaled(self, k: f64) -> Self
    where
        Self: Sized;

    fn summary(&self) -> String {
        format!("{} with area {:.2}", self.name(), self.area())
    }
}

#[derive(Debug, Clone)]
struct Circle {
    r: f64,
}
#[derive(Debug, Clone)]
struct Rect {
    w: f64,
    h: f64,
}

impl Shape for Circle {
    fn name(&self) -> String {
        "circle".to_string()
    }
    fn area(&self) -> f64 {
        3.0 * self.r * self.r
    }
    fn scaled(self, k: f64) -> Self {
        Circle { r: self.r * k }
    }
}

impl Shape for Rect {
    fn name(&self) -> String {
        format!("rect{}x{}", self.w, self.h)
    }
    fn area(&self) -> f64 {
        self.w * self.h
    }
    fn scaled(self, k: f64) -> Self {
        Rect { w: self.w * k, h: self.h * k }
    }
    fn summary(&self) -> String {
        format!("[R] {}", self.area())
    }
}

trait Visitor {
    fn visit(&mut self, s: &dyn Shape);
    fn visit_all<'a, I>(&mut self, it: I)
    where
        I: IntoIterator<Item = &'a Box<dyn Shape>>,
        Self: Sized,
    {
        for s in it {
            self.visit(s.as_ref());
        }
    }
}

struct AreaSum(f64, usize);
impl Visitor for AreaSum {
    fn visit(&mut self, s: &dyn Shape) {
        self.0 += s.area();
        self.1 += 1;
    }
}

fn largest<'a>(shapes: &'a [Box<dyn Shape>]) -> Option<&'a dyn Shape> {
    let mut best: Option<&dyn Shape> = None;
    for s in shapes {
        match best {
            Some(b) if b.area() >= s.area() => {}
            _ => best = Some(s.as_ref()),
        }
    }
    best
}

fn show_generic<S: Shape + Debug>(s: S) {
    let d = s.describe_with(|n, a| format!("{}={:.1}", n, a));
    println!("generic: {} / {:?}", d, s);
    let big = s.scaled(2.0);
    println!("scaled: {}", big.summary());
}

fn main() {
    let shapes: Vec<Box<dyn Shape>> = vec![
        Box::new(Circle { r: 1.0 }),
        Box::new(Rect { w: 2.0, h: 5.0 }),
        Box::new(Circle { r: 2.5 }),
        Box::new(Rect { w: 0.5, h: 0.5 }),
    ];
    for s in shapes.iter() {
        println!("{}", s.summary());
    }
    let mut v = AreaSum(0.0, 0);
    v.visit_all(&shapes);
    println!("total {:.3} over {}", v.0, v.1);

    if let Some(l) = largest(&shapes) {
        println!("largest: {}", l.name());
    }

    show_generic(Circle { r: 0.5 });
    show_generic(Rect { w: 3.0, h: 1.5 });

    // dyn reference to a concrete value, then dyn-only call
    let c = Circle { r: 3.0 };
    let r: &dyn Shape = &c;
    println!("via ref: {}", r.summary());
    // concrete calls the generic method while also viewing as dyn
    println!("{}", c.describe_with(|n, _| n.to_uppercase()));

    let mut names: Vec<String> = shapes.iter().map(|s| s.name()).collect();
    names.sort();
    names.dedup();
    println!("{:?}", names);
}
