//~ ERROR E0592
struct S;
impl S {
    fn a(&self) {}
    fn a(&self, x: i32) {}
}
fn main() {}
