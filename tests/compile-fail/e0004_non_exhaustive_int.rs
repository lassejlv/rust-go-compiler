//~ ERROR E0004
fn main() {
    let n: u8 = 3;
    match n {
        0..=100 => println!("low"),
        200..=255 => println!("high"),
    }
}
