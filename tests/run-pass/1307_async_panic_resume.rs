// Panics inside polled futures: catch_unwind around poll, resuming after completion or
// after a panic (payload messages), drops of locals during unwinding out of a future.
use std::future::Future;
use std::panic::{self, AssertUnwindSafe};
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

struct Noisy(&'static str);
impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Yield(bool);
impl Future for Yield {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.0 { Poll::Ready(()) } else { self.0 = true; Poll::Pending }
    }
}

async fn may_fail(fail_at: u32) -> u32 {
    let _guard = Noisy("guard");
    for i in 0..3 {
        let _step = Noisy(if i == 0 { "step0" } else if i == 1 { "step1" } else { "step2" });
        Yield(false).await;
        if i == fail_at {
            panic!("failed at step {}", i);
        }
    }
    99
}

fn msg(e: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = e.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = e.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "?".into()
    }
}

fn main() {
    panic::set_hook(Box::new(|_| {}));
    let mut cx = Context::from_waker(Waker::noop());

    let mut f = Box::pin(may_fail(1));
    for round in 0..4 {
        let r = panic::catch_unwind(AssertUnwindSafe(|| f.as_mut().poll(&mut cx)));
        match r {
            Ok(p) => println!("round {} -> {:?}", round, p),
            Err(e) => println!("round {} panicked: {}", round, msg(e)),
        }
    }
    drop(f);
    println!("--");
    let mut g = Box::pin(may_fail(10));
    loop {
        match g.as_mut().poll(&mut cx) {
            Poll::Ready(v) => {
                println!("ready {}", v);
                break;
            }
            Poll::Pending => println!("pending"),
        }
    }
    let r = panic::catch_unwind(AssertUnwindSafe(|| g.as_mut().poll(&mut cx)));
    println!("poll after completion: {}", msg(r.unwrap_err()));
    let mut blk = Box::pin(async { 5 });
    let _ = blk.as_mut().poll(&mut cx);
    let r = panic::catch_unwind(AssertUnwindSafe(|| blk.as_mut().poll(&mut cx)));
    println!("block after completion: {}", msg(r.unwrap_err()));
    let _ = panic::take_hook();
    println!("end");
}
