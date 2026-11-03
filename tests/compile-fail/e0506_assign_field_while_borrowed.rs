//~ ERROR E0506
struct P {
    x: i32,
    y: i32,
}

fn main() {
    let mut p = P { x: 1, y: 2 };
    let rx = &p.x;
    p.x = 5;
    println!("{} {}", rx, p.y);
}
