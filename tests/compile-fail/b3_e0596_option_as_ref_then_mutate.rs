//~ ERROR E0596
fn main() {
    let mut o = Some(String::new());
    if let Some(s) = o.as_ref() { s.push('x'); }
    o.take();
}
