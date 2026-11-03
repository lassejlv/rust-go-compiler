//~ ERROR E0599
enum Opt<T> { Some(T), Nothing }
fn main() { let _: Opt<i32> = Opt::None; }
