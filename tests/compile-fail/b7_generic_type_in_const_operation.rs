//~ ERROR generic parameters may not be used in const operations
// `size_of::<T>()` as an array length.
fn bytes<T>(_x: T) -> [u8; std::mem::size_of::<T>()] {
    todo!()
}
fn main() {}
