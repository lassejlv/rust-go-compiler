use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

enum Job {
    Square(u64),
    Factor(u64),
    Stop,
}

fn factor(mut n: u64) -> Vec<u64> {
    let mut out = vec![];
    let mut d = 2;
    while d * d <= n {
        while n % d == 0 {
            out.push(d);
            n /= d;
        }
        d += 1;
    }
    if n > 1 {
        out.push(n);
    }
    out
}

fn main() {
    let (job_tx, job_rx) = mpsc::channel::<Job>();
    let job_rx = Arc::new(Mutex::new(job_rx));
    let (res_tx, res_rx) = mpsc::channel::<(u64, String)>();
    let n_workers = 4;
    let mut workers = Vec::new();
    for id in 0..n_workers {
        let rx = Arc::clone(&job_rx);
        let tx = res_tx.clone();
        workers.push(thread::spawn(move || {
            let mut handled = 0u32;
            loop {
                let job = rx.lock().unwrap().recv();
                match job {
                    Ok(Job::Square(n)) => {
                        tx.send((n, format!("{}^2 = {}", n, n * n))).unwrap();
                    }
                    Ok(Job::Factor(n)) => {
                        tx.send((n, format!("{} = {:?}", n, factor(n)))).unwrap();
                    }
                    Ok(Job::Stop) | Err(_) => break,
                }
                handled += 1;
            }
            let _ = id;
            handled
        }));
    }
    drop(res_tx);
    let inputs: Vec<u64> = vec![12, 97, 360, 1001, 65536, 999983, 15, 4096, 30030, 77];
    for (i, &n) in inputs.iter().enumerate() {
        if i % 2 == 0 {
            job_tx.send(Job::Factor(n)).unwrap();
        } else {
            job_tx.send(Job::Square(n)).unwrap();
        }
    }
    for _ in 0..n_workers {
        job_tx.send(Job::Stop).unwrap();
    }
    let total_handled: u32 = workers.into_iter().map(|w| w.join().unwrap()).sum();
    let mut results: Vec<(u64, String)> = res_rx.iter().collect();
    results.sort();
    for (_, r) in &results {
        println!("{}", r);
    }
    println!("handled {}", total_handled);

    // parallel map-reduce over chunks with Arc<Vec>
    let data: Arc<Vec<u32>> = Arc::new((1..=10_000).collect());
    let chunk = 2500;
    let hs: Vec<_> = (0..4)
        .map(|k| {
            let d = Arc::clone(&data);
            thread::spawn(move || {
                let slice = &d[k * chunk..(k + 1) * chunk];
                let evens = slice.iter().filter(|&&x| x % 2 == 0).count();
                let max = *slice.iter().max().unwrap();
                (k, evens, max, slice.iter().map(|&x| x as u64).sum::<u64>())
            })
        })
        .collect();
    let mut grand = 0;
    for h in hs {
        let (k, evens, max, sum) = h.join().unwrap();
        println!("chunk {} evens {} max {} sum {}", k, evens, max, sum);
        grand += sum;
    }
    println!("grand {}", grand);
}
