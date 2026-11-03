use std::cell::RefCell;
use std::sync::Mutex;

struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
impl std::fmt::Display for D {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "D({})", self.0)
    }
}

fn f() -> String {
    let m = Mutex::new(1);
    format!("{}", m.lock().unwrap())
}

fn g() -> String {
    let c = RefCell::new(vec![1, 2]);
    format!("{:?} {}", c.borrow(), D("arg temp"))
}

fn h() -> String {
    let _l = D("h local");
    format!("{}", D("fmt temp"))
}

fn main() {
    println!("{}", f());
    println!("{}", g());
    println!("{}", h());
}
