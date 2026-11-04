//~ ERROR E0716
// the `&mut` autoref of a method receiver borrows a temporary (not promoted)
fn main() {
    let mut it = [1, 2, 3].iter_mut();
    *it.next().unwrap() += 1;
}
