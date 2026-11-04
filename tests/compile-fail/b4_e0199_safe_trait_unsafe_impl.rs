//~ ERROR E0199
trait Plain {}
struct Pod(u32);
unsafe impl Plain for Pod {}
fn main() {}
