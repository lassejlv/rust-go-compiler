//~ ERROR E0506
fn main() {
    let mut v = [1, 2, 3];
    for x in v.iter() {
        v[0] = *x;
    }
}
