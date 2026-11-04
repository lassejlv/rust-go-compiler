//~ ERROR E0282
// Neither is the element type of a `Vec::new()` that is only asked if empty.
fn main() {
    if Vec::new().is_empty() {
        println!("empty");
    }
}
