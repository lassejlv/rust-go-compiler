//~ ERROR E0606
fn main() {
    let x = &0u8;
    let y: u32 = x as u32;
    println!("{}", y);
}
