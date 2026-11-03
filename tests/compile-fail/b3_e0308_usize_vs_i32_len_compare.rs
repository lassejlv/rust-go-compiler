//~ ERROR E0308
fn main() {
    let v = vec![1];
    let limit: i32 = 3;
    if v.len() < limit { println!("small"); }
}
