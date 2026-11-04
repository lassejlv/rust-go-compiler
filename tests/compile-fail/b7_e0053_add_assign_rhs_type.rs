//~ ERROR E0053
// The impl's parameter type must match the trait's `Rhs`.
use std::ops::AddAssign;
struct Counter(u64);
impl AddAssign<i32> for Counter {
    fn add_assign(&mut self, x: u32) {
        self.0 += x as u64;
    }
}
fn main() {
    let mut c = Counter(0);
    c += 1;
}
