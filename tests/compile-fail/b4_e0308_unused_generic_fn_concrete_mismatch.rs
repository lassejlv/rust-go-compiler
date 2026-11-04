//~ ERROR E0308
// Generic bodies are checked even when never instantiated.
fn count<T>(items: &[T]) -> usize {
    if items.is_empty() {
        return "none";
    }
    items.len()
}
fn main() {}
