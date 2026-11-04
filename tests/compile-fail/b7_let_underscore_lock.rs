//~ ERROR non-binding let on a synchronization lock
// The guard of `let _ = m.lock();` is dropped at once.
use std::sync::Mutex;
fn main() {
    let m = Mutex::new(0);
    let _ = m.lock();
    println!("{:?}", m);
}
