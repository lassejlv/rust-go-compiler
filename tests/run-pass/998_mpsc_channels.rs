use std::sync::mpsc;
use std::thread;

fn main() {
    let (tx, rx) = mpsc::channel();
    let mut hs = vec![];
    for id in 0..4 {
        let tx = tx.clone();
        hs.push(thread::spawn(move || {
            for j in 0..5 {
                tx.send((id, j, id * 100 + j)).unwrap();
            }
        }));
    }
    drop(tx);
    let mut got: Vec<_> = rx.iter().collect();
    for h in hs { h.join().unwrap(); }
    got.sort();
    println!("{} msgs first {:?} last {:?}", got.len(), got[0], got[got.len() - 1]);
    let per: Vec<i32> = (0..4).map(|id| got.iter().filter(|m| m.0 == id).map(|m| m.2).sum()).collect();
    println!("{:?}", per);

    let (stx, srx) = mpsc::sync_channel::<String>(2);
    let prod = thread::spawn(move || {
        for w in ["alpha", "beta", "gamma", "delta"] {
            stx.send(w.to_string()).unwrap();
        }
        "producer done"
    });
    let mut words = vec![];
    for w in srx.iter() { words.push(w); }
    println!("{:?} {}", words, prod.join().unwrap());

    let (tx, rx) = mpsc::channel::<i32>();
    println!("try_recv empty: {:?}", rx.try_recv());
    tx.send(1).unwrap();
    println!("try_recv: {:?}", rx.try_recv());
    drop(tx);
    println!("after drop: {:?}", rx.try_recv());
    println!("recv: {:?}", rx.recv());
    let (tx, rx) = mpsc::channel::<i32>();
    drop(rx);
    println!("send to closed: {}", tx.send(5).is_err());
    if let Err(mpsc::SendError(v)) = tx.send(9) { println!("returned {}", v); }

    let (rtx, rrx) = mpsc::sync_channel::<u8>(0);
    let h = thread::spawn(move || {
        let v = rrx.recv().unwrap();
        v * 2
    });
    rtx.send(21).unwrap();
    println!("rendezvous {}", h.join().unwrap());
    let (tx, rx) = mpsc::sync_channel(1);
    tx.try_send(1).unwrap();
    match tx.try_send(2) {
        Err(mpsc::TrySendError::Full(v)) => println!("full {}", v),
        other => println!("{:?}", other),
    }
    println!("{:?}", rx.recv_timeout(std::time::Duration::from_millis(10)));
    println!("{:?}", rx.recv_timeout(std::time::Duration::from_millis(10)));

    let (tx, rx) = mpsc::channel::<Box<dyn FnOnce() -> String + Send>>();
    let worker = thread::spawn(move || rx.into_iter().map(|f| f()).collect::<Vec<_>>());
    for i in 0..3 {
        tx.send(Box::new(move || format!("job{}", i))).unwrap();
    }
    drop(tx);
    println!("{:?}", worker.join().unwrap());
}
