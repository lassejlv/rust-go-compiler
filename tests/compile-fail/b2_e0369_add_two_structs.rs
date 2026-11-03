//~ ERROR E0369
struct V2 { x: f32 }
fn main() {
    let a = V2 { x: 1.0 };
    let b = V2 { x: 2.0 };
    let _ = a + b;
}
