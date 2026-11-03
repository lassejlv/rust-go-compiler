//~ ERROR E0080
const BIG: u8 = 255 + 1;

fn main() {
    println!("{}", BIG);
}
