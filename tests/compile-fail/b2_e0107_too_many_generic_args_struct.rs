//~ ERROR E0107
struct Pair<T> { a: T, b: T }
fn main() { let _: Pair<i32, i32> = Pair { a: 1, b: 2 }; }
