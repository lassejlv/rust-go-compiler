//~ ERROR E0605
enum Shape { Circle(f64), Square(f64) }
fn main() {
    let s = Shape::Circle(1.0);
    let tag = s as i32;
    println!("{}", tag);
}
