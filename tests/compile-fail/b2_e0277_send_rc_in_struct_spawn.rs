//~ ERROR E0277
use std::rc::Rc;
struct Job { data: Rc<Vec<i32>> }
fn main() {
    let j = Job { data: Rc::new(vec![1]) };
    std::thread::spawn(move || println!("{}", j.data.len()));
}
