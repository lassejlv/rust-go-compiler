//~ ERROR E0382
use std::sync::mpsc;
fn main() {
    let (tx, rx) = mpsc::channel();
    let msg = String::from("hello");
    tx.send(msg).unwrap();
    println!("{}", msg);
    let _ = rx.recv();
}
