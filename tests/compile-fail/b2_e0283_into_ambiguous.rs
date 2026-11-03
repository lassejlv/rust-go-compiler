//~ ERROR E0283
fn main() {
    let x: u32 = 5;
    let y = x.into();
    println!("{}", y);
}
