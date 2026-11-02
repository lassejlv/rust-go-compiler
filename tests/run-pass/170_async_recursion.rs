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

fn fib(n: u32) -> Pin<Box<dyn Future<Output = u64>>> {
    Box::pin(async move {
        if n < 2 {
            n as u64
        } else {
            fib(n - 1).await + fib(n - 2).await
        }
    })
}

#[derive(Debug)]
enum Tree {
    Leaf(i32),
    Node(Box<Tree>, Box<Tree>),
}

fn sum_tree<'a>(t: &'a Tree, depth: usize) -> Pin<Box<dyn Future<Output = i32> + 'a>> {
    Box::pin(async move {
        match t {
            Tree::Leaf(v) => {
                println!("{}leaf {}", "  ".repeat(depth), v);
                *v
            }
            Tree::Node(l, r) => {
                println!("{}node", "  ".repeat(depth));
                sum_tree(l, depth + 1).await + sum_tree(r, depth + 1).await
            }
        }
    })
}

// Recursion through async fn directly with Box::pin at the call site (Rust 1.77+).
async fn count_down(n: u32, acc: &mut Vec<u32>) {
    acc.push(n);
    if n > 0 {
        Box::pin(count_down(n - 1, acc)).await;
    }
}

fn hanoi(n: u32, from: char, to: char, via: char, moves: &mut Vec<String>) -> Pin<Box<dyn Future<Output = ()> + '_>> {
    Box::pin(async move {
        if n == 0 {
            return;
        }
        hanoi(n - 1, from, via, to, moves).await;
        moves.push(format!("{}{}", from, to));
        hanoi(n - 1, via, to, from, moves).await;
    })
}

fn main() {
    for n in [0, 1, 10, 20] {
        println!("fib({}) = {}", n, block_on(fib(n)));
    }
    let t = Tree::Node(
        Box::new(Tree::Leaf(1)),
        Box::new(Tree::Node(Box::new(Tree::Leaf(2)), Box::new(Tree::Node(Box::new(Tree::Leaf(3)), Box::new(Tree::Leaf(4)))))),
    );
    println!("sum = {}", block_on(sum_tree(&t, 0)));
    let mut acc = Vec::new();
    block_on(count_down(5, &mut acc));
    println!("{:?}", acc);
    let mut moves = Vec::new();
    block_on(hanoi(4, 'A', 'C', 'B', &mut moves));
    println!("{} moves: {}", moves.len(), moves.join(" "));
}
