//~ ERROR E0277
use std::sync::{Mutex, MutexGuard};
fn needs_send<T: Send>(_t: T) {}
fn main() {
    let m = Mutex::new(1);
    let g: MutexGuard<i32> = m.lock().unwrap();
    needs_send(g);
}
