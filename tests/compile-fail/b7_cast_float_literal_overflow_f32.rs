//~ ERROR literal out of range
// `1e40 as f32`: the float literal is typed `f32` and overflows it.
fn main() {
    let big = 1e40 as f32;
    let neg = -1.0e39 as f32;
    println!("{} {}", big, neg);
}
