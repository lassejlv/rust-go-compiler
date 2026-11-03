//~ ERROR E0317
fn main() {
    let x = 3;
    let msg: &str = if x > 1 { "big" };
}
