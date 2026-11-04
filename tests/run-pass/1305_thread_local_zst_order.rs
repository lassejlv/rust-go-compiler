// thread_local! values that are zero-sized (accessed repeatedly, with destructors), the
// LIFO destruction order of thread-locals, a deferred-callback runner, and try_with
// after destruction.
use std::cell::{Cell, RefCell};
use std::sync::Mutex;
use std::thread;

static EVENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());
fn ev(s: String) {
    EVENTS.lock().unwrap().push(s);
}

struct Marker;
impl Drop for Marker {
    fn drop(&mut self) {
        ev("drop Marker".into());
    }
}

struct Named(&'static str);
impl Drop for Named {
    fn drop(&mut self) {
        let other = LATE.try_with(|l| l.get()).map_err(|_| "destroyed");
        ev(format!("drop {} (LATE {:?})", self.0, other));
    }
}

struct RunDeferred;
impl Drop for RunDeferred {
    fn drop(&mut self) {
        let fs = DEFERRED.with(|d| std::mem::take(&mut *d.borrow_mut()));
        for f in fs {
            f();
        }
    }
}

thread_local! {
    static MARK: Marker = Marker;
    static UNIT: () = ();
    static FIRST: Named = Named("first");
    static LATE: Cell<u32> = Cell::new(7);
    static SECOND: Named = Named("second");
    static DEFERRED: RefCell<Vec<Box<dyn FnOnce()>>> = RefCell::new(Vec::new());
    static RUNNER: RunDeferred = RunDeferred;
}

fn main() {
    let h = thread::spawn(|| {
        for _ in 0..3 {
            MARK.with(|_| ());
            UNIT.with(|u| assert_eq!(*u, ()));
        }
        FIRST.with(|f| ev(format!("touch {}", f.0)));
        LATE.set(8);
        SECOND.with(|f| ev(format!("touch {}", f.0)));
        DEFERRED.with(|d| d.borrow_mut().push(Box::new(|| ev("deferred a".into()))));
        RUNNER.with(|_| ());
        DEFERRED.with(|d| d.borrow_mut().push(Box::new(|| ev("deferred b".into()))));
        ev("body done".into());
    });
    h.join().unwrap();
    for e in EVENTS.lock().unwrap().iter() {
        println!("{}", e);
    }
    EVENTS.lock().unwrap().clear();
    // a thread that only touches LATE after SECOND: SECOND sees LATE destroyed
    thread::spawn(|| {
        SECOND.with(|_| ());
        LATE.set(1);
    })
    .join()
    .unwrap();
    for e in EVENTS.lock().unwrap().iter() {
        println!("{}", e);
    }
    MARK.with(|_| ());
    MARK.with(|_| ());
    println!("main zst access ok");
}
