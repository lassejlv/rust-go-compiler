//~ ERROR E0609
struct P {
    x: i32,
}

fn main() {
    let p = P { x: 1 };
    println!("{}", p.y);
}
