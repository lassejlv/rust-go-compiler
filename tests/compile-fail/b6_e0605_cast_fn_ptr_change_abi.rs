//~ ERROR E0605
// `as` cannot change the ABI of a fn pointer.
fn rust_inc(x: i32) -> i32 { x + 1 }

fn main() {
    let p: fn(i32) -> i32 = rust_inc;
    let c = p as extern "C" fn(i32) -> i32;
    println!("{}", c(1));
}
