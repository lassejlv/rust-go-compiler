//~ ERROR E0596
fn main() {
    let mut v = vec![String::new()];
    for s in v.iter() {
        s.push('x');
    }
}
