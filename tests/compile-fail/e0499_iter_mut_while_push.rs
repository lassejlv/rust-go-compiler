//~ ERROR E0499
fn main() {
    let mut v = vec![1, 2, 3];
    for x in v.iter_mut() {
        if *x == 2 {
            v.push(4);
        }
        *x += 1;
    }
}
