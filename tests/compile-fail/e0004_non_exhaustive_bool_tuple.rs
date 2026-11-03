//~ ERROR E0004
fn main() {
    let t = (true, false);
    match t {
        (true, true) => {}
        (false, _) => {}
    }
}
