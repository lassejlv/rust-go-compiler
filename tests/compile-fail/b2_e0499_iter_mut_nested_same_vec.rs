//~ ERROR E0499
fn main() {
    let mut v = vec![1, 2, 3];
    for a in v.iter_mut() {
        for b in v.iter_mut() {
            *b += *a;
        }
    }
}
