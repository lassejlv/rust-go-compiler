//~ ERROR E0133
// Calling the unsafe fn pointer returned by a function.
fn get() -> unsafe fn() -> i32 {
    fn f() -> i32 { 1 }
    f
}

fn main() {
    println!("{}", get()());
}
