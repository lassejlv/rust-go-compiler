// async fns returning impl Trait (closures, iterators, Display), async closures returned
// from functions (impl AsyncFn), and async trait methods returning impl Trait.
use std::fmt::Display;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

fn block_on<F: Future>(f: F) -> F::Output {
    let mut cx = Context::from_waker(Waker::noop());
    let mut f = std::pin::pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

struct Tick(u8);
impl Future for Tick {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.0 == 0 { Poll::Ready(()) } else { self.0 -= 1; Poll::Pending }
    }
}

async fn make_adder(n: i32) -> impl Fn(i32) -> i32 {
    Tick(2).await;
    move |x| x + n
}

async fn evens(limit: u32) -> impl Iterator<Item = u32> {
    Tick(1).await;
    (0..limit).filter(|x| x % 2 == 0)
}

async fn label(v: &[i32]) -> impl Display + '_ {
    Tick(1).await;
    struct L<'a>(&'a [i32]);
    impl Display for L<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "<{}>", self.0.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("|"))
        }
    }
    L(v)
}

async fn pair() -> (impl Display, impl Fn() -> String) {
    Tick(1).await;
    (42u8, || "closure".to_string())
}

async fn checked(x: i32) -> Result<impl Display, String> {
    Tick(1).await;
    if x >= 0 { Ok(x * 2) } else { Err(format!("negative {}", x)) }
}

fn multiplier(k: i32) -> impl AsyncFn(i32) -> i32 {
    async move |x| {
        Tick(1).await;
        x * k
    }
}

trait Source {
    async fn items(&self) -> impl Iterator<Item = String>;
    fn count(&self) -> impl Future<Output = usize> {
        async { 0 }
    }
}
struct Words(&'static str);
impl Source for Words {
    async fn items(&self) -> impl Iterator<Item = String> {
        Tick(1).await;
        self.0.split(' ').map(|s| s.to_string())
    }
    async fn count(&self) -> usize {
        self.items().await.count()
    }
}
struct Empty;
impl Source for Empty {
    async fn items(&self) -> impl Iterator<Item = String> {
        std::iter::empty()
    }
}

async fn describe<S: Source>(s: &S) -> String {
    let v: Vec<String> = s.items().await.collect();
    format!("{:?} count={}", v, s.count().await)
}

fn main() {
    let add = block_on(make_adder(7));
    println!("{}", add(3));
    println!("{:?}", block_on(evens(9)).collect::<Vec<_>>());
    let data = [1, 2, 3];
    println!("{}", block_on(label(&data)));
    let (d, c) = block_on(pair());
    println!("{} {}", d, c());
    match block_on(checked(4)) {
        Ok(v) => println!("ok {}", v),
        Err(e) => println!("err {}", e),
    }
    match block_on(checked(-1)) {
        Ok(v) => println!("ok {}", v),
        Err(e) => println!("err {}", e),
    }
    let m = multiplier(6);
    println!("{} {}", block_on(m(2)), block_on(m(5)));
    println!("{}", block_on(describe(&Words("a bb ccc"))));
    println!("{}", block_on(describe(&Empty)));
}
