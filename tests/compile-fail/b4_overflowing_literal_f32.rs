//~ ERROR literal out of range
// A float literal that rounds to infinity in its type.
fn main() {
    let big: f32 = 3.5e38;
    println!("{}", big);
}
