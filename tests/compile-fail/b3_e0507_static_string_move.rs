//~ ERROR E0507
static NAMES: Vec<String> = Vec::new();
fn main() { let v = NAMES; println!("{:?}", v); }
