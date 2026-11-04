//~ ERROR E0038
// Neither is `Default`.
fn main() {
    let d: Box<dyn Default> = Box::new(0u8);
}
