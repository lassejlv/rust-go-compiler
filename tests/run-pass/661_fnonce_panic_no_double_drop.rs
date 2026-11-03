struct D(&'static str);
impl Drop for D { fn drop(&mut self) { println!("drop {}", self.0); } }
fn take(d: D) { println!("took {}", d.0); }
fn call<F: FnOnce()>(f: F) { f() }
fn callbox(f: Box<dyn FnOnce()>) { f() }
fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let d = D("a");
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || { take(d); panic!("x"); }));
    println!("1 {}", r.is_err());
    let d = D("b");
    let r = std::panic::catch_unwind(move || call(move || { take(d); panic!("x"); }));
    println!("2 {}", r.is_err());
    let d = D("c");
    let r = std::panic::catch_unwind(move || callbox(Box::new(move || { take(d); panic!("x"); })));
    println!("3 {}", r.is_err());
}
