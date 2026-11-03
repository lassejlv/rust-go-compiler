use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

trait Observer { fn notify(&self, topic: &str, value: i64) -> bool; fn name(&self) -> String; }

struct Subject { observers: RefCell<Vec<Weak<dyn Observer>>>, history: RefCell<Vec<(String, i64)>> }

impl Subject {
    fn new() -> Rc<Subject> { Rc::new(Subject { observers: RefCell::new(vec![]), history: RefCell::new(vec![]) }) }
    fn subscribe(&self, o: &Rc<dyn Observer>) { self.observers.borrow_mut().push(Rc::downgrade(o)); }
    fn publish(&self, topic: &str, value: i64) -> usize {
        self.history.borrow_mut().push((topic.to_string(), value));
        let live: Vec<Rc<dyn Observer>> = self.observers.borrow().iter().filter_map(Weak::upgrade).collect();
        let mut keep = vec![];
        let mut handled = 0;
        for o in &live { if o.notify(topic, value) { handled += 1; keep.push(Rc::downgrade(o)); } else { println!("  {} unsubscribed", o.name()); } }
        *self.observers.borrow_mut() = keep;
        handled
    }
}

struct Logger { prefix: String, lines: RefCell<Vec<String>> }
impl Observer for Logger {
    fn notify(&self, topic: &str, value: i64) -> bool { self.lines.borrow_mut().push(format!("{}{}={}", self.prefix, topic, value)); true }
    fn name(&self) -> String { format!("logger({})", self.prefix) }
}
struct Threshold { limit: i64, hits: Cell<u32>, max_hits: u32 }
impl Observer for Threshold {
    fn notify(&self, _t: &str, v: i64) -> bool { if v > self.limit { self.hits.set(self.hits.get() + 1); } self.hits.get() < self.max_hits }
    fn name(&self) -> String { format!("threshold>{}", self.limit) }
}
struct Relay { target: Weak<Subject>, factor: i64, depth: Cell<u8> }
impl Observer for Relay {
    fn notify(&self, topic: &str, v: i64) -> bool {
        if topic.starts_with("relayed") || self.depth.get() > 0 { return true; }
        if let Some(t) = self.target.upgrade() { self.depth.set(1); t.publish(&format!("relayed.{}", topic), v * self.factor); self.depth.set(0); true } else { false }
    }
    fn name(&self) -> String { "relay".into() }
}
impl Drop for Logger { fn drop(&mut self) { println!("  dropping {} with {} lines", self.name(), self.lines.borrow().len()); } }

fn main() {
    let hub = Subject::new();
    let sink = Subject::new();
    let log_a = Rc::new(Logger { prefix: "A:".into(), lines: RefCell::new(vec![]) });
    let log_a_dyn: Rc<dyn Observer> = log_a.clone();
    let thr: Rc<dyn Observer> = Rc::new(Threshold { limit: 10, hits: Cell::new(0), max_hits: 2 });
    let relay: Rc<dyn Observer> = Rc::new(Relay { target: Rc::downgrade(&sink), factor: 100, depth: Cell::new(0) });
    let sink_log = Rc::new(Logger { prefix: "S:".into(), lines: RefCell::new(vec![]) });
    let sink_log_dyn: Rc<dyn Observer> = sink_log.clone();
    hub.subscribe(&log_a_dyn); hub.subscribe(&thr); hub.subscribe(&relay);
    sink.subscribe(&sink_log_dyn); sink.subscribe(&relay);
    {
        let temp: Rc<dyn Observer> = Rc::new(Logger { prefix: "T:".into(), lines: RefCell::new(vec![]) });
        hub.subscribe(&temp);
        println!("published to {}", hub.publish("x", 5));
    }
    for (t, v) in [("y", 20), ("z", 30), ("w", 1)] { println!("{}={} handled by {}", t, v, hub.publish(t, v)); }
    drop(log_a_dyn);
    println!("strong(log_a)={} weak={}", Rc::strong_count(&log_a), Rc::weak_count(&log_a));
    drop(sink);
    println!("after sink dropped: {}", hub.publish("q", 0));
    println!("A lines {:?}", log_a.lines.borrow());
    println!("S lines {:?}", sink_log.lines.borrow());
    println!("hub history {:?}", hub.history.borrow());
    drop(log_a);
    println!("observers left {}", hub.observers.borrow().iter().filter(|w| w.upgrade().is_some()).count());
    println!("end");
}
