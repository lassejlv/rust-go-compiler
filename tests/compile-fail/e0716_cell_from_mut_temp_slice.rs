//~ ERROR E0716
use std::cell::Cell;
fn main() {
    let c = Cell::from_mut(&mut [1, 2, 3][..]).as_slice_of_cells();
    c[0].set(100);
}
