//~ ERROR E0740
// Union fields must be Copy (or ManuallyDrop).
union Data {
    n: u32,
    s: String,
}
fn main() {}
