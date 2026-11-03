//~ ERROR E0596
fn main() {
    let mut o = Some(String::from("a"));
    match &o {
        Some(s) => s.push('b'),
        None => {}
    }
    o = None;
    let _ = o;
}
