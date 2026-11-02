use std::sync::mpsc::{sync_channel, TrySendError};
use std::thread;

fn main() {
    let (tx, rx) = sync_channel::<u32>(2);
    let producer = thread::spawn(move || {
        let mut sent = 0;
        for i in 0..10 {
            tx.send(i * i).unwrap();
            sent += 1;
        }
        sent
    });
    let mut got = Vec::new();
    for v in rx.iter() {
        got.push(v);
    }
    println!("sent {} got {:?}", producer.join().unwrap(), got);

    // bounded capacity: try_send reports Full
    let (tx, rx) = sync_channel::<&str>(1);
    println!("{:?}", tx.try_send("a"));
    match tx.try_send("b") {
        Err(TrySendError::Full(v)) => println!("full, returned {}", v),
        other => println!("unexpected {:?}", other),
    }
    println!("recv {:?}", rx.recv());
    println!("{:?}", tx.try_send("c"));
    drop(rx);
    match tx.try_send("d") {
        Err(TrySendError::Disconnected(v)) => println!("disconnected, returned {}", v),
        other => println!("unexpected {:?}", other),
    }

    // rendezvous channel (capacity 0)
    let (tx, rx) = sync_channel::<String>(0);
    let (ack_tx, ack_rx) = sync_channel::<usize>(0);
    let worker = thread::spawn(move || {
        let mut lens = 0;
        for s in rx {
            lens += s.len();
            ack_tx.send(lens).unwrap();
        }
        lens
    });
    for word in ["alpha", "be", "gam"] {
        tx.send(word.to_string()).unwrap();
        let running = ack_rx.recv().unwrap();
        println!("sent {} running total {}", word, running);
    }
    drop(tx);
    println!("worker total {}", worker.join().unwrap());

    // many producers into a bounded channel, sum collected
    let (tx, rx) = sync_channel::<u64>(3);
    let hs: Vec<_> = (1..=4u64)
        .map(|k| {
            let tx = tx.clone();
            thread::spawn(move || {
                for j in 1..=25 {
                    tx.send(k * j).unwrap();
                }
            })
        })
        .collect();
    drop(tx);
    let total: u64 = rx.iter().sum();
    for h in hs {
        h.join().unwrap();
    }
    println!("bounded total {}", total);
}
