use std::fmt::Debug;

trait Shape {
    fn area(&self) -> f64;
    fn name(&self) -> String;

    // Generic method: not object-safe, so excluded from the vtable.
    fn scaled_area<T: Into<f64>>(&self, factor: T) -> f64
    where
        Self: Sized,
    {
        self.area() * factor.into()
    }

    // Returns Self: also excluded.
    fn duplicate(&self) -> Self
    where
        Self: Sized + Clone,
    {
        self.clone()
    }

    fn describe(&self) -> String {
        format!("{} with area {:.2}", self.name(), self.area())
    }
}

#[derive(Clone, Debug)]
struct Circle {
    r: f64,
}

#[derive(Clone, Debug)]
struct Rect {
    w: f64,
    h: f64,
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        3.0 * self.r * self.r
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
        format!("rect {}x{}", self.w, self.h)
    }
    fn describe(&self) -> String {
        format!("[{}]", self.name())
    }
}

trait Counter {
    fn bump(&mut self) -> u32;
    fn get(&self) -> u32;
}

#[derive(Debug)]
struct Simple(u32);
#[derive(Debug)]
struct Stepped {
    v: u32,
    step: u32,
}

impl Counter for Simple {
    fn bump(&mut self) -> u32 {
        self.0 += 1;
        self.0
    }
    fn get(&self) -> u32 {
        self.0
    }
}

impl Counter for Stepped {
    fn bump(&mut self) -> u32 {
        self.v += self.step;
        self.v
    }
    fn get(&self) -> u32 {
        self.v
    }
}

fn bump_n(c: &mut dyn Counter, n: u32) -> u32 {
    for _ in 0..n {
        c.bump();
    }
    c.get()
}

fn show<T: Debug>(x: &T) {
    println!("show: {:?}", x);
}

fn main() {
    let c = Circle { r: 2.0 };
    let r = Rect { w: 3.0, h: 4.5 };
    println!("{}", c.scaled_area(2));
    println!("{}", r.scaled_area(0.5f32));
    let c2 = c.duplicate();
    show(&c2);

    let shapes: Vec<Box<dyn Shape>> = vec![Box::new(c), Box::new(r), Box::new(Circle { r: 0.5 })];
    for s in &shapes {
        println!("{}", s.describe());
    }
    let total: f64 = shapes.iter().map(|s| s.area()).sum();
    println!("total area {:.3}", total);

    let mut a = Simple(0);
    let mut b = Stepped { v: 10, step: 5 };
    println!("a -> {}", bump_n(&mut a, 3));
    println!("b -> {}", bump_n(&mut b, 3));
    {
        let counters: [&mut dyn Counter; 2] = [&mut a, &mut b];
        for c in counters {
            let v = c.bump();
            println!("bumped to {}", v);
        }
    }
    show(&a);
    show(&b);
    let mut boxed: Vec<Box<dyn Counter>> = vec![Box::new(Simple(100)), Box::new(Stepped { v: 0, step: 7 })];
    for c in boxed.iter_mut() {
        let r: &mut dyn Counter = c.as_mut();
        println!("boxed -> {}", bump_n(r, 2));
    }
}
