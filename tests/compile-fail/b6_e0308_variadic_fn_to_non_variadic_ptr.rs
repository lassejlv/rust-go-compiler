//~ ERROR E0308
// A C-variadic function is not a fixed-arity fn pointer.
use std::ffi::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

fn main() {
    let p: unsafe extern "C" fn(*const c_char) -> c_int = printf;
    unsafe { p(c"hi\n".as_ptr()); }
}
