//~ ERROR E0506
fn main() {
    let mut v = vec![1, 2, 3];
    for x in &v {
        if *x == 2 { v = Vec::new(); }
    }
}
