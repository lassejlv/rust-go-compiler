//~ ERROR E0525
// `+=` on a captured variable makes the closure FnMut.
fn apply<F: Fn(i32)>(f: F) {
    f(1)
}
fn main() {
    let mut sum = 0;
    apply(|x| sum += x);
    println!("{}", sum);
}
