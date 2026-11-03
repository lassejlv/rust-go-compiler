//~ ERROR E0594
static LIMIT: u32 = 10;
fn main() {
    LIMIT = 20;
    println!("{}", LIMIT);
}
