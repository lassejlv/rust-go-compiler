//~ ERROR E0046
trait Shape {
    fn area(&self) -> f64;
    fn name(&self) -> String;
}

struct Sq(f64);

impl Shape for Sq {
    fn area(&self) -> f64 {
        self.0 * self.0
    }
}

fn main() {
    println!("{}", Sq(2.0).area());
}
