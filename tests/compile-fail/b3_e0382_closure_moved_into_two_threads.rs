//~ ERROR E0382
use std::thread;
fn main() {
    let name = String::from("w");
    let f = move || println!("{}", name);
    let a = thread::spawn(f);
    let b = thread::spawn(f);
    a.join().unwrap(); b.join().unwrap();
}
