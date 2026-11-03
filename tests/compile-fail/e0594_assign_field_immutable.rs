//~ ERROR E0594
struct P {
    x: i32,
}

fn main() {
    let p = P { x: 1 };
    p.x = 2;
    println!("{}", p.x);
}
