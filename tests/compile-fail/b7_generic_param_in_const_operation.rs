//~ ERROR generic parameters may not be used in const operations
// `N + 1` in a repeat count.
fn grow<const N: usize>() -> usize {
    let a = [0u8; N + 1];
    a.len()
}
fn main() {
    println!("{}", grow::<2>());
}
