//~ ERROR E0030
fn classify(n: u8) -> &'static str {
    match n {
        0..=9 => "digit",
        200..=100 => "never",
        _ => "other",
    }
}
fn main() { println!("{}", classify(3)); }
