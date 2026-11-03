//~ ERROR E0505
fn main() {
    let v = vec![1, 2, 3];
    let first = v.first();
    let s: i32 = v.into_iter().sum();
    println!("{:?} {}", first, s);
}
