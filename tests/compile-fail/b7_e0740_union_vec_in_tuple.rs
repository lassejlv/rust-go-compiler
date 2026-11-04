//~ ERROR E0740
// Also inside a tuple field.
union Pair {
    a: (u8, Vec<u8>),
    b: u64,
}
fn main() {}
