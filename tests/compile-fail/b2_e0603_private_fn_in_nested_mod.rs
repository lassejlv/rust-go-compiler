//~ ERROR E0603
mod a { pub mod b { fn secret() {} } }
fn main() { a::b::secret(); }
