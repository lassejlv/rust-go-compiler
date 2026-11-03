//~ ERROR E0596
fn fill(v: Vec<i32>) -> Vec<i32> {
    v.push(1);
    v
}
fn main() { println!("{:?}", fill(vec![])); }
