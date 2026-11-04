//~ ERROR E0015
// Neither is `vec![x; n]` in a const.
const ZEROS: Vec<u8> = vec![0; 16];
fn main() {
    println!("{:?}", ZEROS);
}
