//~ ERROR E0365
mod foo { pub const X: u32 = 1; }
pub use foo as foo2;
fn main() {}
