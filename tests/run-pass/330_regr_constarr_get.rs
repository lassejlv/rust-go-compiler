const NAMES: &[&str] = &["a", "b", "c"];
fn name<'a>(i: usize) -> Option<&'a str> {
    match NAMES.get(i) {
        Some(name) => Some(name),
        None => None,
    }
}
fn main() {
    println!("{:?} {:?}", name(1), name(7));
}
