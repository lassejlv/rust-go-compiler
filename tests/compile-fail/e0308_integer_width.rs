//~ ERROR E0308
fn main() {
    let a: u8 = 1;
    let b: u32 = a;
    println!("{}", b);
}
