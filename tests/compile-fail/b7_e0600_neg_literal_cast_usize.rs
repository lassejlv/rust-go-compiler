//~ ERROR E0600
// `-1 as usize` negates a `usize` literal.
fn main() {
    let v = vec![1, 2, 3];
    let last = v.len() + -1 as usize;
    println!("{}", last);
}
