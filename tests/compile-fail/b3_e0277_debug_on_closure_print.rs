//~ ERROR E0277
fn main() {
    let f = |x: i32| x + 1;
    println!("{:?}", f);
}
