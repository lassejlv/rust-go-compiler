//~ ERROR E0277
use std::rc::Rc;
use std::thread;

fn main() {
    let r = Rc::new(5);
    let h = thread::spawn(move || println!("{}", r));
    h.join().unwrap();
}
