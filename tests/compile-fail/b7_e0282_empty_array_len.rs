//~ ERROR E0282
// The element type of `[]` is never inferred.
fn main() {
    let n = [].len();
    println!("{}", n);
}
