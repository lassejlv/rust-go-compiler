fn make_adder(n: i32) -> impl Fn(i32) -> i32 { move |x| x + n }
fn compose<F: Fn(i32) -> i32, G: Fn(i32) -> i32>(f: F, g: G) -> impl Fn(i32) -> i32 { move |x| g(f(x)) }
struct Counter { n: i32 }
impl Iterator for Counter {
    type Item = i32;
    fn next(&mut self) -> Option<i32> { if self.n < 5 { self.n += 1; Some(self.n) } else { None } }
}
trait Container { type Item; fn get(&self, i: i32) -> Self::Item; fn first(&self) -> Self::Item { self.get(0) } }
struct Squares;
impl Container for Squares { type Item = i32; fn get(&self, i: i32) -> i32 { i * i } }
fn show<C: Container<Item = i32>>(c: &C) -> i32 { c.get(3) + c.first() }
fn call_twice<F: FnMut()>(mut f: F) { f(); f(); }
fn consume_once<F: FnOnce() -> String>(f: F) -> String { f() }
fn main() {
    let add5 = make_adder(5);
    println!("{}", add5(10));
    let h = compose(make_adder(1), |x| x * 3);
    println!("{}", h(4));
    let mut total = 0;
    for v in (Counter { n: 0 }) { total += v; }
    println!("total {}", total);
    println!("show {}", show(&Squares));
    let mut hits = 0;
    call_twice(|| hits += 10);
    println!("hits {}", hits);
    let s = String::from("moved");
    let r = consume_once(move || s);
    println!("{}", r.as_str());
    let nested = |a: i32| { let inner = |b: i32| a * b; inner(a + 1) };
    println!("nested {}", nested(4));
    let fp: fn(i32) -> i32 = |x| x - 1;
    println!("fnptr {}", fp(10));
    let mut c = Counter { n: 2 };
    while let Some(x) = c.next() { println!("w {}", x); }
    let label = 'outer: loop { loop { break 'outer 42; } };
    println!("label {}", label);
}
