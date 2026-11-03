//~ ERROR E0503
fn main() {
    let mut x = 10;
    let r = &mut x;
    let y = x + 1;
    *r += 1;
    println!("{}", y);
}
