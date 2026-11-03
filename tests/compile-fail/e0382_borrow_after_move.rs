//~ ERROR E0382
fn main() {
    let s = String::from("x");
    let t = s;
    let r = &s;
    println!("{} {}", t, r);
}
