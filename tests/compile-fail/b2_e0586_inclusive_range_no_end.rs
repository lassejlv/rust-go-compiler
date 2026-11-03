//~ ERROR E0586
fn main() {
    let v = [1, 2, 3];
    let s = &v[1..=];
}
