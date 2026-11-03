//~ ERROR E0133
fn main() {
    let v = vec![1u8, 2];
    let s: &[u8] = std::slice::from_raw_parts(v.as_ptr(), 2);
    println!("{:?}", s);
}
