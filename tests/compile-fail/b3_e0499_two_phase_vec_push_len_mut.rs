//~ ERROR E0499
fn grow(v: &mut Vec<usize>) -> usize { v.push(1); v.len() }
fn main() {
    let mut v = vec![0usize];
    v.push(grow(&mut v));
    println!("{:?}", v);
}
