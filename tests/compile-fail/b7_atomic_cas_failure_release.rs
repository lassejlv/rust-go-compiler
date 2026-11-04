//~ ERROR failure ordering may not be `Release` or `AcqRel`
// A failed compare_exchange does not write.
use std::sync::atomic::{AtomicI32, Ordering};
fn main() {
    let a = AtomicI32::new(0);
    let _ = a.compare_exchange(0, 1, Ordering::SeqCst, Ordering::Release);
}
