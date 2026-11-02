fn main() {
    let x = 5;
    println!("x = {}", x);
    let mut y = 10;
    println!("y = {}", y);
    y = y + x;
    println!("y = {}", y);
    y += 3;
    println!("y += 3 -> {}", y);
    y -= 1;
    println!("y -= 1 -> {}", y);
    y *= 2;
    println!("y *= 2 -> {}", y);
    y /= 3;
    println!("y /= 3 -> {}", y);
    y %= 7;
    println!("y %= 7 -> {}", y);
    let z;
    if y > 2 {
        z = 100;
    } else {
        z = 200;
    }
    println!("z = {}", z);
    let (p, q) = (1, 2);
    println!("p = {}, q = {}", p, q);
    let mut counter = 0;
    counter += 1;
    counter += 1;
    println!("counter = {}", counter);
    let _unused = 99;
    let block_value = {
        let t = 3;
        t * t + 1
    };
    println!("block_value = {}", block_value);
}
