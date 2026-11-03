//~ ERROR lifetime may not live long enough
fn main() {
    let f = |x: &i32| x;
    println!("{}", f(&1));
}
