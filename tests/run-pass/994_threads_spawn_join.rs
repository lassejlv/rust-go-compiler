use std::thread;

fn main() {
    let handles: Vec<_> = (0..8)
        .map(|i| {
            thread::spawn(move || {
                let s: u64 = (0..=i * 1000).sum();
                (i, s)
            })
        })
        .collect();
    let mut results: Vec<(u64, u64)> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    results.sort();
    for (i, s) in &results {
        println!("{} -> {}", i, s);
    }
    let data = vec![1, 2, 3];
    let h = thread::spawn(move || data.iter().map(|x| x * 2).collect::<Vec<i32>>());
    println!("{:?}", h.join().unwrap());

    let b = thread::Builder::new().name("worker-7".into()).stack_size(1 << 20);
    let h = b.spawn(|| thread::current().name().map(|s| s.to_string())).unwrap();
    println!("{:?}", h.join().unwrap());
    println!("main name {:?}", thread::current().name());
    let h = thread::spawn(|| thread::current().name().is_none());
    println!("unnamed {}", h.join().unwrap());
    let h = thread::spawn(|| 5);
    println!("finished-eventually {}", { let r = h.join().unwrap(); r });
    let id1 = thread::current().id();
    let id2 = thread::spawn(|| thread::current().id()).join().unwrap();
    println!("ids differ {}", id1 != id2);
    let nested = thread::spawn(|| {
        let inner = thread::spawn(|| "inner");
        format!("outer+{}", inner.join().unwrap())
    });
    println!("{}", nested.join().unwrap());
}
