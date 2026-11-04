//~ ERROR E0658
// `?` is not allowed in const fns.
const fn inc(x: Option<u8>) -> Option<u8> {
    let v = x?;
    Some(v + 1)
}
fn main() {
    println!("{:?}", inc(Some(1)));
}
