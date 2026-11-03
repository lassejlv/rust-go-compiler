//~ ERROR E0599
fn main() {
    let mut s = String::new();
    s.write_str("hi").unwrap();
    println!("{}", s);
}
