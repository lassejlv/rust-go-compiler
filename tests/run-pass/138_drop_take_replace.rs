use std::mem;

struct Noisy {
    name: String,
}

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.name);
    }
}

fn noisy(s: &str) -> Noisy {
    Noisy { name: s.to_string() }
}

#[derive(Default, Debug)]
struct Stats {
    count: u32,
    names: Vec<String>,
}

struct Holder {
    current: Option<Noisy>,
    history: Vec<Noisy>,
}

impl Holder {
    fn set(&mut self, n: Noisy) {
        if let Some(old) = self.current.replace(n) {
            println!("replaced {}", old.name);
            self.history.push(old);
        }
    }
    fn clear(&mut self) -> Option<Noisy> {
        self.current.take()
    }
}

enum State {
    Idle,
    Running(Noisy),
    Done(String),
}

fn advance(s: &mut State) {
    let old = mem::replace(s, State::Idle);
    *s = match old {
        State::Idle => State::Running(noisy("job")),
        State::Running(n) => State::Done(n.name.clone()),
        State::Done(d) => {
            println!("already done: {}", d);
            State::Done(d)
        }
    };
}

fn main() {
    let mut stats = Stats { count: 3, names: vec!["a".into(), "b".into()] };
    let taken = mem::take(&mut stats);
    println!("taken {:?}", taken);
    println!("left {:?}", stats);

    let mut v = vec![1, 2, 3];
    let old = mem::replace(&mut v, vec![9]);
    println!("{:?} {:?}", old, v);

    let mut s = String::from("hello");
    let t = mem::take(&mut s);
    println!("[{}] [{}]", t, s);

    println!("-- holder");
    let mut h = Holder { current: None, history: Vec::new() };
    h.set(noisy("first"));
    h.set(noisy("second"));
    h.set(noisy("third"));
    let c = h.clear();
    println!("cleared {:?}", c.as_ref().map(|n| n.name.clone()));
    println!("current is none: {}", h.current.is_none());
    drop(c);
    println!("history len {}", h.history.len());
    let first = h.history.remove(0);
    println!("removed {}", first.name);
    mem::drop(first);

    println!("-- state");
    let mut st = State::Idle;
    for _ in 0..4 {
        advance(&mut st);
        match &st {
            State::Idle => println!("idle"),
            State::Running(n) => println!("running {}", n.name),
            State::Done(d) => println!("done {}", d),
        }
    }

    println!("-- swap");
    let mut a = noisy("A");
    let mut b = noisy("B");
    mem::swap(&mut a, &mut b);
    println!("a={} b={}", a.name, b.name);
    let mut opt = Some(noisy("opt"));
    if let Some(n) = opt.as_mut() {
        n.name.push_str("-modified");
    }
    let inner = opt.take().unwrap();
    println!("got {}", inner.name);
    let _ = mem::replace(&mut a, noisy("A2"));
    println!("-- end of main");
}
