//~ ERROR E0499
fn main() {
    let mut x = 5;
    let mut refs: Vec<&mut i32> = Vec::new();
    for _ in 0..2 {
        refs.push(&mut x);
    }
    println!("{}", refs.len());
}
