//~ ERROR E0597
fn needs_static(_: &'static str) {}
fn main() {
    let s = String::from("x");
    needs_static(&s);
}
