// Mixed static/dynamic dispatch: generic functions taking &dyn and impl Trait, trait objects for
// traits with generic supertraits (Into/From/AsRef), impl Trait in arg/return position, trait methods
// returning Box<dyn Trait>, PartialEq between trait objects via a dyn-safe eq method, and Clone for
// Box<dyn Trait> via a clone_box pattern.
use std::fmt::{self, Debug, Display};

trait Shape: Debug {
    fn area(&self) -> f64;
    fn name(&self) -> &'static str;
    fn clone_box(&self) -> Box<dyn Shape>;
    fn eq_dyn(&self, other: &dyn Shape) -> bool {
        self.name() == other.name() && (self.area() - other.area()).abs() < 1e-9
    }
    fn scaled(&self, k: f64) -> Box<dyn Shape>;
}

impl Clone for Box<dyn Shape> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
impl PartialEq for dyn Shape {
    fn eq(&self, o: &dyn Shape) -> bool {
        self.eq_dyn(o)
    }
}
impl Display for dyn Shape {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}({:.2})", self.name(), self.area())
    }
}

#[derive(Debug, Clone)]
struct Sq(f64);
#[derive(Debug, Clone)]
struct Tri(f64, f64);

impl Shape for Sq {
    fn area(&self) -> f64 { self.0 * self.0 }
    fn name(&self) -> &'static str { "sq" }
    fn clone_box(&self) -> Box<dyn Shape> { Box::new(self.clone()) }
    fn scaled(&self, k: f64) -> Box<dyn Shape> { Box::new(Sq(self.0 * k)) }
}
impl Shape for Tri {
    fn area(&self) -> f64 { 0.5 * self.0 * self.1 }
    fn name(&self) -> &'static str { "tri" }
    fn clone_box(&self) -> Box<dyn Shape> { Box::new(self.clone()) }
    fn scaled(&self, k: f64) -> Box<dyn Shape> { Box::new(Tri(self.0 * k, self.1 * k)) }
}

fn total_area(shapes: &[Box<dyn Shape>]) -> f64 {
    shapes.iter().map(|s| s.area()).sum()
}

fn largest<'a>(shapes: impl IntoIterator<Item = &'a dyn Shape>) -> Option<&'a dyn Shape> {
    shapes.into_iter().fold(None, |best: Option<&dyn Shape>, s| match best {
        Some(b) if b.area() >= s.area() => Some(b),
        _ => Some(s),
    })
}

fn make(kind: &str, x: f64) -> Box<dyn Shape> {
    if kind == "sq" { Box::new(Sq(x)) } else { Box::new(Tri(x, 2.0)) }
}

fn labels(items: &[impl AsRef<str>]) -> String {
    items.iter().map(|s| s.as_ref().to_uppercase()).collect::<Vec<_>>().join("+")
}

fn sum_into<T: Into<f64> + Copy>(xs: &[T]) -> f64 {
    xs.iter().map(|&x| x.into()).sum()
}

fn show_all(items: &[&dyn Display]) -> String {
    items.iter().map(|d| format!("<{}>", d)).collect::<String>()
}

fn evens(limit: u32) -> impl Iterator<Item = u32> + Clone {
    (0..limit).filter(|x| x % 2 == 0)
}

fn main() {
    let shapes: Vec<Box<dyn Shape>> = vec![make("sq", 2.0), make("tri", 3.0), Box::new(Sq(1.5))];
    println!("total {:.3}", total_area(&shapes));
    let copies = shapes.clone();
    println!("{}", copies.iter().zip(&shapes).all(|(a, b)| **a == **b));
    let doubled: Vec<Box<dyn Shape>> = shapes.iter().map(|s| s.scaled(2.0)).collect();
    for (a, b) in shapes.iter().zip(&doubled) {
        println!("{} -> {} same={}", a, b, **a == **b);
    }
    println!("{:?}", largest(shapes.iter().map(|b| b.as_ref())).map(|s| (s.name(), s.area())));
    println!("{:?}", largest(std::iter::empty()).is_none());
    let boxed: Box<dyn Shape> = Box::new(Sq(3.0));
    println!("{}", *make("sq", 3.0) == *boxed);
    println!("{}", *make("sq", 3.0) == *make("tri", 9.0));
    println!("{:?}", copies[1]);

    println!("{}", labels(&["a", "bc"]));
    println!("{}", labels(&[String::from("x"), String::from("yz")]));
    println!("{} {}", sum_into(&[1u8, 2, 3]), sum_into(&[1.5f32, 2.25]));
    let s = String::from("str");
    println!("{}", show_all(&[&1, &"two", &3.5, &'c', &s, &format!("{}", &*shapes[0])]));
    let e = evens(10);
    let e2 = e.clone().map(|x| x * x);
    println!("{:?} {:?}", e.collect::<Vec<_>>(), e2.collect::<Vec<_>>());
}
