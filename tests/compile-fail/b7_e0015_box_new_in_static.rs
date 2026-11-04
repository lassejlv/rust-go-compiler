//~ ERROR E0015
// `Box::new` is not a const fn.
static B: Box<i32> = Box::new(1);
fn main() {
    println!("{}", B);
}
