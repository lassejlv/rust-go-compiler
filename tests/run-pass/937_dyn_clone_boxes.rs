use std::fmt::Debug;

trait Shape: ShapeClone + Debug {
    fn area(&self) -> f64;
    fn scale(&mut self, k: f64);
}
trait ShapeClone { fn clone_box(&self) -> Box<dyn Shape>; }
impl<T: Shape + Clone + 'static> ShapeClone for T {
    fn clone_box(&self) -> Box<dyn Shape> { Box::new(self.clone()) }
}
impl Clone for Box<dyn Shape> { fn clone(&self) -> Self { self.clone_box() } }

#[derive(Debug, Clone)] struct Sq(f64);
#[derive(Debug, Clone)] struct Rect(f64, f64);
impl Shape for Sq { fn area(&self) -> f64 { self.0 * self.0 } fn scale(&mut self, k: f64) { self.0 *= k } }
impl Shape for Rect { fn area(&self) -> f64 { self.0 * self.1 } fn scale(&mut self, k: f64) { self.0 *= k; self.1 *= k } }

#[derive(Debug, Clone)]
struct Scene { shapes: Vec<Box<dyn Shape>>, name: String }

trait Plugin { fn run(&self, input: &str) -> String; fn name(&self) -> String; }
impl<F: Fn(&str) -> String> Plugin for (&'static str, F) {
    fn run(&self, input: &str) -> String { (self.1)(input) }
    fn name(&self) -> String { self.0.to_string() }
}
impl<P: Plugin + ?Sized> Plugin for Box<P> {
    fn run(&self, input: &str) -> String { format!("[{}]", (**self).run(input)) }
    fn name(&self) -> String { (**self).name() }
}
fn run_plugin<P: Plugin>(p: &P, s: &str) -> String { format!("{}: {}", p.name(), p.run(s)) }

fn main() {
    let s = Scene { shapes: vec![Box::new(Sq(2.0)), Box::new(Rect(1.0, 3.0))], name: "a".into() };
    let mut s2 = s.clone();
    s2.name.push('2');
    for sh in s2.shapes.iter_mut() { sh.scale(2.0); }
    println!("{:?}", s);
    println!("{:?}", s2);
    let total: f64 = s.shapes.iter().chain(s2.shapes.iter()).map(|x| x.area()).sum();
    println!("{}", total);
    let b: Box<dyn Shape> = Box::new(Sq(1.5));
    let c = b.clone();
    println!("{:?} {:?} {}", b, c, c.area());
    let up = ("upper", |s: &str| s.to_uppercase());
    let rev = ("rev", |s: &str| s.chars().rev().collect::<String>());
    println!("{}", run_plugin(&up, "abc"));
    println!("{}", run_plugin(&rev, "abc"));
    let boxed: Box<dyn Plugin> = Box::new(("len", |s: &str| s.len().to_string()));
    println!("{}", run_plugin(&boxed, "hello"));
    let bb = Box::new(Box::new(up));
    println!("{}", run_plugin(&bb, "x"));
}
