//~ ERROR E0605
// `as` cannot make an unsafe fn safe.
unsafe fn peek(p: *const i32) -> i32 { *p }

fn main() {
    let f = peek as fn(*const i32) -> i32;
    let x = 3;
    println!("{}", f(&x));
}
