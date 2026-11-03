//~ ERROR E0277
use std::thread;
trait Task { fn run(&self); }
struct T1;
impl Task for T1 { fn run(&self) {} }
fn main() {
    let t: Box<dyn Task> = Box::new(T1);
    thread::spawn(move || t.run()).join().unwrap();
}
