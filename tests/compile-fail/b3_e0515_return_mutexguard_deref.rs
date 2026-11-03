//~ ERROR E0515
use std::sync::Mutex;
fn peek(m: &Mutex<Vec<i32>>) -> &Vec<i32> {
    let g = m.lock().unwrap();
    &g
}
fn main() { let m = Mutex::new(vec![]); peek(&m); }
