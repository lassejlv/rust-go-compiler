//~ ERROR E0027
struct P { x: i32, y: i32 }
fn f(p: P) -> i32 { let P { x } = p; x }
fn main() { f(P { x: 1, y: 2 }); }
