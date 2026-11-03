//~ ERROR E0277
use std::rc::Rc;
use std::thread;
fn main() {
    let shared = Rc::new(5);
    let h = thread::spawn(move || println!("{}", shared));
    h.join().unwrap();
}
