use std::mem;

#[derive(Debug)]
enum List { Nil, Cons(i32, Box<List>) }
use List::{Cons, Nil};

impl List {
    fn from_slice(v: &[i32]) -> List { v.iter().rev().fold(Nil, |acc, &x| Cons(x, Box::new(acc))) }
    fn reverse(self) -> List {
        let mut prev = Nil;
        let mut cur = self;
        while let Cons(v, mut next) = cur {
            cur = mem::replace(&mut *next, Nil);
            *next = prev;
            prev = Cons(v, next);
        }
        prev
    }
    fn to_vec(&self) -> Vec<i32> { let mut v = vec![]; let mut c = self; while let Cons(x, n) = c { v.push(*x); c = n; } v }
    fn push_back(&mut self, x: i32) {
        let mut cur = self;
        while let Cons(_, next) = cur { cur = next; }
        *cur = Cons(x, Box::new(Nil));
    }
    fn remove_all(&mut self, target: i32) {
        let mut cur = self;
        loop {
            match cur {
                Nil => break,
                Cons(v, next) if *v == target => { *cur = mem::replace(&mut **next, Nil); }
                Cons(_, next) => { cur = next; }
            }
        }
    }
}

#[derive(Debug, Default)]
struct Buffer { data: Vec<u8>, name: String, flushed: usize }

impl Buffer {
    fn flush(&mut self) -> Vec<u8> { self.flushed += 1; mem::take(&mut self.data) }
    fn rename(&mut self, n: &str) -> String { mem::replace(&mut self.name, n.to_string()) }
}

#[derive(Debug)]
enum State { Idle, Running { ticks: u32, log: Vec<String> }, Done(Vec<String>) }

fn step(s: &mut State) {
    *s = match mem::replace(s, State::Idle) {
        State::Idle => State::Running { ticks: 0, log: vec!["start".into()] },
        State::Running { ticks, mut log } if ticks >= 2 => { log.push("stop".into()); State::Done(log) }
        State::Running { ticks, mut log } => { log.push(format!("tick{}", ticks)); State::Running { ticks: ticks + 1, log } }
        State::Done(l) => State::Done(l),
    };
}

fn main() {
    let l = List::from_slice(&[1, 2, 3, 4, 5]);
    println!("{:?}", l.to_vec());
    let mut r = l.reverse();
    println!("{:?}", r.to_vec());
    r.push_back(3); r.push_back(9);
    println!("{:?}", r.to_vec());
    r.remove_all(3);
    println!("{:?}", r.to_vec());
    let mut a = vec![1, 2];
    let mut b = vec![9, 8, 7];
    mem::swap(&mut a, &mut b);
    println!("{:?} {:?}", a, b);
    let (mut x, mut y) = (String::from("left"), String::from("right"));
    mem::swap(&mut x, &mut y);
    println!("{} {}", x, y);
    let mut arr = [10, 20, 30, 40];
    let (lo, hi) = arr.split_at_mut(2);
    mem::swap(&mut lo[0], &mut hi[1]);
    arr.swap(1, 2);
    println!("{:?}", arr);
    let mut buf = Buffer { data: vec![1, 2, 3], name: "a".into(), flushed: 0 };
    let out = buf.flush();
    let old = buf.rename("b");
    buf.data.extend_from_slice(b"hi");
    println!("{:?} {} {:?}", out, old, buf);
    let taken = mem::take(&mut buf);
    println!("{:?} {:?}", taken, buf);
    let mut st = State::Idle;
    for _ in 0..6 { step(&mut st); println!("{:?}", st); }
    let mut opt = Some(5);
    let t = opt.take();
    println!("{:?} {:?}", t, opt);
    let mut opt2 = Some(1);
    let old2 = opt2.replace(2);
    println!("{:?} {:?}", old2, opt2);
    let mut v = vec![Some("a"), None, Some("c")];
    let collected: Vec<&str> = v.iter_mut().filter_map(|o| o.take()).collect();
    println!("{:?} {:?}", collected, v);
    let mut s = String::from("abc");
    let s2 = mem::take(&mut s);
    println!("[{}] [{}] {}", s, s2, mem::size_of::<Option<Box<i32>>>() == mem::size_of::<Box<i32>>());
    let mut n = 5;
    let r1 = &mut n;
    let prev = mem::replace(r1, 50);
    *r1 += prev;
    println!("{}", n);
    println!("{} {} {} {}", mem::size_of::<u128>(), mem::size_of::<(u8, u32)>(), mem::size_of::<[u16; 5]>(), mem::align_of::<u64>());
    println!("{} {} {}", mem::size_of::<Option<u32>>(), mem::size_of::<Option<&u8>>(), mem::size_of::<()>());
    let d1 = mem::discriminant(&State::Idle);
    let d2 = mem::discriminant(&State::Done(vec![]));
    let d3 = mem::discriminant(&State::Done(vec!["x".into()]));
    println!("{} {}", d1 == d2, d2 == d3);
    let boxed = Box::new([1u8; 3]);
    let s3 = mem::ManuallyDrop::new(String::from("manual"));
    println!("{:?} {}", boxed, *s3);
    let _ = mem::ManuallyDrop::into_inner(s3);
    struct Noisy(i32);
    impl Drop for Noisy { fn drop(&mut self) { println!("drop noisy {}", self.0); } }
    let nz = Noisy(1);
    mem::forget(nz);
    let mut nz2 = Noisy(2);
    let nz3 = mem::replace(&mut nz2, Noisy(3));
    println!("replaced; old is {}", nz3.0);
    drop(nz3);
    println!("end");
}
