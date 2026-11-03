//~ ERROR E0382
use std::thread;
fn main() {
    let data = vec![1, 2, 3];
    let h = thread::spawn(move || data.iter().sum::<i32>());
    println!("{}", data.len());
    h.join().unwrap();
}
