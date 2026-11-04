//~ ERROR E0415
// Two parameters of one function bind the same name.
fn add(a: i32, a: i32) -> i32 {
    a + a
}
fn main() {
    println!("{}", add(1, 2));
}
