//~ ERROR E0525
// Calling a `&mut self` method on a captured variable makes it FnMut.
fn run_twice<F>(f: F) where F: Fn() {
    f();
    f();
}
fn main() {
    let mut log = Vec::new();
    run_twice(|| log.push("x"));
    println!("{:?}", log);
}
