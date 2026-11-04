//~ ERROR E0307
struct Counter { n: u32 }
impl Counter {
    fn get(self: u32) -> u32 { self }
}
fn main() {}
