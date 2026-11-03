//~ ERROR E0594
fn main() {
    let mut v = vec![1, 2, 3];
    for x in v.iter() {
        *x = 0;
    }
    v.push(1);
}
