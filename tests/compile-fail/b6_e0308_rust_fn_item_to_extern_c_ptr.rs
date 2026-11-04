//~ ERROR E0308
// A Rust-ABI fn item does not coerce to an `extern "C"` fn pointer.
fn callback(x: i32) -> i32 { x + 1 }

fn main() {
    let cb: extern "C" fn(i32) -> i32 = callback;
    println!("{}", cb(1));
}
