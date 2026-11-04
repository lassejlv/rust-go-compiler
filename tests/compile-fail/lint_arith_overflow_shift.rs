//~ ERROR arithmetic_overflow
fn shift(x: u32) -> u32 {
    x << 32
}
fn main() {
    println!("{}", shift(1));
}
