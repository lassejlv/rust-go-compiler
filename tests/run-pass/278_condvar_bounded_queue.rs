// A bounded blocking queue with Mutex + two Condvars (not_empty / not_full), close semantics,
// multiple producers/consumers, and a countdown latch; output is order-independent.
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Inner<T> {
    q: VecDeque<T>,
    closed: bool,
    max_seen: usize,
    producer_waits: usize,
}

struct BoundedQueue<T> {
    inner: Mutex<Inner<T>>,
    not_empty: Condvar,
    not_full: Condvar,
    cap: usize,
}

#[derive(Debug, PartialEq)]
enum PushError<T> {
    Closed(T),
}

impl<T> BoundedQueue<T> {
    fn new(cap: usize) -> Self {
        BoundedQueue {
            inner: Mutex::new(Inner { q: VecDeque::new(), closed: false, max_seen: 0, producer_waits: 0 }),
            not_empty: Condvar::new(),
            not_full: Condvar::new(),
            cap,
        }
    }

    fn push(&self, v: T) -> Result<(), PushError<T>> {
        let mut g = self.inner.lock().unwrap();
        while g.q.len() >= self.cap && !g.closed {
            g.producer_waits += 1;
            g = self.not_full.wait(g).unwrap();
        }
        if g.closed {
            return Err(PushError::Closed(v));
        }
        g.q.push_back(v);
        g.max_seen = g.max_seen.max(g.q.len());
        drop(g);
        self.not_empty.notify_one();
        Ok(())
    }

    fn pop(&self) -> Option<T> {
        let mut g = self.inner.lock().unwrap();
        loop {
            if let Some(v) = g.q.pop_front() {
                drop(g);
                self.not_full.notify_one();
                return Some(v);
            }
            if g.closed {
                return None;
            }
            g = self.not_empty.wait(g).unwrap();
        }
    }

    fn close(&self) {
        self.inner.lock().unwrap().closed = true;
        self.not_empty.notify_all();
        self.not_full.notify_all();
    }
}

struct Latch {
    count: Mutex<usize>,
    cv: Condvar,
}

impl Latch {
    fn count_down(&self) {
        let mut c = self.count.lock().unwrap();
        *c -= 1;
        if *c == 0 {
            self.cv.notify_all();
        }
    }
    fn wait(&self) {
        let g = self.count.lock().unwrap();
        let _g = self.cv.wait_while(g, |c| *c > 0).unwrap();
    }
}

fn main() {
    let q = Arc::new(BoundedQueue::<(usize, u64)>::new(4));
    let producers_done = Arc::new(Latch { count: Mutex::new(3), cv: Condvar::new() });

    let producers: Vec<_> = (0..3)
        .map(|p| {
            let q = Arc::clone(&q);
            let latch = Arc::clone(&producers_done);
            thread::spawn(move || {
                for i in 0..50u64 {
                    q.push((p, i * i)).unwrap();
                }
                latch.count_down();
            })
        })
        .collect();

    let consumers: Vec<_> = (0..2)
        .map(|_| {
            let q = Arc::clone(&q);
            thread::spawn(move || {
                let mut per_producer = [0u64; 3];
                let mut n = 0;
                while let Some((p, v)) = q.pop() {
                    per_producer[p] += v;
                    n += 1;
                }
                (n, per_producer)
            })
        })
        .collect();

    // closer waits for all producers via the latch, then closes the queue
    let closer = {
        let q = Arc::clone(&q);
        let latch = Arc::clone(&producers_done);
        thread::spawn(move || {
            latch.wait();
            q.close();
        })
    };

    for p in producers {
        p.join().unwrap();
    }
    closer.join().unwrap();
    let mut total_items = 0;
    let mut sums = [0u64; 3];
    for c in consumers {
        let (n, per) = c.join().unwrap();
        total_items += n;
        for i in 0..3 {
            sums[i] += per[i];
        }
    }
    let expect: u64 = (0..50u64).map(|i| i * i).sum();
    println!("items={} sums={:?} expected each={}", total_items, sums, expect);
    let g = q.inner.lock().unwrap();
    println!("max queue len <= cap: {} (cap {}), closed={}, empty={}", g.max_seen <= q.cap, q.cap, g.closed, g.q.is_empty());
    drop(g);
    println!("push after close: {:?}", q.push((9, 9)));
    println!("pop after close: {:?}", q.pop());

    // ping-pong between two threads using a pair of capacity-1 queues: strictly ordered output
    let a = Arc::new(BoundedQueue::<u32>::new(1));
    let b = Arc::new(BoundedQueue::<u32>::new(1));
    let (a2, b2) = (Arc::clone(&a), Arc::clone(&b));
    let ponger = thread::spawn(move || {
        let mut seen = vec![];
        while let Some(v) = a2.pop() {
            seen.push(v);
            b2.push(v * 10).unwrap();
        }
        b2.close();
        seen
    });
    let mut replies = vec![];
    for i in 1..=5 {
        a.push(i).unwrap();
        replies.push(b.pop().unwrap());
    }
    a.close();
    let seen = ponger.join().unwrap();
    println!("ping {:?} pong {:?} after-close {:?}", seen, replies, b.pop());
}
