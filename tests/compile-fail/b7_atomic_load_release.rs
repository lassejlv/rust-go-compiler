//~ ERROR atomic loads cannot have `Release` or `AcqRel` ordering
// invalid_atomic_ordering is deny-by-default.
use std::sync::atomic::{AtomicUsize, Ordering};
fn main() {
    let a = AtomicUsize::new(0);
    println!("{}", a.load(Ordering::Release));
}
