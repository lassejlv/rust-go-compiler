// mpsc: multiple producers, sync_channel(0) rendezvous, bounded, recv_timeout, try_recv, iter.
use std::sync::mpsc::{self, RecvTimeoutError, TryRecvError, TrySendError};
use std::thread;
use std::time::Duration;

fn main() {
    let (tx, rx) = mpsc::channel::<(usize, u32)>();
    for id in 0..4 {
        let tx = tx.clone();
        thread::spawn(move || {
            for k in 0..3 {
                tx.send((id, k * id as u32)).unwrap();
            }
        });
    }
    drop(tx);
    let mut all: Vec<_> = rx.iter().collect();
    all.sort();
    println!("{} msgs: {:?}", all.len(), all);

    // rendezvous channel
    let (stx, srx) = mpsc::sync_channel::<String>(0);
    let h = thread::spawn(move || {
        for w in ["one", "two", "three"] {
            stx.send(w.to_string()).unwrap();
        }
        "producer done"
    });
    let got: Vec<String> = srx.iter().collect();
    println!("{:?} {}", got, h.join().unwrap());

    // bounded channel try_send
    let (btx, brx) = mpsc::sync_channel::<i32>(2);
    println!("{:?}", btx.try_send(1));
    println!("{:?}", btx.try_send(2));
    match btx.try_send(3) {
        Err(TrySendError::Full(v)) => println!("full, got back {}", v),
        other => println!("unexpected {:?}", other),
    }
    println!("{:?} {:?}", brx.recv(), brx.try_recv());
    println!("{:?}", brx.try_recv());
    drop(btx);
    println!("{:?}", brx.try_recv() == Err(TryRecvError::Disconnected));
    println!("{:?}", brx.recv());

    // recv_timeout
    let (ttx, trx) = mpsc::channel::<u8>();
    println!("{:?}", trx.recv_timeout(Duration::from_millis(15)));
    ttx.send(9).unwrap();
    println!("{:?}", trx.recv_timeout(Duration::from_millis(15)));
    drop(ttx);
    println!("{:?}", trx.recv_timeout(Duration::from_millis(15)) == Err(RecvTimeoutError::Disconnected));

    // send after receiver dropped
    let (dtx, drx) = mpsc::channel::<Vec<u8>>();
    drop(drx);
    match dtx.send(vec![1, 2]) {
        Err(e) => println!("send error returned {:?}: {}", e.0, e),
        Ok(()) => println!("??"),
    }

    // try_iter and pipeline stages
    let (a_tx, a_rx) = mpsc::channel::<u64>();
    let (b_tx, b_rx) = mpsc::channel::<u64>();
    let stage = thread::spawn(move || {
        for x in a_rx {
            b_tx.send(x * 3).unwrap();
        }
    });
    for i in 0..10 {
        a_tx.send(i).unwrap();
    }
    drop(a_tx);
    stage.join().unwrap();
    let v: Vec<u64> = b_rx.try_iter().collect();
    println!("pipeline {:?}", v);
}
