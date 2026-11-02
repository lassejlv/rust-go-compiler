fn fib(n: u32) -> u64 {
    if n < 2 {
        n as u64
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

fn factorial(n: u64) -> u64 {
    if n == 0 { 1 } else { n * factorial(n - 1) }
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn ackermann(m: u64, n: u64) -> u64 {
    if m == 0 {
        n + 1
    } else if n == 0 {
        ackermann(m - 1, 1)
    } else {
        ackermann(m - 1, ackermann(m, n - 1))
    }
}

fn is_even(n: u32) -> bool {
    if n == 0 { true } else { is_odd(n - 1) }
}

fn is_odd(n: u32) -> bool {
    if n == 0 { false } else { is_even(n - 1) }
}

fn power(base: i64, exp: u32) -> i64 {
    if exp == 0 {
        1
    } else if exp % 2 == 0 {
        let half = power(base, exp / 2);
        half * half
    } else {
        base * power(base, exp - 1)
    }
}

fn main() {
    for i in 0..15 {
        print!("{} ", fib(i));
    }
    println!();
    println!("fib(30) = {}", fib(30));
    println!("20! = {}", factorial(20));
    println!("gcd(48, 18) = {}", gcd(48, 18));
    println!("gcd(1071, 462) = {}", gcd(1071, 462));
    println!("ackermann(2, 3) = {}", ackermann(2, 3));
    println!("is_even(10) = {}, is_odd(7) = {}", is_even(10), is_odd(7));
    println!("3^13 = {}", power(3, 13));
    println!("(-2)^7 = {}", power(-2, 7));
}
