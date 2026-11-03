//~ ERROR E0384
fn main() {
    match Some(3) {
        Some(n) => { n = 4; println!("{}", n); }
        None => {}
    }
}
