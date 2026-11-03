//~ ERROR E0502
use std::collections::VecDeque;
fn main() {
    let mut q: VecDeque<String> = VecDeque::new();
    q.push_back("x".into());
    let head = q.front().unwrap();
    q.push_back(head.clone());
    println!("{}", head);
}
