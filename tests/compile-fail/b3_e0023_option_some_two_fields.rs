//~ ERROR E0023
fn main() {
    let o = Some((1, 2));
    if let Some(a, b) = o { println!("{} {}", a, b); }
}
