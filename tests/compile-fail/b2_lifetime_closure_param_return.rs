//~ ERROR lifetime
fn main() {
    let f = |x: &i32| -> &i32 { x };
    let y = 5;
    println!("{}", f(&y));
}
