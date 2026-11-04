//~ ERROR E0308
// A closure capturing a local passed where a fn pointer is expected.
fn apply(f: fn(i32) -> i32) -> i32 {
    f(1)
}
fn main() {
    let offset = 10;
    println!("{}", apply(|x| x + offset));
}
