//~ ERROR E0277
use std::thread;
fn run_bg<T: 'static + std::fmt::Debug>(t: T) {
    thread::spawn(move || println!("{:?}", t)).join().unwrap();
}
fn main() { run_bg(1); }
