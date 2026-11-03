//~ ERROR E0510
fn main() {
    let mut x = Some(1);
    match x {
        None => {}
        Some(_) if { x = None; false } => {}
        Some(_) => {}
    }
}
