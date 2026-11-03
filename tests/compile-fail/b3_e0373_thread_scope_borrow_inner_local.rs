//~ ERROR E0373
use std::thread;
fn main() {
    let mut results = Vec::new();
    thread::scope(|s| {
        let local = vec![1, 2];
        s.spawn(|| results.push(local.len()));
    });
    let _ = &results;
}
