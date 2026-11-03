//~ ERROR E0505
fn main() {
    let s = String::from("abc");
    let r = &s;
    let f = move || s.len();
    println!("{} {}", r, f());
}
