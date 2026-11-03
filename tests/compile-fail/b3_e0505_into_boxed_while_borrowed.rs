//~ ERROR E0505
fn main() {
    let v = vec![1u8, 2, 3];
    let head = &v[0];
    let b: Box<[u8]> = v.into_boxed_slice();
    println!("{} {:?}", head, b);
}
