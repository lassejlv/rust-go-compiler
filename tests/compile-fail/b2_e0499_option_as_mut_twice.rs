//~ ERROR E0499
fn main() {
    let mut o = Some(String::new());
    let a = o.as_mut().unwrap();
    let b = o.as_mut().unwrap();
    a.push('x');
    b.push('y');
}
