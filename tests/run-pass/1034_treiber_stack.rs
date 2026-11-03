use std::ptr;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

struct Node<T> { val: T, next: *mut Node<T> }

pub struct Stack<T> { head: AtomicPtr<Node<T>>, len: AtomicUsize }

unsafe impl<T: Send> Send for Stack<T> {}
unsafe impl<T: Send> Sync for Stack<T> {}

impl<T> Stack<T> {
    fn new() -> Self { Stack { head: AtomicPtr::new(ptr::null_mut()), len: AtomicUsize::new(0) } }
    fn push(&self, val: T) {
        let n = Box::into_raw(Box::new(Node { val, next: ptr::null_mut() }));
        let mut cur = self.head.load(Ordering::Relaxed);
        loop {
            unsafe { (*n).next = cur; }
            match self.head.compare_exchange_weak(cur, n, Ordering::Release, Ordering::Relaxed) {
                Ok(_) => break,
                Err(actual) => cur = actual,
            }
        }
        self.len.fetch_add(1, Ordering::Relaxed);
    }
    // only safe single-threaded (no ABA protection); used after joins
    fn pop(&self) -> Option<T> {
        let cur = self.head.load(Ordering::Acquire);
        if cur.is_null() { return None; }
        let next = unsafe { (*cur).next };
        self.head.store(next, Ordering::Release);
        self.len.fetch_sub(1, Ordering::Relaxed);
        let b = unsafe { Box::from_raw(cur) };
        Some(b.val)
    }
}

impl<T> Drop for Stack<T> {
    fn drop(&mut self) {
        let mut n = 0;
        let mut cur = *self.head.get_mut();
        while !cur.is_null() {
            let b = unsafe { Box::from_raw(cur) };
            cur = b.next;
            n += 1;
        }
        println!("stack dropped with {} nodes left", n);
    }
}

fn main() {
    let s = Arc::new(Stack::new());
    let hs: Vec<_> = (0..4).map(|t| {
        let s = s.clone();
        thread::spawn(move || {
            for i in 0..250 { s.push(t * 1000 + i); }
        })
    }).collect();
    for h in hs { h.join().unwrap(); }
    println!("len {}", s.len.load(Ordering::SeqCst));
    let mut all = vec![];
    for _ in 0..990 { all.push(s.pop().unwrap()); }
    all.sort();
    println!("popped {} distinct {}", all.len(), { let mut d = all.clone(); d.dedup(); d.len() });
    let per_thread: Vec<usize> = (0..4).map(|t| all.iter().filter(|&&v| v / 1000 == t).count()).collect();
    println!("popped per thread total {}", per_thread.iter().sum::<usize>());
    let single = Stack::new();
    for w in ["a", "b", "c"] { single.push(w.to_string()); }
    println!("{:?} {:?}", single.pop(), single.pop());
    drop(single);
    drop(s);
}
