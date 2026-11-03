//~ ERROR E0502
fn main() {
    let mut v = vec![1, 2, 3, 4];
    for (i, x) in v.iter().enumerate() {
        if *x % 2 == 0 { v.remove(i); }
    }
}
