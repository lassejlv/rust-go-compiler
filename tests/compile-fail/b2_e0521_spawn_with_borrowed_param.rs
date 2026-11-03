//~ ERROR E0521
use std::thread;
fn run(v: &Vec<i32>) {
    thread::spawn(move || println!("{:?}", v)).join().unwrap();
}
fn main() { run(&vec![1]); }
