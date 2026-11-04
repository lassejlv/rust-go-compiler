//~ ERROR E0716
// `&mut` of a constant expression is not promoted to a static (only `&mut []`)
fn id<T>(x: T) -> T { x }
fn main() {
    let a = id(&mut [1, 2, 3]);
    a[0] = 5;
    println!("{:?}", a);
}
