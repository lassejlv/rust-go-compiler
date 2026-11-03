// Temporaries and locals in async bodies: temporaries that live across an
// await within one statement, scrutinee temporaries of match/if-let around
// awaits, shadowed locals, loop-scoped locals, and the order in which
// everything is dropped relative to awaits and to completion.
use std::cell::RefCell;
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

thread_local! { static EV: RefCell<Vec<String>> = RefCell::new(vec![]); }
fn ev(s: String) { EV.with(|e| e.borrow_mut().push(s)); }
fn dump(t: &str) { let v = EV.with(|e| std::mem::take(&mut *e.borrow_mut())); println!("{:<14}{}", t, v.join(" ")); }

struct D(&'static str);
impl Drop for D { fn drop(&mut self) { ev(format!("~{}", self.0)); } }
impl D { fn len(&self) -> usize { self.0.len() } fn opt(&self) -> Option<usize> { Some(self.0.len()) } }

struct Tick(bool);
impl Future for Tick {
    type Output = ();
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 { Poll::Ready(()) } else { self.0 = true; ev("pending".into()); cx.waker().wake_by_ref(); Poll::Pending }
    }
}
fn tick() -> Tick { Tick(false) }

async fn measure(d: &D) -> usize { tick().await; d.len() }

fn run<F: Future>(f: F) -> F::Output {
    let mut f = pin!(f);
    let mut cx = Context::from_waker(Waker::noop());
    loop { if let Poll::Ready(v) = f.as_mut().poll(&mut cx) { return v; } }
}

fn main() {
    let n = run(async {
        let a = measure(&D("temp-across")).await + 1;
        ev(format!("a={}", a));
        a
    });
    ev(format!("n={}", n));
    dump("temp across:");

    run(async {
        match D("scrut").opt() {
            Some(k) => { tick().await; ev(format!("arm {}", k)); }
            None => {}
        }
        ev("after match".into());
    });
    dump("match scrut:");

    run(async {
        let x = D("shadow1");
        tick().await;
        let x = D(if x.len() > 3 { "shadow2" } else { "short" });
        tick().await;
        ev(format!("using {}", x.0));
    });
    dump("shadowing:");

    run(async {
        for name in ["L0", "L1"] {
            let _g = D(name);
            tick().await;
            ev(format!("iter {}", name));
        }
        ev("loop done".into());
    });
    dump("loop locals:");

    let r = run(async {
        let _outer = D("outer");
        let inner = async {
            let _i = D("inner-local");
            tick().await;
            7
        };
        let v = inner.await;
        ev(format!("inner returned {}", v));
        v * 2
    });
    ev(format!("r={}", r));
    dump("nested:");

    run(async {
        let v = vec![D("v0"), D("v1")];
        let total: usize = {
            let mut t = 0;
            for d in &v { t += measure(d).await; }
            t
        };
        drop(v);
        ev(format!("total {}", total));
    });
    dump("vec borrow:");

    run(async {
        if let Some(k) = D("iflet").opt() {
            tick().await;
            ev(format!("got {}", k));
        }
        ev("after if-let".into());
    });
    dump("if-let scrut:");
}
