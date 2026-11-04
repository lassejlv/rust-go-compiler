//~ ERROR overflowing_literals
fn f(x: i8) -> bool {
    matches!(x, -129..=0)
}
fn main() {
    println!("{}", f(3));
}
