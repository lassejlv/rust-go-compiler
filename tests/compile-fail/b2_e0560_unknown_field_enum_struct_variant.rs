//~ ERROR E0560
struct Pt { x: i32, y: i32 }
fn main() { let _ = Pt { x: 1, z: 2 }; }
