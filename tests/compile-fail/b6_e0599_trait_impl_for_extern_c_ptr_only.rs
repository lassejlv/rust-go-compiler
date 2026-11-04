//~ ERROR E0599
// A trait implemented for `extern "C" fn(i32)` is not implemented for
// `fn(i32)`.
trait Describe {
    fn describe(&self) -> &'static str;
}

impl Describe for extern "C" fn(i32) {
    fn describe(&self) -> &'static str { "C callback" }
}

fn rust_cb(_x: i32) {}

fn main() {
    let p: fn(i32) = rust_cb;
    println!("{}", p.describe());
}
