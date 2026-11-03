//~ ERROR E0571
fn main() {
    let mut i = 0;
    while i < 3 { i += 1; if i == 2 { break 5; } }
}
