//~ ERROR E0379
trait Zero { fn zero() -> Self; }
impl Zero for u32 { const fn zero() -> u32 { 0 } }
fn main() {}
