// An App with a deferred effect queue (like GPUI's flush_effects): views emit events and notify,
// observers and subscribers run after the update completes, `defer` closures, and re-entrancy safety.
use std::any::Any;
use std::collections::{BTreeMap, VecDeque};

type EntityId = usize;

enum Effect {
    Notify(EntityId),
    Emit(EntityId, Box<dyn Any>),
    Defer(Box<dyn FnOnce(&mut App)>),
}

type ObserverFn = Box<dyn FnMut(EntityId, &mut App)>;
type SubscriberFn = Box<dyn FnMut(&dyn Any, &mut App)>;

struct App {
    entities: BTreeMap<EntityId, Box<dyn Any>>,
    effects: VecDeque<Effect>,
    observers: BTreeMap<EntityId, Vec<ObserverFn>>,
    subscribers: BTreeMap<EntityId, Vec<SubscriberFn>>,
    flushing: bool,
    log: Vec<String>,
    next: EntityId,
}

impl App {
    fn new() -> Self {
        App { entities: BTreeMap::new(), effects: VecDeque::new(), observers: BTreeMap::new(), subscribers: BTreeMap::new(), flushing: false, log: vec![], next: 0 }
    }
    fn insert<T: 'static>(&mut self, v: T) -> EntityId {
        let id = self.next;
        self.next += 1;
        self.entities.insert(id, Box::new(v));
        id
    }
    fn read<T: 'static>(&self, id: EntityId) -> &T {
        self.entities[&id].downcast_ref().unwrap()
    }
    fn update<T: 'static, R>(&mut self, id: EntityId, f: impl FnOnce(&mut T, &mut App) -> R) -> R {
        let mut e = self.entities.remove(&id).unwrap();
        let r = f(e.downcast_mut().unwrap(), self);
        self.entities.insert(id, e);
        if !self.flushing {
            self.flush_effects();
        }
        r
    }
    fn notify(&mut self, id: EntityId) {
        self.effects.push_back(Effect::Notify(id));
    }
    fn emit<E: 'static>(&mut self, id: EntityId, e: E) {
        self.effects.push_back(Effect::Emit(id, Box::new(e)));
    }
    fn defer(&mut self, f: impl FnOnce(&mut App) + 'static) {
        self.effects.push_back(Effect::Defer(Box::new(f)));
    }
    fn observe(&mut self, id: EntityId, f: impl FnMut(EntityId, &mut App) + 'static) {
        self.observers.entry(id).or_default().push(Box::new(f));
    }
    fn subscribe<E: 'static>(&mut self, id: EntityId, mut f: impl FnMut(&E, &mut App) + 'static) {
        self.subscribers.entry(id).or_default().push(Box::new(move |e, app| {
            if let Some(e) = e.downcast_ref::<E>() {
                f(e, app)
            }
        }));
    }
    fn flush_effects(&mut self) {
        self.flushing = true;
        let mut processed = 0;
        while let Some(eff) = self.effects.pop_front() {
            processed += 1;
            match eff {
                Effect::Notify(id) => {
                    // dedupe further pending notifies for the same entity
                    self.effects.retain(|e| !matches!(e, Effect::Notify(x) if *x == id));
                    if let Some(mut obs) = self.observers.remove(&id) {
                        for o in obs.iter_mut() {
                            o(id, self);
                        }
                        self.observers.entry(id).or_default().splice(0..0, obs);
                    }
                }
                Effect::Emit(id, ev) => {
                    if let Some(mut subs) = self.subscribers.remove(&id) {
                        for s in subs.iter_mut() {
                            s(ev.as_ref(), self);
                        }
                        self.subscribers.entry(id).or_default().splice(0..0, subs);
                    }
                }
                Effect::Defer(f) => f(self),
            }
        }
        self.flushing = false;
        self.log.push(format!("flushed {} effects", processed));
    }
}

struct Counter {
    value: i32,
}
#[derive(Debug)]
enum CounterEvent {
    Changed(i32),
    HitTen,
}
struct Label {
    text: String,
    renders: u32,
}

fn main() {
    let mut app = App::new();
    let counter = app.insert(Counter { value: 0 });
    let label = app.insert(Label { text: String::new(), renders: 0 });
    let total = app.insert(0i64);

    // label re-renders whenever the counter notifies
    app.observe(counter, move |cid, app| {
        let v = app.read::<Counter>(cid).value;
        app.update::<Label, _>(label, |l, app| {
            l.renders += 1;
            l.text = format!("count = {}", v);
            app.log.push(format!("label rendered {:?}", l.text));
        });
    });
    app.subscribe::<CounterEvent>(counter, move |e, app| {
        app.log.push(format!("event {:?}", e));
        if let CounterEvent::Changed(d) = e {
            app.update::<i64, _>(total, |t, _| *t += *d as i64);
        }
        if let CounterEvent::HitTen = e {
            app.defer(move |app| {
                app.log.push("deferred: resetting counter".into());
                app.update::<Counter, _>(counter, |c, app| {
                    c.value = 0;
                    app.notify(counter);
                });
            });
        }
    });

    for delta in [3, 4, 5, -1] {
        app.update::<Counter, _>(counter, |c, app| {
            let before = c.value;
            c.value += delta;
            app.emit(counter, CounterEvent::Changed(delta));
            app.notify(counter);
            app.notify(counter); // duplicate, deduped
            if before < 10 && c.value >= 10 {
                app.emit(counter, CounterEvent::HitTen);
            }
        });
    }
    for l in &app.log {
        println!("{}", l);
    }
    let lbl = app.read::<Label>(label);
    println!("label={:?} renders={} counter={} total={}", lbl.text, lbl.renders, app.read::<Counter>(counter).value, app.read::<i64>(total));
}
