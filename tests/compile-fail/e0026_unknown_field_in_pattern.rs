//~ ERROR E0026
struct P {
    x: i32,
    y: i32,
}

fn main() {
    let p = P { x: 1, y: 2 };
    let P { x, z } = p;
    println!("{}", x);
}
