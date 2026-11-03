//~ ERROR E0382
fn main() {
    let mut out = Vec::new();
    let item = String::from("x");
    for _ in 0..3 {
        out.push(item);
    }
    println!("{}", out.len());
}
