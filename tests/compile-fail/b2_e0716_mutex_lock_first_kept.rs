//~ ERROR E0716
use std::sync::Mutex;
fn main() {
    let m = Mutex::new(vec![1]);
    let first: &i32 = m.lock().unwrap().first().unwrap();
    println!("{}", first);
}
