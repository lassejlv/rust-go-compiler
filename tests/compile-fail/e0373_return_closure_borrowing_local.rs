//~ ERROR E0373
fn adder(n: i32) -> impl Fn(i32) -> i32 {
    |x| x + n
}

fn main() {
    println!("{}", adder(1)(2));
}
