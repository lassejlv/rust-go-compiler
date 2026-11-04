//~ ERROR E0392
// A type parameter used only in a (non-binding) bound is unused.
struct Converter<T, U: AsRef<T>> {
    source: U,
}
fn main() {}
