struct Circle {
    radius: f64,
}

impl Circle {
    const PI: f64 = 3.14159;
    const UNIT: Circle = Circle { radius: 1.0 };

    fn new(radius: f64) -> Circle {
        Circle { radius }
    }

    fn area(&self) -> f64 {
        Self::PI * self.radius * self.radius
    }

    fn circumference(&self) -> f64 {
        2.0 * Circle::PI * self.radius
    }
}

struct Temperature;

impl Temperature {
    const FREEZING_F: f64 = 32.0;

    fn c_to_f(c: f64) -> f64 {
        c * 9.0 / 5.0 + Self::FREEZING_F
    }

    fn f_to_c(f: f64) -> f64 {
        (f - Self::FREEZING_F) * 5.0 / 9.0
    }
}

struct IdGen {
    next: u32,
}

impl IdGen {
    const START: u32 = 1000;
    fn new() -> Self {
        IdGen { next: Self::START }
    }
    fn next_id(&mut self) -> u32 {
        let id = self.next;
        self.next += 1;
        id
    }
}

fn main() {
    let c = Circle::new(2.0);
    println!("area = {:.4}", c.area());
    println!("circumference = {:.4}", c.circumference());
    println!("unit area = {:.5}", Circle::UNIT.area());
    println!("PI = {}", Circle::PI);
    for c in [-40.0, 0.0, 37.0, 100.0] {
        println!("{}C = {}F", c, Temperature::c_to_f(c));
    }
    println!("212F = {}C", Temperature::f_to_c(212.0));
    let mut g = IdGen::new();
    for _ in 0..3 {
        println!("id = {}", g.next_id());
    }
    println!("i32::MAX via assoc = {}", i32::MAX);
    println!("u8::BITS = {}", u8::BITS);
}
