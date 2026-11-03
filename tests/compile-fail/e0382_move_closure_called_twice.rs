//~ ERROR E0382
fn main() {
    let s = String::from("once");
    let f = move || s;
    let a = f();
    let b = f();
    println!("{} {}", a, b);
}
