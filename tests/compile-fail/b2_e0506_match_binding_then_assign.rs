//~ ERROR E0506
fn main() {
    let mut o = Some(5);
    match &o {
        Some(v) => { o = None; println!("{}", v); }
        None => {}
    }
}
