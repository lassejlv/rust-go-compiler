//~ ERROR E0502
fn main() {
    let mut s = String::from("x");
    let m = &mut s;
    let r = &s;
    m.push('y');
    println!("{}", r);
}
