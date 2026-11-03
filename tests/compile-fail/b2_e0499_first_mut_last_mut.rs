//~ ERROR E0499
fn main() {
    let mut v = vec![1, 2, 3];
    let f = v.first_mut().unwrap();
    let l = v.last_mut().unwrap();
    std::mem::swap(f, l);
}
