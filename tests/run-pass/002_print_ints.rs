fn main() {
    println!("{}", 42);
    println!("{}", -7);
    println!("{}", 0);
    println!("a={} b={}", 1, 2);
    println!("{} + {} = {}", 3, 4, 3 + 4);
    println!("{}", "a string literal");
    println!("{} {}", true, false);
    println!("{}", 'x');
    println!("{}", 1_000_000);
    println!("{}", 0xff);
    println!("{}", 0o17);
    println!("{}", 0b1010);
    println!("big: {}", 9223372036854775807i64);
    println!("min: {}", -2147483648i32);
}
