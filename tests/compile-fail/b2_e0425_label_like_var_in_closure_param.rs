//~ ERROR E0425
fn main() {
    let f = |a: i32| a + b;
    f(1);
}
