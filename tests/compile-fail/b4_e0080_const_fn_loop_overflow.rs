//~ ERROR E0080
const fn weight(n: u32) -> u32 {
    let mut total = 0u32;
    let mut i = 0;
    while i < n {
        total += 1_000_000_000;
        i += 1;
    }
    total
}
const W: u32 = weight(10);
fn main() { println!("{}", W); }
