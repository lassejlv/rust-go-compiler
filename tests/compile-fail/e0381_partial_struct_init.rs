//~ ERROR E0381
struct P {
    x: i32,
    y: i32,
}

fn main() {
    let p: P;
    p.x = 1;
    println!("{}", p.x);
}
