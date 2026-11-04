//~ ERROR memory fences cannot have `Relaxed` ordering
// A relaxed fence orders nothing.
use std::sync::atomic::{fence, Ordering};
fn main() {
    fence(Ordering::Relaxed);
}
