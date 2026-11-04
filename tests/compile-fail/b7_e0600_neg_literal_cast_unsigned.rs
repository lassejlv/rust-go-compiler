//~ ERROR E0600
// `-1 as u32`: the literal is a `u32`, which cannot be negated.
fn main() {
    let mask = -1 as u32;
    println!("{:x}", mask);
}
