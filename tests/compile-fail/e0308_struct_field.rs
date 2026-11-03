//~ ERROR E0308
struct P {
    x: f64,
}

fn main() {
    let p = P { x: "1.0" };
    println!("{}", p.x);
}
