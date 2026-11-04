//~ ERROR E0449
// Not on inherent impls either.
pub struct Counter(u32);
pub impl Counter {
    pub fn get(&self) -> u32 {
        self.0
    }
}
fn main() {
    println!("{}", Counter(1).get());
}
