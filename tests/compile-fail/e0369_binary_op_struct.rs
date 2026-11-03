//~ ERROR E0369
struct V {
    x: i32,
}

fn main() {
    let a = V { x: 1 };
    let b = V { x: 2 };
    let c = a + b;
}
