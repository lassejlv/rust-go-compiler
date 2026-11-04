//~ ERROR E0070
const LIMIT: u32 = 10;
fn main() {
    LIMIT = 20;
    println!("{}", LIMIT);
}
