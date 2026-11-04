//~ ERROR literal out of range
// The expectation of a cast reaches the literal through parentheses.
fn main() {
    let x = (-(300)) as i8;
    println!("{}", x);
}
