//~ ERROR E0599
fn show<T>(t: T) -> String { t.to_string() }
fn main() { show(1); }
