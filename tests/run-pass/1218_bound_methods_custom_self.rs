// Methods of where-clause traits that are not imported, called through
// custom receivers (Pin<&mut Self>, Box<Self>, Rc<Self>), RPITIT and
// async fn in traits, and a user trait method named `call` on closures.

mod traits {
    use std::pin::Pin;
    use std::rc::Rc;

    pub trait Machine {
        fn step(self: Pin<&mut Self>) -> u32;
        fn finish(self: Box<Self>) -> String;
        fn shared(self: Rc<Self>) -> usize;
        fn name(&self) -> &'static str;
    }

    pub struct Counter(pub u32);
    impl Machine for Counter {
        fn step(mut self: Pin<&mut Self>) -> u32 {
            self.0 += 1;
            self.0
        }
        fn finish(self: Box<Self>) -> String {
            format!("counter done at {}", self.0)
        }
        fn shared(self: Rc<Self>) -> usize {
            Rc::strong_count(&self)
        }
        fn name(&self) -> &'static str {
            "counter"
        }
    }

    pub struct Doubler(pub u64);
    impl Machine for Doubler {
        fn step(mut self: Pin<&mut Self>) -> u32 {
            self.0 *= 2;
            self.0 as u32
        }
        fn finish(self: Box<Self>) -> String {
            format!("doubler done at {}", self.0)
        }
        fn shared(self: Rc<Self>) -> usize {
            Rc::strong_count(&self) * 10
        }
        fn name(&self) -> &'static str {
            "doubler"
        }
    }
}

// `traits::Machine` is deliberately not imported: its methods are found
// through the bounds.
fn drive<M: traits::Machine + Unpin>(mut m: M, n: usize) -> (Vec<u32>, String) {
    let mut out = vec![];
    for _ in 0..n {
        out.push(std::pin::Pin::new(&mut m).step());
    }
    let label = m.name();
    let b = Box::new(m);
    (out, format!("{}: {}", label, b.finish()))
}

fn share<M: traits::Machine>(m: M) -> usize {
    let rc = std::rc::Rc::new(m);
    let other = rc.clone();
    let n = rc.shared();
    drop(other);
    n
}

fn poll_all<F: std::future::Future>(f: F) -> F::Output {
    use std::task::{Context, Poll, Waker};
    let w = Waker::noop();
    let mut cx = Context::from_waker(&w);
    let mut f = std::pin::pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

trait Store {
    fn items(&self) -> impl Iterator<Item = u32> + '_;
    fn total(&self) -> u32 {
        self.items().sum()
    }
    async fn fetch(&self, i: usize) -> Option<u32>;
}

struct VecStore(Vec<u32>);
impl Store for VecStore {
    fn items(&self) -> impl Iterator<Item = u32> + '_ {
        self.0.iter().copied().filter(|x| x % 2 == 1)
    }
    async fn fetch(&self, i: usize) -> Option<u32> {
        self.0.get(i).copied()
    }
}

struct RangeStore(u32, u32);
impl Store for RangeStore {
    fn items(&self) -> impl Iterator<Item = u32> + '_ {
        self.0..self.1
    }
    async fn fetch(&self, i: usize) -> Option<u32> {
        let base = async { self.0 }.await;
        Some(base + i as u32)
    }
}

fn describe<S: Store>(s: &S) -> String {
    format!("{:?} total={} first={:?}", s.items().collect::<Vec<_>>(), s.total(), poll_all(s.fetch(0)))
}

trait Callback {
    fn call(&self, x: i32) -> i32;
}
impl<F: Fn(i32) -> i32> Callback for F {
    fn call(&self, x: i32) -> i32 {
        self(x) * 10
    }
}

fn via_bound<F: Fn(i32) -> i32>(f: F) -> i32 {
    f(1) + Callback::call(&f, 1)
}

fn main() {
    println!("{:?}", drive(traits::Counter(0), 3));
    println!("{:?}", drive(traits::Doubler(1), 4));
    println!("{} {}", share(traits::Counter(5)), share(traits::Doubler(5)));

    println!("{}", describe(&VecStore(vec![1, 2, 3, 5])));
    println!("{}", describe(&RangeStore(3, 7)));
    println!("{:?}", poll_all(VecStore(vec![]).fetch(3)));

    let k = 4;
    let add = |x: i32| x + k;
    println!("{} {}", add.call(1), (|x: i32| x - 1).call(0));
    println!("{}", via_bound(|x| x * 3));
    let cbs: Vec<&dyn Callback> = vec![&add, &|x: i32| -x];
    println!("{:?}", cbs.iter().map(|c| c.call(2)).collect::<Vec<_>>());
}
