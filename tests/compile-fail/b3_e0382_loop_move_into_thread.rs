//~ ERROR E0382
use std::thread;
fn main() {
    let data = vec![1, 2, 3];
    let mut hs = Vec::new();
    for _ in 0..2 {
        hs.push(thread::spawn(move || data.len()));
    }
    for h in hs { h.join().unwrap(); }
}
