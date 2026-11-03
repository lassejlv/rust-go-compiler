use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;

fn stage<I: Send + 'static, O: Send + 'static>(
    name: &'static str,
    rx: Receiver<I>,
    f: impl Fn(I) -> Option<O> + Send + 'static,
) -> (Receiver<O>, thread::JoinHandle<(&'static str, usize)>) {
    let (tx, out): (SyncSender<O>, Receiver<O>) = sync_channel(2);
    let h = thread::spawn(move || {
        let mut n = 0;
        for item in rx {
            n += 1;
            if let Some(o) = f(item) {
                if tx.send(o).is_err() { break; }
            }
        }
        (name, n)
    });
    (out, h)
}

struct Gate { open: Mutex<bool>, cv: Condvar }

fn main() {
    let (src_tx, src_rx) = sync_channel::<u32>(4);
    let (r1, h1) = stage("square", src_rx, |x| Some(x * x));
    let (r2, h2) = stage("filter_odd", r1, |x| if x % 2 == 1 { Some(x) } else { None });
    let (r3, h3) = stage("format", r2, |x| Some(format!("<{}>", x)));
    let producer = thread::spawn(move || {
        for i in 0..15 { src_tx.send(i).unwrap(); }
    });
    let collected: Vec<String> = r3.iter().collect();
    producer.join().unwrap();
    for h in [h1, h2] {
        let (n, c) = h.join().unwrap();
        println!("{} processed {}", n, c);
    }
    let (n, c) = h3.join().unwrap();
    println!("{} processed {}", n, c);
    println!("{}", collected.join(","));

    let config = Arc::new(RwLock::new(vec![1, 2, 3]));
    let gate = Arc::new(Gate { open: Mutex::new(false), cv: Condvar::new() });
    let readers: Vec<_> = (0..3).map(|i| {
        let c = config.clone();
        let g = gate.clone();
        thread::spawn(move || {
            let mut open = g.open.lock().unwrap();
            while !*open { open = g.cv.wait(open).unwrap(); }
            drop(open);
            let snap = c.read().unwrap();
            (i, snap.iter().sum::<i32>())
        })
    }).collect();
    config.write().unwrap().push(4);
    {
        *gate.open.lock().unwrap() = true;
        gate.cv.notify_all();
    }
    let mut res: Vec<_> = readers.into_iter().map(|h| h.join().unwrap()).collect();
    res.sort();
    println!("{:?}", res);
    let rw = RwLock::new(String::from("x"));
    {
        let r = rw.read().unwrap();
        println!("read {} try_read again ok {}", *r, rw.try_read().is_ok());
    }
    rw.write().unwrap().push('y');
    let w = rw.write().unwrap();
    let downgraded = std::sync::RwLockWriteGuard::downgrade(w);
    println!("downgraded {}", *downgraded);
}
