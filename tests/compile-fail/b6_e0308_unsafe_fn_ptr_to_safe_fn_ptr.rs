//~ ERROR E0308
// `unsafe fn` pointers do not coerce back to safe fn pointers.
fn double(x: i32) -> i32 { x * 2 }

fn main() {
    let u: unsafe fn(i32) -> i32 = double;
    let s: fn(i32) -> i32 = u;
    println!("{}", s(2));
}
