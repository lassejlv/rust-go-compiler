//~ ERROR E0594
fn main() {
    let mut v = vec![1, 2];
    for x in v.iter() {
        *x += 1;
    }
}
