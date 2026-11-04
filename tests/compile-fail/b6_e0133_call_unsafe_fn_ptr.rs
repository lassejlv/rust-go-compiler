//~ ERROR E0133
// Calling an `unsafe fn` pointer needs an unsafe block.
fn triple(x: i32) -> i32 { x * 3 }

fn main() {
    let p: unsafe fn(i32) -> i32 = triple;
    println!("{}", p(2));
}
