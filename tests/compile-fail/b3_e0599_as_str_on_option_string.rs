//~ ERROR E0599
fn main() {
    let o: Option<String> = Some("a".into());
    let s: &str = o.as_str();
    println!("{}", s);
}
