//~ ERROR E0027
struct P {
    x: i32,
    y: i32,
}

fn main() {
    let p = P { x: 1, y: 2 };
    let P { x } = p;
    println!("{}", x);
}
