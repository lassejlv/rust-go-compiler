//~ ERROR E0425
struct C { n: i32 }
impl C {
    fn double(&self) -> i32 { self.n * 2 }
    fn quad(&self) -> i32 { double() * 2 }
}
fn main() { C { n: 1 }.quad(); }
