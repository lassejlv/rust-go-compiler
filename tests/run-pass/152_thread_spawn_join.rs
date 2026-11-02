use std::thread;

fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    let mut d = 2;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += 1;
    }
    true
}

fn main() {
    let h = thread::spawn(|| 6 * 7);
    println!("answer = {}", h.join().unwrap());

    let name = String::from("worker");
    let h = thread::spawn(move || format!("{} says hi, len {}", name, name.len()));
    println!("{}", h.join().unwrap());

    let mut handles = Vec::new();
    for i in 0..8u64 {
        handles.push(thread::spawn(move || {
            let lo = i * 1000;
            let hi = lo + 1000;
            (i, (lo..hi).filter(|&n| is_prime(n)).count())
        }));
    }
    let mut total = 0;
    for h in handles {
        let (i, c) = h.join().unwrap();
        println!("chunk {} primes {}", i, c);
        total += c;
    }
    println!("total primes below 8000: {}", total);

    let data: Vec<Vec<i32>> = (0..4).map(|k| (0..10).map(|x| x * k).collect()).collect();
    let handles: Vec<thread::JoinHandle<i32>> = data
        .into_iter()
        .map(|row| thread::spawn(move || row.iter().sum()))
        .collect();
    let sums: Vec<i32> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    println!("row sums {:?}", sums);

    let builder = thread::Builder::new().name("named-thread".to_string());
    let h = builder
        .spawn(|| thread::current().name().map(|s| s.to_string()))
        .unwrap();
    println!("thread name: {:?}", h.join().unwrap());

    let nested = thread::spawn(|| {
        let inner = thread::spawn(|| (1..=10).product::<u64>());
        inner.join().unwrap() + 1
    });
    println!("nested = {}", nested.join().unwrap());
    let main_name = thread::current().name().map(String::from);
    println!("main thread name: {:?}", main_name);
    let h = thread::spawn(|| {
        let v: Vec<String> = vec!["a".into(), "b".into()];
        v
    });
    let v = h.join().unwrap();
    println!("{:?} finished={}", v, true);
}
