//~ ERROR E0562
fn main() {
    let x: impl Fn() -> i32 = || 1;
    println!("{}", x());
}
