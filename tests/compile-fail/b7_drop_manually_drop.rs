//~ ERROR calls to `std::mem::drop` with `std::mem::ManuallyDrop`
// Dropping a ManuallyDrop does nothing.
use std::mem::ManuallyDrop;
fn main() {
    let s = ManuallyDrop::new(String::from("leak"));
    drop(s);
}
