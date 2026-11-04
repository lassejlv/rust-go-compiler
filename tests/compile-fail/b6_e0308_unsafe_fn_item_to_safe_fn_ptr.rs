//~ ERROR E0308
// An unsafe fn item does not coerce to a safe fn pointer.
unsafe fn dangerous(x: i32) -> i32 { x }

fn main() {
    let f: fn(i32) -> i32 = dangerous;
    println!("{}", f(1));
}
