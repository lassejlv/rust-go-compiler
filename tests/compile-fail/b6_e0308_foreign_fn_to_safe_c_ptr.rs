//~ ERROR E0308
// Foreign functions are unsafe: they coerce to `unsafe extern "C" fn`
// pointers only.
use std::ffi::c_int;

extern "C" {
    fn abs(x: c_int) -> c_int;
}

fn main() {
    let f: extern "C" fn(c_int) -> c_int = abs;
    println!("{}", f(-3));
}
