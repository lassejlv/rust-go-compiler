//~ ERROR atomic stores cannot have `Acquire` or `AcqRel` ordering
// A store cannot acquire.
use std::sync::atomic::{AtomicBool, Ordering};
fn main() {
    let flag = AtomicBool::new(false);
    flag.store(true, Ordering::AcqRel);
}
