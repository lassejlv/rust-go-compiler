//~ ERROR E0503
fn main() {
    let mut count = 0;
    let r = &mut count;
    let doubled = count * 2;
    *r += 1;
    println!("{}", doubled);
}
