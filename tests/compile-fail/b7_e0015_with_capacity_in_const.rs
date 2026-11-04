//~ ERROR E0015
// `Vec::with_capacity` is not const.
const BUF: Vec<u8> = Vec::with_capacity(64);
fn main() {
    println!("{}", BUF.capacity());
}
