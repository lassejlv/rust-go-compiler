//~ ERROR E0507
use std::sync::Mutex;
fn main() {
    let m = Mutex::new(String::from("x"));
    let s: String = *m.lock().unwrap();
    println!("{}", s);
}
