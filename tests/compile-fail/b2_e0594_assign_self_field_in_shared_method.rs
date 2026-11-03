//~ ERROR E0594
struct Counter { n: u32 }
impl Counter {
    fn tick(&self) { self.n += 1; }
}
fn main() { Counter { n: 0 }.tick(); }
