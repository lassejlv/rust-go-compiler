//~ ERROR E0499
fn main() {
    let mut v = vec![1, 2, 3];
    let r = &mut v;
    let mut saved = Vec::new();
    for _ in 0..2 {
        saved.push(&mut *r);
    }
    println!("{}", saved.len());
}
