//~ ERROR E0277
use std::thread;
struct Handle { p: *mut u8 }
fn main() {
    let mut x = 0u8;
    let h = Handle { p: &mut x };
    thread::spawn(move || { let q = h.p; println!("{:?}", q); }).join().unwrap();
}
