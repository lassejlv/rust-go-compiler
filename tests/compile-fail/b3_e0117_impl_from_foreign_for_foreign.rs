//~ ERROR E0117
impl From<String> for Vec<u32> { fn from(_s: String) -> Self { Vec::new() } }
fn main() {}
