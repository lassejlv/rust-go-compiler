//~ ERROR E0080
// A constant cannot read a `static mut`.
static mut COUNTER: u32 = 0;
const START: u32 = unsafe { COUNTER };
fn main() {
    println!("{}", START);
}
