//~ ERROR E0382
fn main() {
    let o: Option<String> = Some("v".to_string());
    let a = o.unwrap();
    let b = o.unwrap_or_default();
    println!("{} {}", a, b);
}
