//~ ERROR E0539
// `#[export_name]` needs a string value.
#[export_name = 1]
pub fn f() {}
fn main() {}
