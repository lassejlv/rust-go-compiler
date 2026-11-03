//~ ERROR E0599
mod shapes {
    pub trait Area { fn area(&self) -> f64; }
    pub struct Sq(pub f64);
    impl Area for Sq { fn area(&self) -> f64 { self.0 * self.0 } }
}
fn main() { println!("{}", shapes::Sq(2.0).area()); }
