//~ ERROR E0046
struct C;
impl Iterator for C { type Item = u32; }
fn main() {}
