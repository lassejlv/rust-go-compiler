//~ ERROR E0373
use std::thread;
fn main() {
    let mut hs = vec![];
    for i in 0..3 {
        hs.push(thread::spawn(|| println!("{}", i)));
    }
    for h in hs { h.join().unwrap(); }
}
