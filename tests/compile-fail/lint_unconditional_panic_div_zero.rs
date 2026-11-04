//~ ERROR unconditional_panic
fn ratio(n: u64) -> u64 {
    n / 0
}
fn main() {
    println!("{}", ratio(4));
}
