//~ ERROR E0507
use std::rc::Rc;

fn main() {
    let r = Rc::new(String::from("shared"));
    let s: String = *r;
    println!("{}", s);
}
