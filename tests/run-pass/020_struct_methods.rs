struct Rectangle {
    width: u32,
    height: u32,
}

impl Rectangle {
    fn new(width: u32, height: u32) -> Rectangle {
        Rectangle { width, height }
    }

    fn square(size: u32) -> Self {
        Self { width: size, height: size }
    }

    fn area(&self) -> u32 {
        self.width * self.height
    }

    fn perimeter(&self) -> u32 {
        2 * (self.width + self.height)
    }

    fn can_hold(&self, other: &Rectangle) -> bool {
        self.width > other.width && self.height > other.height
    }

    fn scale(&mut self, factor: u32) {
        self.width *= factor;
        self.height *= factor;
    }

    fn into_tuple(self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn is_square(&self) -> bool {
        self.width == self.height
    }
}

struct Counter {
    count: i32,
}

impl Counter {
    fn increment(&mut self) -> &mut Self {
        self.count += 1;
        self
    }
    fn get(&self) -> i32 {
        self.count
    }
}

fn main() {
    let mut r = Rectangle::new(4, 7);
    println!("area = {}, perimeter = {}", r.area(), r.perimeter());
    let s = Rectangle::square(3);
    println!("square area = {}, is_square = {}", s.area(), s.is_square());
    println!("r can hold s: {}", r.can_hold(&s));
    println!("s can hold r: {}", s.can_hold(&r));
    r.scale(2);
    println!("scaled: {}x{} area = {}", r.width, r.height, r.area());
    let (w, h) = r.into_tuple();
    println!("tuple = ({}, {})", w, h);

    let mut c = Counter { count: 0 };
    c.increment().increment().increment();
    println!("counter = {}", c.get());
    let rr = &mut c;
    rr.increment();
    println!("counter = {}", c.get());
    println!("ufcs area = {}", Rectangle::area(&s));
}
