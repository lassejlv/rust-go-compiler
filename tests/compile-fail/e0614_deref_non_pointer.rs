//~ ERROR E0614
fn main() {
    let x = 5u32;
    let y = *x;
    println!("{}", y);
}
