// mpsc channels: single producer ordering, multiple producers with per-producer sequence numbers
// sorted at the end, sync_channel(0) rendezvous, try_recv/recv after disconnect, iterating receiver
// until all senders drop, sending Box<dyn FnOnce> jobs to a worker, and request/response channels.
use std::sync::mpsc::{self, RecvError, TryRecvError};
use std::thread;

enum Job {
    Run(Box<dyn FnOnce() -> String + Send>),
    Stop,
}

fn main() {
    // single producer keeps order
    let (tx, rx) = mpsc::channel();
    let h = thread::spawn(move || {
        for i in 0..5 {
            tx.send(format!("msg{}", i)).unwrap();
        }
    });
    let got: Vec<String> = rx.iter().collect();
    h.join().unwrap();
    println!("{:?}", got);

    // multiple producers: collect then sort for deterministic output
    let (tx, rx) = mpsc::channel::<(usize, usize)>();
    let mut hs = vec![];
    for p in 0..3 {
        let tx = tx.clone();
        hs.push(thread::spawn(move || {
            for seq in 0..4 {
                tx.send((p, seq * 10 + p)).unwrap();
            }
        }));
    }
    drop(tx);
    let mut all: Vec<(usize, usize)> = rx.into_iter().collect();
    for h in hs {
        h.join().unwrap();
    }
    // within each producer order is preserved
    let mut per: Vec<Vec<usize>> = vec![vec![]; 3];
    for (p, v) in &all {
        per[*p].push(*v);
    }
    println!("per producer {:?}", per);
    all.sort();
    println!("count {} first {:?} last {:?}", all.len(), all.first(), all.last());

    // rendezvous channel
    let (stx, srx) = mpsc::sync_channel::<u32>(0);
    let h = thread::spawn(move || {
        let mut acc = vec![];
        for _ in 0..3 {
            acc.push(srx.recv().unwrap());
        }
        acc
    });
    for v in [7, 8, 9] {
        stx.send(v).unwrap();
    }
    println!("rendezvous {:?}", h.join().unwrap());

    // try_recv states
    let (tx, rx) = mpsc::channel::<i32>();
    println!("{:?}", rx.try_recv() == Err(TryRecvError::Empty));
    tx.send(1).unwrap();
    println!("{:?}", rx.try_recv());
    drop(tx);
    println!("{:?} {:?}", rx.try_recv() == Err(TryRecvError::Disconnected), rx.recv() == Err(RecvError));

    // send to dropped receiver
    let (tx, rx) = mpsc::channel::<String>();
    drop(rx);
    let e = tx.send("lost".into()).unwrap_err();
    println!("send error returns {:?}", e.0);

    // job queue with boxed closures
    let (jtx, jrx) = mpsc::channel::<Job>();
    let (rtx, rrx) = mpsc::channel::<String>();
    let worker = thread::spawn(move || {
        let mut n = 0;
        while let Ok(job) = jrx.recv() {
            match job {
                Job::Run(f) => {
                    n += 1;
                    rtx.send(format!("#{} {}", n, f())).unwrap();
                }
                Job::Stop => break,
            }
        }
        n
    });
    let base = 100;
    for i in 0..3 {
        jtx.send(Job::Run(Box::new(move || format!("{}", base + i * i)))).unwrap();
    }
    let owned = String::from("owned data");
    jtx.send(Job::Run(Box::new(move || owned.to_uppercase()))).unwrap();
    jtx.send(Job::Stop).unwrap();
    jtx.send(Job::Run(Box::new(|| "never".into()))).ok();
    let processed = worker.join().unwrap();
    let results: Vec<String> = rrx.iter().collect();
    println!("processed {} {:?}", processed, results);

    // request/response: each request carries its own reply sender
    let (req_tx, req_rx) = mpsc::channel::<(u64, mpsc::Sender<u64>)>();
    let server = thread::spawn(move || {
        for (n, reply) in req_rx {
            let fib = (0..n).fold((0u64, 1u64), |(a, b), _| (b, a + b)).0;
            reply.send(fib).unwrap();
        }
    });
    let mut answers = vec![];
    for n in [10, 20, 50, 90] {
        let (tx, rx) = mpsc::channel();
        req_tx.send((n, tx)).unwrap();
        answers.push((n, rx.recv().unwrap()));
    }
    drop(req_tx);
    server.join().unwrap();
    println!("{:?}", answers);

    // pipeline of stages
    let (a_tx, a_rx) = mpsc::channel::<i32>();
    let (b_tx, b_rx) = mpsc::channel::<i32>();
    let s1 = thread::spawn(move || {
        for v in a_rx {
            b_tx.send(v * v).unwrap();
        }
    });
    for v in 1..=5 {
        a_tx.send(v).unwrap();
    }
    drop(a_tx);
    let out: Vec<i32> = b_rx.iter().collect();
    s1.join().unwrap();
    println!("pipeline {:?}", out);
}
