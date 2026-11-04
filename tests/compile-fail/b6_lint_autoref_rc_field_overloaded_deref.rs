//~ ERROR lint
// dangerous_implicit_autorefs: reading a field through `Rc`'s overloaded
// `Deref` borrows `*p`.
use std::rc::Rc;

struct Config {
    level: u8,
}

fn level(p: *const Rc<Config>) -> u8 {
    unsafe { (*p).level }
}

fn main() {
    let c = Rc::new(Config { level: 3 });
    println!("{}", level(&c));
}
