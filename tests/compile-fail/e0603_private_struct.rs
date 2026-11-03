//~ ERROR E0603
mod shapes {
    struct Circle {
        pub r: f64,
    }
}

fn main() {
    let c = shapes::Circle { r: 1.0 };
    println!("{}", c.r);
}
