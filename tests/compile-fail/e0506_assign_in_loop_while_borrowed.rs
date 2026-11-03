//~ ERROR E0506
fn main() {
    let mut best = 0;
    let mut refs = Vec::new();
    for i in 0..3 {
        refs.push(&best);
        best = i;
    }
    println!("{}", refs.len());
}
