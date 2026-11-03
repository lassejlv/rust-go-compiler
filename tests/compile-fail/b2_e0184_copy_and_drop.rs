//~ ERROR E0184
#[derive(Clone, Copy)]
struct H(u32);
impl Drop for H { fn drop(&mut self) {} }
fn main() {}
