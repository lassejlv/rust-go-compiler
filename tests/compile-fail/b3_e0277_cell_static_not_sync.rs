//~ ERROR E0277
use std::cell::Cell;
static COUNTER: Cell<u32> = Cell::new(0);
fn main() { COUNTER.set(1); }
