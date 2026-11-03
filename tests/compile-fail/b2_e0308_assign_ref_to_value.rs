//~ ERROR E0308
fn main() {
    let mut n: i32 = 0;
    let v = vec![1, 2];
    for x in v.iter() { n = x; }
}
