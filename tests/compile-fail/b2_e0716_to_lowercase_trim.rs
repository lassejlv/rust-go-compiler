//~ ERROR E0716
fn main() {
    let input = "  HeLLo ";
    let t = input.to_lowercase().trim();
    println!("{}", t);
}
