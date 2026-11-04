//~ ERROR E0415
// Also for closure parameters, including inside tuple patterns.
fn main() {
    let f = |(x, y): (i32, i32), x: i32| x + y;
    println!("{}", f((1, 2), 3));
}
