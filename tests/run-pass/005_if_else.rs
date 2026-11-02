fn classify(n: i32) -> &'static str {
    if n < 0 {
        "negative"
    } else if n == 0 {
        "zero"
    } else if n < 10 {
        "small"
    } else {
        "large"
    }
}

fn main() {
    let values = [-5, 0, 3, 42];
    let mut i = 0;
    while i < 4 {
        println!("{} is {}", values[i], classify(values[i]));
        i += 1;
    }
    let x = 7;
    let parity = if x % 2 == 0 { "even" } else { "odd" };
    println!("{} is {}", x, parity);
    if x > 5 && x < 10 {
        println!("x in (5, 10)");
    }
    if x < 0 || x > 6 {
        println!("x < 0 or x > 6");
    }
    if !(x == 3) {
        println!("x is not 3");
    }
    let max = if x > 10 { x } else { 10 };
    println!("max = {}", max);
}
