//~ ERROR E0308
// Non-capturing closures only coerce to Rust-ABI fn pointers.
fn main() {
    let cb: extern "C" fn(i32) -> i32 = |x| x + 1;
    println!("{}", cb(1));
}
