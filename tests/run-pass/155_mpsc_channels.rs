use std::sync::mpsc;
use std::thread;

#[derive(Debug)]
enum Msg {
    Data { producer: u32, value: u32 },
    Done(u32),
}

fn main() {
    let (tx, rx) = mpsc::channel();
    for p in 0..4u32 {
        let tx = tx.clone();
        thread::spawn(move || {
            for k in 0..5 {
                tx.send(Msg::Data { producer: p, value: p * 100 + k }).unwrap();
            }
            tx.send(Msg::Done(p)).unwrap();
        });
    }
    drop(tx);
    let mut values = Vec::new();
    let mut done = Vec::new();
    for m in rx {
        match m {
            Msg::Data { producer, value } => values.push((producer, value)),
            Msg::Done(p) => done.push(p),
        }
    }
    values.sort();
    done.sort();
    println!("received {} values", values.len());
    for chunk in values.chunks(5) {
        println!("{:?}", chunk.iter().map(|(_, v)| *v).collect::<Vec<_>>());
    }
    println!("done {:?}", done);

    // per-producer order is preserved
    let (tx, rx) = mpsc::channel::<(usize, usize)>();
    let hs: Vec<_> = (0..3)
        .map(|id| {
            let tx = tx.clone();
            thread::spawn(move || {
                for seq in 0..50 {
                    tx.send((id, seq)).unwrap();
                }
            })
        })
        .collect();
    drop(tx);
    let mut last = [None::<usize>; 3];
    let mut in_order = true;
    let mut count = 0;
    while let Ok((id, seq)) = rx.recv() {
        if let Some(prev) = last[id] {
            if seq != prev + 1 {
                in_order = false;
            }
        }
        last[id] = Some(seq);
        count += 1;
    }
    for h in hs {
        h.join().unwrap();
    }
    println!("count {} in_order {} last {:?}", count, in_order, last);

    // pipeline: stage1 -> stage2 -> main
    let (tx1, rx1) = mpsc::channel::<i64>();
    let (tx2, rx2) = mpsc::channel::<String>();
    let stage2 = thread::spawn(move || {
        for v in rx1 {
            tx2.send(format!("sq({})={}", v, v * v)).unwrap();
        }
    });
    for i in 1..=5 {
        tx1.send(i).unwrap();
    }
    drop(tx1);
    let out: Vec<String> = rx2.iter().collect();
    stage2.join().unwrap();
    println!("{}", out.join(", "));

    let (tx, rx) = mpsc::channel::<i32>();
    println!("try_recv empty: {:?}", rx.try_recv());
    tx.send(9).unwrap();
    println!("try_recv: {:?}", rx.try_recv());
    drop(tx);
    println!("after drop: {:?}", rx.recv());
    println!("try after drop: {:?}", rx.try_recv());
    let (tx, rx) = mpsc::channel::<i32>();
    drop(rx);
    println!("send to closed: {}", tx.send(1).is_err());
}
