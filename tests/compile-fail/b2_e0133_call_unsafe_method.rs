//~ ERROR E0133
struct R;
impl R { unsafe fn raw(&self) -> i32 { 1 } }
fn main() { println!("{}", R.raw()); }
