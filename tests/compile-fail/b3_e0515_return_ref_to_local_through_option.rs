//~ ERROR E0515
fn first_word(text: &str) -> Option<&str> {
    let lowered = text.to_lowercase();
    lowered.split(' ').next()
}
fn main() { first_word("A b"); }
