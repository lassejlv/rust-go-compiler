struct Noisy { id: i32 }
impl Drop for Noisy { fn drop(&mut self) { println!("drop {}", self.id); } }
struct Pair<T> { a: T, b: T }
impl<T> Pair<T> { fn first(self) -> T { self.a } }
fn make(id: i32) -> Noisy { Noisy { id } }
fn consume(n: Noisy) { println!("consume {}", n.id); }
enum E { A(Noisy), B }
fn main() {
    let _a = make(1);
    let b = make(2);
    consume(b);
    {
        let _c = make(3);
        println!("inner");
    }
    let _ = make(4);
    let p = Pair { a: make(5), b: make(6) };
    let f = p.first();
    println!("first {}", f.id);
    let mut x = make(7);
    x = make(8);
    println!("reassigned {}", x.id);
    let e = E::A(make(9));
    match e { E::A(n) => println!("got {}", n.id), E::B => {} }
    let cond = true;
    let m = make(10);
    if cond { consume(m); }
    println!("end");
}
