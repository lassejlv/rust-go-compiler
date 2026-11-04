//~ ERROR E0277
// `#[derive(Eq)]` needs every field to be `Eq`; `f32` is not.
#[derive(PartialEq, Eq)]
struct Point {
    x: f32,
    y: f32,
}
fn main() {
    println!("{}", Point { x: 1.0, y: 2.0 } == Point { x: 1.0, y: 2.0 });
}
