//~ ERROR E0277
fn main() {
    let s: str = *"abc";
    println!("{}", s.len());
}
