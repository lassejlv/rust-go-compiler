//~ ERROR E0277
struct Point { x: i32 }
fn main() {
    let p: Point = "1".parse().unwrap();
    let _ = p.x;
}
