// Awaits in unusual positions: args, struct literals, match scrutinees, loop conditions,
// `?` inside async, labeled breaks, let-else, early returns, while let with a hand-written stream.
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

struct Yield<T>(Option<T>, bool);
impl<T: Unpin> Future for Yield<T> {
    type Output = T;
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<T> {
        if self.1 {
            Poll::Ready(self.0.take().unwrap())
        } else {
            self.1 = true;
            Poll::Pending
        }
    }
}
fn later<T: Unpin>(v: T) -> Yield<T> {
    Yield(Some(v), false)
}

struct Stream {
    items: Vec<i32>,
}
impl Stream {
    async fn next(&mut self) -> Option<i32> {
        later(()).await;
        if self.items.is_empty() { None } else { Some(self.items.remove(0)) }
    }
}

#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}

async fn parse(s: &str) -> Result<i32, String> {
    later(()).await;
    s.parse::<i32>().map_err(|e| format!("{}: {}", s, e))
}

async fn sum_all(xs: &[&str]) -> Result<i32, String> {
    let mut t = 0;
    for x in xs {
        t += parse(x).await?;
    }
    Ok(t)
}

fn add(a: i32, b: i32) -> i32 {
    a * 10 + b
}

async fn find_first_neg(xs: Vec<i32>) -> Option<usize> {
    for (i, x) in xs.into_iter().enumerate() {
        if later(x).await < 0 {
            return Some(i);
        }
    }
    None
}

async fn classify(v: Option<i32>) -> &'static str {
    let Some(x) = later(v).await else {
        return "none";
    };
    match later(x % 3).await {
        0 => "fizz",
        1 if later(x > 10).await => "big-one",
        1 => "one",
        _ => "two",
    }
}

fn main() {
    let r = block_on(async {
        let a = add(later(1).await, later(2).await);
        let p = Point { x: later(3).await, y: later(4).await + a };
        let arr = [later(5).await, later(6).await];
        let tup = (later("t").await, later(7u8).await);
        (a, p, arr, tup)
    });
    println!("{:?}", r);

    println!("{:?}", block_on(sum_all(&["1", "2", "39"])));
    println!("{:?}", block_on(sum_all(&["1", "zz", "3"])));
    println!("{:?} {:?}", block_on(find_first_neg(vec![3, 4, -1, 2])), block_on(find_first_neg(vec![1])));
    for v in [None, Some(3), Some(4), Some(13), Some(5)] {
        println!("{:?} -> {}", v, block_on(classify(v)));
    }

    let out = block_on(async {
        let mut s = Stream { items: vec![1, 2, 3, 4, 5, 6] };
        let mut acc = vec![];
        while let Some(x) = s.next().await {
            if x == 5 {
                break;
            }
            acc.push(x * x);
        }
        let mut count = 0;
        'outer: loop {
            loop {
                count += later(1).await;
                if count > 3 {
                    break 'outer;
                }
                if count % 2 == 0 {
                    continue 'outer;
                }
            }
        }
        let v = if later(true).await { "yes" } else { "no" };
        let blockval = 'b: {
            if later(count).await == 4 {
                break 'b count * 2;
            }
            0
        };
        (acc, count, v, s.items, blockval)
    });
    println!("{:?}", out);

    // async block with explicit return type through ?
    let res: Result<i32, String> = block_on(async {
        let a = parse("10").await?;
        let b = parse("x").await.unwrap_or(5);
        Ok::<i32, String>(a + b)
    });
    println!("{:?}", res);
}
