trait Shape {
    fn area(&self) -> f64;
    fn perimeter(&self) -> f64;
    fn name(&self) -> String;
    fn scale(&mut self, k: f64);
}

struct Circle {
    r: f64,
}

struct Rect {
    w: f64,
    h: f64,
}

struct Triangle {
    a: f64,
    b: f64,
    c: f64,
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        std::f64::consts::PI * self.r * self.r
    }
    fn perimeter(&self) -> f64 {
        2.0 * std::f64::consts::PI * self.r
    }
    fn name(&self) -> String {
        format!("Circle(r={})", self.r)
    }
    fn scale(&mut self, k: f64) {
        self.r *= k;
    }
}

impl Shape for Rect {
    fn area(&self) -> f64 {
        self.w * self.h
    }
    fn perimeter(&self) -> f64 {
        2.0 * (self.w + self.h)
    }
    fn name(&self) -> String {
        format!("Rect({}x{})", self.w, self.h)
    }
    fn scale(&mut self, k: f64) {
        self.w *= k;
        self.h *= k;
    }
}

impl Shape for Triangle {
    fn area(&self) -> f64 {
        let s = self.perimeter() / 2.0;
        (s * (s - self.a) * (s - self.b) * (s - self.c)).sqrt()
    }
    fn perimeter(&self) -> f64 {
        self.a + self.b + self.c
    }
    fn name(&self) -> String {
        format!("Triangle({}, {}, {})", self.a, self.b, self.c)
    }
    fn scale(&mut self, k: f64) {
        self.a *= k;
        self.b *= k;
        self.c *= k;
    }
}

fn make_shape(kind: &str, size: f64) -> Box<dyn Shape> {
    match kind {
        "circle" => Box::new(Circle { r: size }),
        "square" => Box::new(Rect { w: size, h: size }),
        _ => Box::new(Triangle { a: size, b: size, c: size }),
    }
}

fn main() {
    let mut shapes: Vec<Box<dyn Shape>> = Vec::new();
    shapes.push(Box::new(Circle { r: 1.0 }));
    shapes.push(Box::new(Rect { w: 3.0, h: 4.0 }));
    shapes.push(Box::new(Triangle { a: 3.0, b: 4.0, c: 5.0 }));
    shapes.push(make_shape("square", 2.0));
    shapes.push(make_shape("tri", 2.0));
    for s in shapes.iter() {
        println!("{:<22} area={:>8.3} perimeter={:>7.3}", s.name(), s.area(), s.perimeter());
    }
    let total: f64 = shapes.iter().map(|s| s.area()).sum();
    println!("total area = {:.3}", total);
    for s in shapes.iter_mut() {
        s.scale(2.0);
    }
    println!("after scaling:");
    for s in &shapes {
        println!("  {} -> {:.3}", s.name(), s.area());
    }
    shapes.sort_by(|a, b| a.area().partial_cmp(&b.area()).unwrap());
    let names: Vec<String> = shapes.iter().map(|s| s.name()).collect();
    println!("sorted: {}", names.join(", "));
    let biggest: &Box<dyn Shape> = shapes.last().unwrap();
    println!("biggest = {}", biggest.name());
    let single: Box<dyn Shape> = make_shape("circle", 0.5);
    println!("single = {} {:.4}", single.name(), single.area());
}
