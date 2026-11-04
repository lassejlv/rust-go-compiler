//~ ERROR E0658
// Box patterns are unstable.
fn main() {
    let b = Some(Box::new(5));
    match b {
        Some(box n) => println!("{}", n),
        None => {}
    }
}
