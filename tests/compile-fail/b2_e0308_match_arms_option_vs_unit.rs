//~ ERROR E0308
fn main() {
    let x = 3;
    let r = match x {
        1 => Some(1),
        _ => (),
    };
}
