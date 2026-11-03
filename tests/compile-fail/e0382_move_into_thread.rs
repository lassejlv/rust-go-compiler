//~ ERROR E0382
use std::thread;

fn main() {
    let v = vec![1, 2, 3];
    let h = thread::spawn(move || v.len());
    println!("{:?}", v);
    h.join().unwrap();
}
