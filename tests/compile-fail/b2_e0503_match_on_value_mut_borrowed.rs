//~ ERROR E0503
fn main() {
    let mut n = 3;
    let r = &mut n;
    match n { 3 => println!("three"), _ => {} }
    *r = 4;
}
