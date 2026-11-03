//~ ERROR E0308
fn find(x: Option<&str>) -> bool { x.is_some() }
fn main() { let s = String::from("a"); find(Some(s)); }
