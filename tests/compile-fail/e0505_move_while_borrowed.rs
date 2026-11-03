//~ ERROR E0505
fn main() {
    let s = String::from("x");
    let r = &s;
    let t = s;
    println!("{} {}", r, t);
}
