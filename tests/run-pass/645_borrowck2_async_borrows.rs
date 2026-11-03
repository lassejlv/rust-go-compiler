// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p07_async_borrow_local {
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};
    fn block_on<F: Future>(f: F) -> F::Output {
        let mut f = pin!(f);
        let mut cx = Context::from_waker(Waker::noop());
        loop { if let Poll::Ready(v) = f.as_mut().poll(&mut cx) { return v; } }
    }
    async fn total(v: &[i32]) -> i32 { v.iter().sum() }
    async fn push_and_len(v: &mut Vec<i32>, x: i32) -> usize { v.push(x); v.len() }
    async fn run() -> String {
        let mut data = vec![1, 2, 3];
        let s = total(&data).await;
        let n = push_and_len(&mut data, s).await;
        let r = &data;
        let s2 = async { r.iter().max().copied() }.await;
        let mut name = String::from("x");
        async { name.push('y'); }.await;
        format!("{} {} {:?} {:?} {}", s, n, s2, data, name)
    }
    pub fn main() { println!("{}", block_on(run())); }
}

mod p53_async_trait_and_closures {
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};
    fn block_on<F: Future>(f: F) -> F::Output {
        let mut f = pin!(f);
        let mut cx = Context::from_waker(Waker::noop());
        loop { if let Poll::Ready(v) = f.as_mut().poll(&mut cx) { return v; } }
    }
    struct Db { rows: Vec<String> }
    impl Db {
        async fn find(&self, p: &str) -> Option<&String> { self.rows.iter().find(|r| r.starts_with(p)) }
        async fn add(&mut self, r: &str) -> usize { self.rows.push(r.to_string()); self.rows.len() }
    }
    async fn twice<F, Fut>(f: F) -> u32 where F: Fn(u32) -> Fut, Fut: Future<Output = u32> { f(1).await + f(2).await }
    pub fn main() {
        let out = block_on(async {
            let mut db = Db { rows: vec!["apple".into()] };
            let n = db.add("banana").await;
            let found = db.find("ban").await.cloned();
            let t = twice(|x| async move { x * 10 }).await;
            let mut acc = Vec::new();
            for i in 0..3 { let r = &mut acc; async { r.push(i) }.await; }
            format!("{} {:?} {} {:?}", n, found, t, acc)
        });
        println!("{}", out);
    }
}

fn main() {
    p07_async_borrow_local::main();
    p53_async_trait_and_closures::main();
}
