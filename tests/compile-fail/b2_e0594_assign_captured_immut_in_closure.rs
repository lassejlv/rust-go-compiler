//~ ERROR E0594
fn main() {
    let x = 1;
    let mut f = || { x = 2; };
    f();
}
