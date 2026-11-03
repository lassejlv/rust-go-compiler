//~ ERROR E0451
mod m {
    pub struct P {
        pub x: i32,
        y: i32,
    }
}

fn main() {
    let p = m::P { x: 1, y: 2 };
    println!("{}", p.x);
}
