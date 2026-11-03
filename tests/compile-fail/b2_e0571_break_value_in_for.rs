//~ ERROR E0571
fn main() {
    let r = for i in 0..3 { if i == 1 { break i; } };
}
