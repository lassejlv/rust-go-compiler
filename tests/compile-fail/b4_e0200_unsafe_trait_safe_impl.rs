//~ ERROR E0200
unsafe trait Zeroable {}
struct Pod(u32);
impl Zeroable for Pod {}
fn main() {}
