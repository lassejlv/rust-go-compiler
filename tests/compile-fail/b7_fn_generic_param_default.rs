//~ ERROR defaults for generic parameters are not allowed here
// Functions cannot give their type parameters defaults.
fn make<T = i32>() -> Vec<T> {
    Vec::new()
}
fn main() {
    let _ = make::<u8>();
}
