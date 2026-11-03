//~ ERROR E0308
fn main() {
    let v = vec![1u32, 2];
    let s = v.iter().fold(0u64, |acc, x| acc + x);
    println!("{}", s);
}
