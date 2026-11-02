fn main() {
    let mut i = 0;
    while i < 5 {
        println!("while i = {}", i);
        i += 1;
    }
    let mut sum = 0;
    let mut n = 1;
    while n <= 100 {
        sum += n;
        n += 1;
    }
    println!("sum 1..=100 = {}", sum);

    let mut count = 0;
    let result = loop {
        count += 1;
        if count == 10 {
            break count * 2;
        }
    };
    println!("loop result = {}", result);

    let mut k = 0;
    loop {
        k += 1;
        if k % 2 == 0 {
            continue;
        }
        if k > 9 {
            break;
        }
        println!("odd k = {}", k);
    }

    // Collatz
    let mut c: u64 = 27;
    let mut steps = 0;
    while c != 1 {
        c = if c % 2 == 0 { c / 2 } else { 3 * c + 1 };
        steps += 1;
    }
    println!("collatz(27) steps = {}", steps);
}
