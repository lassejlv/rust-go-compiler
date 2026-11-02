fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn square(x: i64) -> i64 {
    return x * x;
}

fn print_twice(n: i32) {
    println!("n = {}", n);
    println!("n = {}", n);
}

fn early_return(x: i32) -> i32 {
    if x < 0 {
        return 0;
    }
    x * 10
}

fn swap(a: i32, b: i32) -> (i32, i32) {
    (b, a)
}

fn is_even(n: u32) -> bool {
    n % 2 == 0
}

fn unit_fn() {}

fn main() {
    println!("add = {}", add(2, 3));
    println!("square = {}", square(12));
    print_twice(9);
    println!("early_return(-5) = {}", early_return(-5));
    println!("early_return(5) = {}", early_return(5));
    let (a, b) = swap(1, 2);
    println!("swapped = {} {}", a, b);
    println!("is_even(4) = {}, is_even(7) = {}", is_even(4), is_even(7));
    unit_fn();
    let r = add(add(1, 2), add(3, 4));
    println!("nested = {}", r);
}
