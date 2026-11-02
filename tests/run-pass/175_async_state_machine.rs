use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

// A oneshot-like mailbox shared between tasks.
struct Mailbox {
    queue: RefCell<VecDeque<String>>,
}

struct Recv<'a> {
    mb: &'a Mailbox,
}

impl<'a> Future for Recv<'a> {
    type Output = String;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<String> {
        match self.mb.queue.borrow_mut().pop_front() {
            Some(m) => Poll::Ready(m),
            None => Poll::Pending,
        }
    }
}

struct YieldNow(bool);
impl Future for YieldNow {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            return Poll::Ready(());
        }
        self.0 = true;
        Poll::Pending
    }
}

async fn server(inbox: Rc<Mailbox>, outbox: Rc<Mailbox>, log: Rc<RefCell<Vec<String>>>) {
    loop {
        let req = Recv { mb: &inbox }.await;
        log.borrow_mut().push(format!("server got {}", req));
        if req == "quit" {
            outbox.queue.borrow_mut().push_back("bye".to_string());
            break;
        }
        let resp = match req.split_once(' ') {
            Some(("add", rest)) => {
                let s: i32 = rest.split(' ').filter_map(|x| x.parse::<i32>().ok()).sum();
                format!("sum={}", s)
            }
            Some(("rev", rest)) => rest.chars().rev().collect(),
            _ => format!("unknown:{}", req),
        };
        outbox.queue.borrow_mut().push_back(resp);
    }
}

async fn client(to_server: Rc<Mailbox>, from_server: Rc<Mailbox>, log: Rc<RefCell<Vec<String>>>) -> usize {
    let requests = ["add 1 2 3", "rev hello", "ping", "add 10 -4", "quit"];
    let mut n = 0;
    for r in requests {
        to_server.queue.borrow_mut().push_back(r.to_string());
        YieldNow(false).await;
        let resp = Recv { mb: &from_server }.await;
        log.borrow_mut().push(format!("client got {}", resp));
        n += 1;
    }
    n
}

fn main() {
    let a = Rc::new(Mailbox { queue: RefCell::new(VecDeque::new()) });
    let b = Rc::new(Mailbox { queue: RefCell::new(VecDeque::new()) });
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut srv = Box::pin(server(a.clone(), b.clone(), log.clone()));
    let mut cli = Box::pin(client(a.clone(), b.clone(), log.clone()));
    let mut cx = Context::from_waker(Waker::noop());
    let mut srv_done = false;
    let mut cli_result = None;
    let mut rounds = 0;
    while !(srv_done && cli_result.is_some()) {
        rounds += 1;
        if cli_result.is_none() {
            if let Poll::Ready(n) = cli.as_mut().poll(&mut cx) {
                cli_result = Some(n);
            }
        }
        if !srv_done {
            if let Poll::Ready(()) = srv.as_mut().poll(&mut cx) {
                srv_done = true;
            }
        }
        if rounds > 100 {
            println!("stuck");
            break;
        }
    }
    for l in log.borrow().iter() {
        println!("{}", l);
    }
    println!("client handled {:?} in {} rounds", cli_result, rounds);
    println!("size_of Recv future == size_of pointer: {}", std::mem::size_of::<Recv>() == std::mem::size_of::<usize>());
}
