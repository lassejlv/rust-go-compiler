use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

struct Noop;
impl Wake for Noop {
    fn wake(self: Arc<Self>) {}
}

fn poll3<F: Future>(name: &str, f: F)
where
    F::Output: std::fmt::Debug,
{
    let waker = Waker::from(Arc::new(Noop));
    let mut cx = Context::from_waker(&waker);
    let mut f = Box::pin(f);
    for i in 0..3 {
        let r = catch_unwind(AssertUnwindSafe(|| f.as_mut().poll(&mut cx)));
        match r {
            Ok(p) => println!("{} {}: {:?}", name, i, p),
            Err(e) => println!(
                "{} {}: panic {:?} {:?}",
                name,
                i,
                e.downcast_ref::<&str>(),
                e.downcast_ref::<String>()
            ),
        }
    }
}

async fn done() -> u8 {
    7
}

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let loc = info.location().unwrap();
        println!("  hook: panicked at line {} col {}", loc.line(), loc.column());
    }));
    poll3("fn-complete", done());
    poll3("block-complete", async { 1 });
    let c = async |x: u32| -> u32 {
        if x > 0 {
            panic!("closure {}", x);
        }
        x
    };
    poll3("closure-panic", c(3));
    poll3("closure-complete", c(0));
    poll3("block-move-panic", async move { if true { panic!("m") } else { 0 } });
}
