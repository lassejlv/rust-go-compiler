#![no_main]
use std::os::raw::{c_char, c_int};

#[no_mangle]
pub extern "C" fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    let name = unsafe { std::ffi::CStr::from_ptr(*argv) };
    println!("argc {} {}", argc, !name.to_bytes().is_empty());
    3
}
