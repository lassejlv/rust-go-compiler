//~ ERROR E0624
mod shapes {
    pub struct Circle;
    impl Circle { fn unit() -> Circle { Circle } }
}
fn main() { let _c = shapes::Circle::unit(); }
