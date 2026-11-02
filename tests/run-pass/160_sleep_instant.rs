use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    let start = Instant::now();
    thread::sleep(Duration::from_millis(1));
    let el = start.elapsed();
    println!("elapsed >= 1ms: {}", el >= Duration::from_millis(1));

    let d = Duration::from_millis(1500);
    println!("{} {} {}", d.as_secs(), d.subsec_millis(), d.as_millis());
    println!("{:?}", d);
    println!("{:?} {:?}", Duration::from_secs(2) + Duration::from_millis(250), Duration::from_micros(1));
    println!("{:?}", Duration::from_secs(3).checked_sub(Duration::from_secs(5)));
    println!("{:?}", Duration::from_secs(10) / 4);
    println!("{}", Duration::from_secs_f64(0.25).as_millis());
    println!("{:?}", Duration::from_nanos(1_000_000_123));
    println!("{}", Duration::ZERO.is_zero());

    let (tx, rx) = mpsc::channel();
    let hs: Vec<_> = (0..4u64)
        .map(|i| {
            let tx = tx.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(1 + (3 - i)));
                tx.send(i).unwrap();
            })
        })
        .collect();
    drop(tx);
    let mut got: Vec<u64> = rx.iter().collect();
    for h in hs {
        h.join().unwrap();
    }
    got.sort();
    println!("got {:?}", got);

    let t0 = Instant::now();
    let h = thread::spawn(|| {
        thread::sleep(Duration::from_millis(2));
        "slept"
    });
    let msg = h.join().unwrap();
    let t1 = Instant::now();
    println!("{} {}", msg, t1.duration_since(t0) >= Duration::from_millis(2));
    println!("later >= earlier: {}", t1 >= t0);
    println!("checked_duration_since reversed is none-or-zero: {}", t0.checked_duration_since(t1).map_or(true, |d| d.is_zero()));

    let (tx, rx) = mpsc::channel::<i32>();
    let r = rx.recv_timeout(Duration::from_millis(5));
    println!("timeout: {:?}", r);
    tx.send(5).unwrap();
    println!("recv_timeout: {:?}", rx.recv_timeout(Duration::from_millis(100)));
    thread::yield_now();
    println!("done");
}
