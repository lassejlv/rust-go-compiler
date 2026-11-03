//~ ERROR E0277
struct Point {
    x: i32,
}

fn main() {
    let p = Point { x: 1 };
    println!("{:?}", p);
}
