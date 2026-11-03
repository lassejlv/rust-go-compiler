//~ ERROR E0505
fn main() {
    let s = String::from("hello");
    let r = &s;
    let mut v = Vec::new();
    v.push(s);
    println!("{} {}", r, v.len());
}
