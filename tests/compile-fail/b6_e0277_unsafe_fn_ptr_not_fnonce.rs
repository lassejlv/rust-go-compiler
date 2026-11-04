//~ ERROR E0277
// `unsafe fn` pointers do not implement `FnOnce`.
fn run<F: FnOnce() -> u8>(f: F) -> u8 { f() }

unsafe fn zero() -> u8 { 0 }

fn main() {
    let p: unsafe fn() -> u8 = zero;
    println!("{}", run(p));
}
