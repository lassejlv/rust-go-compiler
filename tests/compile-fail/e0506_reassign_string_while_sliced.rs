//~ ERROR E0506
fn main() {
    let mut s = String::from("abc");
    let part = &s[0..1];
    s = String::from("xyz");
    println!("{} {}", part, s);
}
