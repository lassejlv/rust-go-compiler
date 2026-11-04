//~ ERROR E0562
fn make() -> fn() -> impl Clone { todo!() }
fn main() {}
