// Observer/subscription pattern with Weak references: an EventEmitter holding Weak<RefCell<..>>
// subscribers, Subscription handles that unsubscribe on Drop, and `detach()` to keep them alive.
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

type Callback<E> = Box<dyn FnMut(&E) -> bool>; // returns false to auto-unsubscribe

struct SubscriberSet<E> {
    inner: Rc<RefCell<BTreeMap<usize, Callback<E>>>>,
    next: Cell<usize>,
}

#[must_use]
struct Subscription {
    unsubscribe: Option<Box<dyn FnOnce()>>,
}

impl Subscription {
    fn detach(mut self) {
        self.unsubscribe.take();
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(f) = self.unsubscribe.take() {
            f();
        }
    }
}

impl<E: 'static> SubscriberSet<E> {
    fn new() -> Self {
        SubscriberSet { inner: Rc::new(RefCell::new(BTreeMap::new())), next: Cell::new(0) }
    }
    fn subscribe(&self, cb: impl FnMut(&E) -> bool + 'static) -> Subscription {
        let id = self.next.get();
        self.next.set(id + 1);
        self.inner.borrow_mut().insert(id, Box::new(cb));
        let weak: Weak<RefCell<BTreeMap<usize, Callback<E>>>> = Rc::downgrade(&self.inner);
        Subscription {
            unsubscribe: Some(Box::new(move || {
                if let Some(set) = weak.upgrade() {
                    set.borrow_mut().remove(&id);
                }
            })),
        }
    }
    fn emit(&self, e: &E) -> usize {
        // Take callbacks out so callbacks can subscribe re-entrantly.
        let mut cbs = std::mem::take(&mut *self.inner.borrow_mut());
        let mut called = 0;
        cbs.retain(|_, cb| {
            called += 1;
            cb(e)
        });
        let mut inner = self.inner.borrow_mut();
        for (k, v) in cbs {
            inner.insert(k, v);
        }
        called
    }
    fn len(&self) -> usize {
        self.inner.borrow().len()
    }
}

#[derive(Debug, Clone)]
enum BufferEvent {
    Edited { len: usize },
    Saved(String),
    Closed,
}

struct Buffer {
    text: String,
    events: SubscriberSet<BufferEvent>,
}

impl Buffer {
    fn edit(&mut self, s: &str) {
        self.text.push_str(s);
        let n = self.events.emit(&BufferEvent::Edited { len: self.text.len() });
        println!("  (edit notified {})", n);
    }
    fn save(&self, path: &str) {
        self.events.emit(&BufferEvent::Saved(path.into()));
    }
}

struct StatusBar {
    message: String,
    edits: u32,
}

fn main() {
    let mut buffer = Buffer { text: String::new(), events: SubscriberSet::new() };
    let status = Rc::new(RefCell::new(StatusBar { message: String::new(), edits: 0 }));

    // Subscriber holds only a Weak to the status bar so it doesn't keep it alive.
    let weak_status = Rc::downgrade(&status);
    let sub_status = buffer.events.subscribe(move |e| {
        let Some(s) = weak_status.upgrade() else {
            println!("  status bar gone; unsubscribing");
            return false;
        };
        let mut s = s.borrow_mut();
        match e {
            BufferEvent::Edited { len } => {
                s.edits += 1;
                s.message = format!("{} chars", len);
            }
            BufferEvent::Saved(p) => s.message = format!("saved to {}", p),
            BufferEvent::Closed => s.message = "closed".into(),
        }
        true
    });

    let log = Rc::new(RefCell::new(Vec::<String>::new()));
    let l2 = log.clone();
    let sub_log = buffer.events.subscribe(move |e| {
        l2.borrow_mut().push(format!("{:?}", e));
        true
    });

    let once_count = Rc::new(Cell::new(0));
    let oc = once_count.clone();
    buffer.events.subscribe(move |_| {
        oc.set(oc.get() + 1);
        false // fire once
    })
    .detach();

    println!("subscribers: {}", buffer.events.len());
    buffer.edit("hello");
    println!("status: {:?} edits={}", status.borrow().message, status.borrow().edits);
    println!("subscribers after once: {} (once fired {}x)", buffer.events.len(), once_count.get());
    buffer.edit(" world");
    buffer.save("/tmp/a.txt");
    println!("status: {:?} edits={}", status.borrow().message, status.borrow().edits);

    drop(sub_log);
    println!("after dropping log subscription: {}", buffer.events.len());
    buffer.edit("!");
    println!("log: {:?}", log.borrow());

    // Drop the status bar: subscriber notices via failed upgrade and removes itself.
    println!("status strong={} weak={}", Rc::strong_count(&status), Rc::weak_count(&status));
    drop(status);
    buffer.events.emit(&BufferEvent::Closed);
    println!("subscribers: {}", buffer.events.len());
    // Its Subscription handle still exists; dropping it is now a harmless no-op.
    drop(sub_status);
    println!("subscribers: {}", buffer.events.len());

    // Subscription outliving the emitter
    let late;
    {
        let set: SubscriberSet<u8> = SubscriberSet::new();
        late = set.subscribe(|_| true);
        println!("temp set len {}", set.len());
    }
    drop(late);
    println!("dropped subscription after emitter was gone: ok");
    println!("buffer text {:?}", buffer.text);
}
