// mpsc channels: multiple producers with tagged messages sorted, sync_channel, iteration until hangup, thread::scope borrowing
use std::sync::mpsc;
use std::thread;

#[derive(Debug)]
enum Msg {
    Data { worker: usize, seq: usize, val: u64 },
    Done(usize),
}

fn pipeline(input: Vec<u32>) -> Vec<String> {
    let (tx1, rx1) = mpsc::channel::<u32>();
    let (tx2, rx2) = mpsc::channel::<(u32, u32)>();
    let stage1 = thread::spawn(move || {
        for x in rx1 {
            tx2.send((x, x * x)).unwrap();
        }
    });
    for x in input {
        tx1.send(x).unwrap();
    }
    drop(tx1);
    stage1.join().unwrap();
    rx2.iter().map(|(a, b)| format!("{}^2={}", a, b)).collect()
}

fn main() {
    let (tx, rx) = mpsc::channel();
    let mut handles = vec![];
    for w in 0..4 {
        let tx = tx.clone();
        handles.push(thread::spawn(move || {
            for seq in 0..3 {
                tx.send(Msg::Data { worker: w, seq, val: (w as u64 + 1) * 100 + seq as u64 }).unwrap();
            }
            tx.send(Msg::Done(w)).unwrap();
        }));
    }
    drop(tx);
    let mut data = vec![];
    let mut done = vec![];
    for m in rx {
        match m {
            Msg::Data { worker, seq, val } => data.push((worker, seq, val)),
            Msg::Done(w) => done.push(w),
        }
    }
    for h in handles {
        h.join().unwrap();
    }
    data.sort();
    done.sort();
    println!("{} msgs, done={:?}", data.len(), done);
    println!("{:?}", &data[..4]);
    println!("total {}", data.iter().map(|d| d.2).sum::<u64>());

    let (stx, srx) = mpsc::sync_channel::<i32>(2);
    let prod = thread::spawn(move || {
        for i in 0..6 {
            stx.send(i).unwrap();
        }
        "producer done"
    });
    let received: Vec<i32> = srx.iter().collect();
    println!("{:?} {}", received, prod.join().unwrap());

    let (tx, rx) = mpsc::channel::<String>();
    drop(rx);
    println!("send to closed: {}", tx.send("x".into()).is_err());
    let (tx, rx) = mpsc::channel::<u8>();
    println!("try_recv empty: {:?}", rx.try_recv());
    tx.send(7).unwrap();
    println!("try_recv: {:?}", rx.try_recv());
    drop(tx);
    println!("after hangup: {:?} {:?}", rx.try_recv(), rx.recv());
    let (tx, rx) = mpsc::channel::<u8>();
    println!("timeout: {:?}", rx.recv_timeout(std::time::Duration::from_millis(5)));
    drop(tx);

    println!("{:?}", pipeline(vec![3, 1, 4, 1, 5]));

    let mut numbers = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let total: i32 = thread::scope(|s| {
        let (left, right) = numbers.split_at_mut(4);
        let h1 = s.spawn(|| {
            left.iter_mut().for_each(|x| *x *= 10);
            left.iter().sum::<i32>()
        });
        let h2 = s.spawn(|| {
            right.reverse();
            right.iter().sum::<i32>()
        });
        h1.join().unwrap() + h2.join().unwrap()
    });
    println!("{} {:?}", total, numbers);
    let words = vec!["alpha", "beta", "gamma"];
    let mut lens = vec![0; words.len()];
    thread::scope(|s| {
        for (w, l) in words.iter().zip(lens.iter_mut()) {
            s.spawn(move || *l = w.len());
        }
    });
    println!("{:?}", lens);
    let (tx, rx) = mpsc::channel();
    thread::scope(|s| {
        for i in 0..3 {
            let tx = tx.clone();
            let w = &words;
            s.spawn(move || tx.send((i, w[i].to_uppercase())).unwrap());
        }
    });
    drop(tx);
    let mut res: Vec<_> = rx.into_iter().collect();
    res.sort();
    println!("{:?}", res);
}
