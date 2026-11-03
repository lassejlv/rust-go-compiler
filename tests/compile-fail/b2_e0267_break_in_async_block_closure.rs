//~ ERROR E0267
fn main() {
    for i in 0..3 {
        let f = || { if i == 1 { break; } };
        f();
    }
}
