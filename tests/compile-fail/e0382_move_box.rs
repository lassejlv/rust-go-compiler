//~ ERROR E0382
fn main() {
    let b = Box::new(5);
    let c = b;
    println!("{} {}", b, c);
}
