//~ ERROR E0506
fn main() {
    let mut buf = vec![1u8, 2, 3];
    let head = &buf[..2];
    buf = vec![9];
    println!("{:?} {:?}", head, buf);
}
