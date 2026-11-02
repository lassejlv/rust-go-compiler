use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

type Task = Pin<Box<dyn Future<Output = ()>>>;

struct Executor {
    queue: VecDeque<(usize, Task)>,
    next_id: usize,
}

impl Executor {
    fn new() -> Self {
        Executor { queue: VecDeque::new(), next_id: 0 }
    }
    fn spawn(&mut self, f: impl Future<Output = ()> + 'static) {
        self.queue.push_back((self.next_id, Box::pin(f)));
        self.next_id += 1;
    }
    fn run(&mut self) -> usize {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(&waker);
        let mut polls = 0;
        while let Some((id, mut task)) = self.queue.pop_front() {
            polls += 1;
            match task.as_mut().poll(&mut cx) {
                Poll::Ready(()) => println!("[exec] task {} complete", id),
                Poll::Pending => self.queue.push_back((id, task)),
            }
        }
        polls
    }
}

struct Yield(bool);
impl Future for Yield {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            Poll::Pending
        }
    }
}
fn yield_now() -> Yield {
    Yield(false)
}

async fn worker(name: &'static str, steps: u32, log: Rc<RefCell<Vec<String>>>) {
    for i in 0..steps {
        log.borrow_mut().push(format!("{}:{}", name, i));
        yield_now().await;
    }
    log.borrow_mut().push(format!("{}:end", name));
}

struct Channel<T> {
    buf: RefCell<VecDeque<T>>,
    closed: RefCell<bool>,
}

async fn recv<T>(ch: &Channel<T>) -> Option<T> {
    loop {
        if let Some(v) = ch.buf.borrow_mut().pop_front() {
            return Some(v);
        }
        if *ch.closed.borrow() {
            return None;
        }
        yield_now().await;
    }
}

fn main() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut ex = Executor::new();
    ex.spawn(worker("a", 3, log.clone()));
    ex.spawn(worker("b", 1, log.clone()));
    ex.spawn(worker("c", 2, log.clone()));
    let polls = ex.run();
    println!("polls {}", polls);
    println!("{}", log.borrow().join(" "));

    let ch = Rc::new(Channel { buf: RefCell::new(VecDeque::new()), closed: RefCell::new(false) });
    let results = Rc::new(RefCell::new(Vec::new()));
    let mut ex = Executor::new();
    {
        let ch = ch.clone();
        ex.spawn(async move {
            for i in 1..=5 {
                ch.buf.borrow_mut().push_back(i * i);
                println!("produced {}", i * i);
                yield_now().await;
                yield_now().await;
            }
            *ch.closed.borrow_mut() = true;
        });
    }
    {
        let ch = ch.clone();
        let results = results.clone();
        ex.spawn(async move {
            while let Some(v) = recv(&ch).await {
                println!("consumed {}", v);
                results.borrow_mut().push(v);
            }
            println!("consumer saw close");
        });
    }
    let polls = ex.run();
    println!("polls {} results {:?}", polls, results.borrow());
}
