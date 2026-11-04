//~ ERROR `..` can only be used once per slice pattern
// Only one `..` per slice pattern.
fn main() {
    let a = [1, 2, 3, 4];
    let [first, .., x, ..] = a;
    println!("{} {}", first, x);
}
