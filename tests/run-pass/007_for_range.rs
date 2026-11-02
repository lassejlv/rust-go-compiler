fn main() {
    for i in 0..5 {
        print!("{} ", i);
    }
    println!();
    for i in 1..=5 {
        print!("{} ", i);
    }
    println!();
    for i in (0..5).rev() {
        print!("{} ", i);
    }
    println!();
    for i in (0..20).step_by(3) {
        print!("{} ", i);
    }
    println!();
    for i in (1..=10).rev().step_by(4) {
        print!("{} ", i);
    }
    println!();
    let mut total = 0;
    for i in 0..10 {
        if i == 7 {
            break;
        }
        if i % 3 == 0 {
            continue;
        }
        total += i;
    }
    println!("total = {}", total);
    for i in 1..4 {
        for j in 1..4 {
            print!("{:3}", i * j);
        }
        println!();
    }
    let n = 0;
    for _ in 0..n {
        println!("never");
    }
    for i in 10..10 {
        println!("never {}", i);
    }
    println!("done");
}
