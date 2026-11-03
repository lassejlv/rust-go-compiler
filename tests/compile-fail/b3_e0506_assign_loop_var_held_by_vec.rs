//~ ERROR E0506
fn main() {
    let mut refs: Vec<&i32> = Vec::new();
    let mut x = 0;
    while x < 3 {
        refs.push(&x);
        x += 1;
    }
}
