//~ ERROR E0382
fn main() {
    let s = String::from("abc");
    let f = move || s.len();
    println!("{}", f());
    println!("{}", s);
}
