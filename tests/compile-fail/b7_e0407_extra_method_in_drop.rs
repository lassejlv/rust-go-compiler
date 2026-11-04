//~ ERROR E0407
// `Drop` has only `drop`.
struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {}
    fn release(&mut self) {}
}
fn main() {
    let _g = Guard;
}
