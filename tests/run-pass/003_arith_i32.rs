fn main() {
    let a = 17;
    let b = 5;
    println!("a + b = {}", a + b);
    println!("a - b = {}", a - b);
    println!("a * b = {}", a * b);
    println!("a / b = {}", a / b);
    println!("a % b = {}", a % b);
    println!("-a / b = {}", -a / b);
    println!("-a % b = {}", -a % b);
    println!("a / -b = {}", a / -b);
    println!("precedence: {}", 2 + 3 * 4 - 6 / 2);
    println!("parens: {}", (2 + 3) * (4 - 6) / 2);
    let c: i32 = -(a * 2) + b;
    println!("c = {}", c);
    println!("abs = {}", c.abs());
    println!("pow = {}", 2i32.pow(10));
    println!("min/max = {} {}", a.min(b), a.max(b));
    println!("rem_euclid = {}", (-7i32).rem_euclid(3));
    println!("div_euclid = {}", (-7i32).div_euclid(3));
    println!("signum = {} {} {}", (-5i32).signum(), 0i32.signum(), 9i32.signum());
}
