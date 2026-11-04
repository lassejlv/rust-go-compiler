//~ ERROR literal out of range
// The cast target types the literal: `-129 as i8` is an `i8` literal.
fn main() {
    let x = -129 as i8;
    println!("{}", x);
}
