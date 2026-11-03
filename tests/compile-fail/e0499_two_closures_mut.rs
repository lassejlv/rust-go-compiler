//~ ERROR E0499
fn main() {
    let mut v = Vec::new();
    let mut a = || v.push(1);
    let mut b = || v.push(2);
    a();
    b();
}
