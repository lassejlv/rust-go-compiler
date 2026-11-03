//~ ERROR E0502
fn main() {
    let mut v = vec![3, 1, 2];
    let w: Vec<&[i32]> = v.windows(2).collect();
    v.sort();
    println!("{:?}", w);
}
