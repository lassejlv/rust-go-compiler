//~ ERROR E0502
fn main() {
    let mut tokens = vec!["a", "b"];
    let mut it = tokens.iter().peekable();
    if let Some(t) = it.peek() {
        tokens.push(t);
    }
    it.next();
}
