use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Queue<T> {
    items: Mutex<(VecDeque<T>, bool)>,
    cv: Condvar,
}

impl<T> Queue<T> {
    fn new() -> Self { Queue { items: Mutex::new((VecDeque::new(), false)), cv: Condvar::new() } }
    fn push(&self, v: T) {
        self.items.lock().unwrap().0.push_back(v);
        self.cv.notify_one();
    }
    fn close(&self) {
        self.items.lock().unwrap().1 = true;
        self.cv.notify_all();
    }
    fn pop(&self) -> Option<T> {
        let mut g = self.items.lock().unwrap();
        loop {
            if let Some(v) = g.0.pop_front() { return Some(v); }
            if g.1 { return None; }
            g = self.cv.wait(g).unwrap();
        }
    }
}

fn main() {
    let q = Arc::new(Queue::new());
    let consumers: Vec<_> = (0..3).map(|_| {
        let q = q.clone();
        thread::spawn(move || {
            let mut got = vec![];
            while let Some(v) = q.pop() { got.push(v); }
            got
        })
    }).collect();
    for i in 0..30 { q.push(i); }
    q.close();
    let mut all: Vec<i32> = consumers.into_iter().flat_map(|h| h.join().unwrap()).collect();
    all.sort();
    println!("{} items, sum {}", all.len(), all.iter().sum::<i32>());
    println!("{:?}", &all[..5]);

    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let p2 = pair.clone();
    let h = thread::spawn(move || {
        let (m, c) = &*p2;
        *m.lock().unwrap() = true;
        c.notify_one();
        "signalled"
    });
    let (m, c) = &*pair;
    let g = c.wait_while(m.lock().unwrap(), |ready| !*ready).unwrap();
    println!("ready = {}", *g);
    drop(g);
    println!("{}", h.join().unwrap());
    let (g, to) = c.wait_timeout(m.lock().unwrap(), std::time::Duration::from_millis(5)).unwrap();
    println!("timed out {} value {}", to.timed_out(), *g);
}
