//~ ERROR E0005
fn main() {
    let v: &[i32] = &[1, 2];
    let [a, b] = v;
}
