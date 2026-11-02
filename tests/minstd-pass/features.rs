struct Point { x: i32, y: i32 }
impl Point {
    fn new(x: i32, y: i32) -> Point { Point { x, y } }
    fn sum(&self) -> i32 { self.x + self.y }
    fn scale(&mut self, k: i32) { self.x = self.x * k; self.y *= k; }
}
enum Shape { Circle(i32), Rect { w: i32, h: i32 }, Empty }
fn area(s: &Shape) -> i32 {
    match s {
        Shape::Circle(r) => 3 * *r * *r,
        Shape::Rect { w, h } => *w * *h,
        Shape::Empty => 0,
    }
}
trait Animal { fn legs(&self) -> i32; fn describe(&self) -> i32 { self.legs() * 10 } }
struct Dog; struct Bird;
impl Animal for Dog { fn legs(&self) -> i32 { 4 } }
impl Animal for Bird { fn legs(&self) -> i32 { 2 } fn describe(&self) -> i32 { 99 } }
fn total<T: Animal>(a: &T) -> i32 { a.describe() }
fn apply<F: Fn(i32) -> i32>(f: F, v: i32) -> i32 { f(v) }
fn fact(n: i32) -> i32 { if n <= 1 { 1 } else { n * fact(n - 1) } }
fn main() {
    let mut p = Point::new(3, 4);
    println!("sum {}", p.sum());
    p.scale(2);
    println!("scaled {} {}", p.x, p.y);
    let shapes = [Shape::Circle(2), Shape::Rect { w: 3, h: 5 }, Shape::Empty];
    let mut i = 0;
    while i < 3 { println!("area {}", area(&shapes[i])); i += 1; }
    println!("dog {} bird {}", total(&Dog), total(&Bird));
    let k = 7;
    println!("closure {}", apply(|x| x * k + 1, 5));
    println!("fact {}", fact(10));
    let t = (1, 2, 3);
    let (a, b, c) = t;
    println!("tuple {}", a + b + c);
    let opt = Some(5);
    if let Some(v) = opt { println!("some {}", v); }
    let x = match opt { Some(n) if n > 3 => n * 2, Some(n) => n, None => -1 };
    println!("x {}", x);
    let v = loop { if i > 10 { break i * 2; } i += 1; };
    println!("loop {}", v as i32);
    let a: &dyn Animal = &Dog;
    println!("dyn {}", a.describe());
    let mut counter = 0;
    let mut inc = || counter += 1;
    inc(); inc();
    println!("counter {}", counter);
    for j in 0..3 { if j == 1 { continue; } println!("j {}", j); }
    let s = String::new();
    let mut s2 = s;
    s2.push_str("abc");
    println!("s2 {}", s2.as_str());
}
