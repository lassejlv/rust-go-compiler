use mathx::{square, Vec2};
use mathx::shapes::Shape;

fn main() {
    println!("square(7) = {}", square(7));
    let v = Vec2::new(3.0, 4.0) + Vec2::new(1.0, 1.0);
    println!("v = {:?}, len = {:.3}", v, v.len());
    println!("fancy = {}", mathx::fancy_name());
    let s = mathx::shapes::Circle { r: 2.0 };
    println!("area = {:.2}", s.area());
    println!("max = {}", mathx::max_of!(3, 9, 4));
    println!("legacy = {}", legacy::double(21));
    println!("legacy macro = {}", legacy::triple!(5));
    let total: i64 = legacy::helpers::sum(&[1, 2, 3]);
    println!("sum = {}", total);
}
