//~ ERROR is forbidden as the type of a const generic parameter
// `f32` cannot be the type of a const parameter.
fn scale<const F: f32>(x: f32) -> f32 {
    x * F
}
fn main() {}
