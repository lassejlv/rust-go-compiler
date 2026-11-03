//~ ERROR E0277
fn show<T>(items: &[T]) -> String {
    items.iter().map(|i| format!("{}", i)).collect()
}
fn main() { show(&[1]); }
