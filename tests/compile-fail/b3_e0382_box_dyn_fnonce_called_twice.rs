//~ ERROR E0382
fn main() {
    let f: Box<dyn FnOnce() -> String> = Box::new(|| String::from("x"));
    let a = f();
    let b = f();
    println!("{} {}", a, b);
}
