//~ ERROR E0599
fn main() {
    let o: Option<String> = Some("a".into());
    println!("{}", o.push_str("x"));
}
