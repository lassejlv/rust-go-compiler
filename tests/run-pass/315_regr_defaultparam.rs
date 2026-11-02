use std::collections::HashMap;
fn main() {
    let n = [(1, 2)].into_iter().collect::<HashMap<i32, i32>>();
    println!("{}", n.len());
}
