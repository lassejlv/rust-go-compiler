//~ ERROR E0505
use std::sync::mpsc;
fn main() {
    let (tx, rx) = mpsc::channel();
    let msg = String::from("hi");
    let view = msg.as_str();
    tx.send(msg).unwrap();
    println!("{} {}", view, rx.recv().unwrap());
}
