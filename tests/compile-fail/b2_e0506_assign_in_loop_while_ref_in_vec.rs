//~ ERROR E0506
fn main() {
    let mut x = 0;
    let mut refs: Vec<&i32> = Vec::new();
    for i in 0..3 {
        refs.push(&x);
        x = i;
    }
    println!("{:?}", refs);
}
