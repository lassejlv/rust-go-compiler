//~ ERROR E0597
use std::cell::{Cell, RefCell};
fn main() {
    let c: Cell<&str> = Cell::new("a");
    let r: RefCell<Vec<&str>> = RefCell::new(vec![]);
    let mut o: Option<&str> = None;
    let mut arr = ["a", "b"];
    {
        let s = String::from("w");
        o.replace(&s);
        let _ = std::mem::replace(&mut arr[0], &s);
    }
    println!("{} {:?} {:?} {:?}", c.get(), r, o, arr);
}
