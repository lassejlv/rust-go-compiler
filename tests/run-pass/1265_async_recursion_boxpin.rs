// Recursive async via Box::pin, async tree traversal, and dyn Future trait objects.
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

struct YieldOnce(bool);
impl Future for YieldOnce {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.0 { Poll::Ready(()) } else { self.0 = true; Poll::Pending }
    }
}

fn fib(n: u32) -> Pin<Box<dyn Future<Output = u64>>> {
    Box::pin(async move {
        if n < 2 {
            n as u64
        } else {
            YieldOnce(false).await;
            fib(n - 1).await + fib(n - 2).await
        }
    })
}

async fn fact(n: u64) -> u64 {
    if n == 0 {
        1
    } else {
        n * Box::pin(fact(n - 1)).await
    }
}

#[derive(Debug)]
enum Tree {
    Leaf(i32),
    Node(Box<Tree>, Box<Tree>),
}

fn sum_tree<'a>(t: &'a Tree, depth: usize, log: &'a mut Vec<String>) -> Pin<Box<dyn Future<Output = i32> + 'a>> {
    Box::pin(async move {
        match t {
            Tree::Leaf(v) => {
                log.push(format!("{}leaf {}", " ".repeat(depth), v));
                *v
            }
            Tree::Node(l, r) => {
                log.push(format!("{}node", " ".repeat(depth)));
                let a = sum_tree(l, depth + 1, log).await;
                YieldOnce(false).await;
                let b = sum_tree(r, depth + 1, log).await;
                a + b
            }
        }
    })
}

fn main() {
    println!("fib(15) = {}", block_on(fib(15)));
    println!("fact(15) = {}", block_on(fact(15)));
    let t = Tree::Node(
        Box::new(Tree::Node(Box::new(Tree::Leaf(1)), Box::new(Tree::Leaf(2)))),
        Box::new(Tree::Node(Box::new(Tree::Leaf(3)), Box::new(Tree::Node(Box::new(Tree::Leaf(4)), Box::new(Tree::Leaf(5)))))),
    );
    let mut log = Vec::new();
    let s = block_on(sum_tree(&t, 0, &mut log));
    for l in &log {
        println!("{}", l);
    }
    println!("sum = {}", s);
    let futs: Vec<Pin<Box<dyn Future<Output = String>>>> = vec![
        Box::pin(async { "a".to_string() }),
        Box::pin(async { YieldOnce(false).await; "b".repeat(3) }),
        Box::pin(std::future::ready("c".to_string())),
    ];
    let mut out = Vec::new();
    for f in futs {
        out.push(block_on(f));
    }
    println!("{:?}", out);
}
