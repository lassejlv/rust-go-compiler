use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Queue {
    items: VecDeque<u32>,
    closed: bool,
}

fn main() {
    let pair = Arc::new((Mutex::new(Queue { items: VecDeque::new(), closed: false }), Condvar::new()));

    let consumers: Vec<_> = (0..3)
        .map(|_| {
            let p = Arc::clone(&pair);
            thread::spawn(move || {
                let (lock, cv) = &*p;
                let mut taken = Vec::new();
                loop {
                    let mut q = lock.lock().unwrap();
                    while q.items.is_empty() && !q.closed {
                        q = cv.wait(q).unwrap();
                    }
                    match q.items.pop_front() {
                        Some(v) => taken.push(v),
                        None => break,
                    }
                }
                taken
            })
        })
        .collect();

    {
        let (lock, cv) = &*pair;
        for i in 1..=30 {
            lock.lock().unwrap().items.push_back(i);
            cv.notify_one();
        }
        lock.lock().unwrap().closed = true;
        cv.notify_all();
    }

    let mut all: Vec<u32> = Vec::new();
    let mut total_count = 0;
    for c in consumers {
        let v = c.join().unwrap();
        total_count += v.len();
        all.extend(v);
    }
    all.sort();
    println!("consumed {} items", total_count);
    println!("sum {}", all.iter().sum::<u32>());
    println!("all unique and complete: {}", all == (1..=30).collect::<Vec<_>>());

    // simple ready flag
    let flag = Arc::new((Mutex::new(false), Condvar::new()));
    let f2 = Arc::clone(&flag);
    let waiter = thread::spawn(move || {
        let (m, cv) = &*f2;
        let guard = cv.wait_while(m.lock().unwrap(), |ready| !*ready).unwrap();
        *guard
    });
    {
        let (m, cv) = &*flag;
        *m.lock().unwrap() = true;
        cv.notify_all();
    }
    println!("waiter saw ready = {}", waiter.join().unwrap());

    // ping-pong with turn variable
    let state = Arc::new((Mutex::new((0u32, Vec::<String>::new())), Condvar::new()));
    let mut hs = Vec::new();
    for me in 0..2u32 {
        let s = Arc::clone(&state);
        hs.push(thread::spawn(move || {
            let (m, cv) = &*s;
            for round in 0..3 {
                let mut g = cv.wait_while(m.lock().unwrap(), |(turn, _)| *turn % 2 != me).unwrap();
                g.1.push(format!("{}{}", if me == 0 { "ping" } else { "pong" }, round));
                g.0 += 1;
                cv.notify_all();
            }
        }));
    }
    for h in hs {
        h.join().unwrap();
    }
    let g = state.0.lock().unwrap();
    println!("turns {} log {:?}", g.0, g.1);
}
