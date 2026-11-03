// dyn-compatible traits with generic methods gated by `where Self: Sized`
use std::fmt::Debug;

trait Shape {
    fn area(&self) -> f64;
    fn name(&self) -> String;
    fn scaled_by<T: Into<f64>>(&self, k: T) -> f64
    where
        Self: Sized,
    {
        self.area() * k.into()
    }
    fn describe(&self) -> String {
        format!("{} with area {:.2}", self.name(), self.area())
    }
    fn boxed(self) -> Box<dyn Shape>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

#[derive(Debug, Clone)]
struct Circle(f64);
#[derive(Debug, Clone)]
struct Rect {
    w: f64,
    h: f64,
}
#[derive(Debug, Clone)]
struct Tri {
    b: f64,
    h: f64,
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        3.0 * self.0 * self.0
    }
    fn name(&self) -> String {
        "circle".to_string()
    }
}
impl Shape for Rect {
    fn area(&self) -> f64 {
        self.w * self.h
    }
    fn name(&self) -> String {
        String::from("rect")
    }
    fn describe(&self) -> String {
        format!("rect {}x{}", self.w, self.h)
    }
}
impl Shape for Tri {
    fn area(&self) -> f64 {
        0.5 * self.b * self.h
    }
    fn name(&self) -> String {
        "tri".into()
    }
}

trait Visitor {
    fn visit(&mut self, s: &dyn Shape);
    fn visit_all<'a, I>(&mut self, it: I)
    where
        I: IntoIterator<Item = &'a dyn Shape>,
        Self: Sized,
    {
        for s in it {
            self.visit(s);
        }
    }
}

struct Summer {
    total: f64,
    names: Vec<String>,
}
impl Visitor for Summer {
    fn visit(&mut self, s: &dyn Shape) {
        self.total += s.area();
        self.names.push(s.name());
    }
}

fn total_dyn(v: &mut dyn Visitor, shapes: &[Box<dyn Shape>]) {
    for s in shapes {
        v.visit(s.as_ref());
    }
}

trait Container {
    fn len(&self) -> usize;
    fn get_dbg(&self, i: usize) -> String;
    fn map_all<F: Fn(usize) -> usize>(&self, f: F) -> Vec<usize>
    where
        Self: Sized,
    {
        (0..self.len()).map(f).collect()
    }
}

impl<T: Debug> Container for Vec<T> {
    fn len(&self) -> usize {
        Vec::len(self)
    }
    fn get_dbg(&self, i: usize) -> String {
        format!("{:?}", self[i])
    }
}

fn print_container(c: &dyn Container) {
    let parts: Vec<String> = (0..c.len()).map(|i| c.get_dbg(i)).collect();
    println!("[{}]", parts.join("; "));
}

fn main() {
    let c = Circle(2.0);
    let r = Rect { w: 3.0, h: 4.5 };
    let t = Tri { b: 5.0, h: 2.0 };
    println!("{}", c.scaled_by(2));
    println!("{}", r.scaled_by(0.5f32));
    println!("{}", t.scaled_by(3u8));
    let shapes: Vec<Box<dyn Shape>> = vec![c.clone().boxed(), r.clone().boxed(), t.clone().boxed()];
    for s in &shapes {
        println!("{}", s.describe());
    }
    let mut sm = Summer { total: 0.0, names: vec![] };
    total_dyn(&mut sm, &shapes);
    println!("total={} names={:?}", sm.total, sm.names);
    let mut sm2 = Summer { total: 0.0, names: vec![] };
    let refs: Vec<&dyn Shape> = vec![&c, &r, &t, &c];
    sm2.visit_all(refs.iter().copied());
    println!("total2={} n={}", sm2.total, sm2.names.len());

    let v = vec!["a", "bb", "ccc"];
    print_container(&v);
    println!("{:?}", v.map_all(|i| i * 10));
    let v2: Vec<Option<i32>> = vec![Some(1), None];
    print_container(&v2);
    let dynref: &dyn Container = &v2;
    println!("len via dyn = {}", dynref.len());
    let biggest = shapes
        .iter()
        .max_by(|a, b| a.area().partial_cmp(&b.area()).unwrap())
        .unwrap();
    println!("biggest: {}", biggest.name());
}
