// GPUI-style entities: typed Entity<T> handles (id + PhantomData), an EntityMap storing Box<dyn Any>,
// update via leasing, WeakEntity with ref counts, and Context<T> passed to update closures.
use std::any::Any;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct EntityId(u64);

struct Entity<T> {
    id: EntityId,
    counts: Rc<RefCell<BTreeMap<EntityId, usize>>>,
    _t: PhantomData<T>,
}

struct WeakEntity<T> {
    id: EntityId,
    counts: Rc<RefCell<BTreeMap<EntityId, usize>>>,
    _t: PhantomData<T>,
}

impl<T> Clone for Entity<T> {
    fn clone(&self) -> Self {
        *self.counts.borrow_mut().get_mut(&self.id).unwrap() += 1;
        Entity { id: self.id, counts: self.counts.clone(), _t: PhantomData }
    }
}

impl<T> Drop for Entity<T> {
    fn drop(&mut self) {
        let mut c = self.counts.borrow_mut();
        let n = c.get_mut(&self.id).unwrap();
        *n -= 1;
    }
}

impl<T: 'static> Entity<T> {
    fn downgrade(&self) -> WeakEntity<T> {
        WeakEntity { id: self.id, counts: self.counts.clone(), _t: PhantomData }
    }
    fn read<'a>(&self, app: &'a App) -> &'a T {
        app.entities.get(&self.id).unwrap().downcast_ref::<T>().unwrap()
    }
    fn update<R>(&self, app: &mut App, f: impl FnOnce(&mut T, &mut Context<T>) -> R) -> R {
        app.update_entity(self, f)
    }
}

impl<T: 'static> WeakEntity<T> {
    fn upgrade(&self) -> Option<Entity<T>> {
        let mut c = self.counts.borrow_mut();
        let n = c.get_mut(&self.id)?;
        if *n == 0 {
            return None;
        }
        *n += 1;
        Some(Entity { id: self.id, counts: self.counts.clone(), _t: PhantomData })
    }
}

struct App {
    next_id: u64,
    entities: BTreeMap<EntityId, Box<dyn Any>>,
    counts: Rc<RefCell<BTreeMap<EntityId, usize>>>,
    notified: Vec<EntityId>,
}

struct Context<'a, T> {
    app: &'a mut App,
    id: EntityId,
    _t: PhantomData<T>,
}

impl<'a, T: 'static> Context<'a, T> {
    fn notify(&mut self) {
        self.app.notified.push(self.id);
    }
    fn entity_id(&self) -> EntityId {
        self.id
    }
    fn new_entity<U: 'static>(&mut self, f: impl FnOnce(&mut Context<U>) -> U) -> Entity<U> {
        self.app.new_entity(f)
    }
}

impl App {
    fn new() -> Self {
        App { next_id: 1, entities: BTreeMap::new(), counts: Rc::new(RefCell::new(BTreeMap::new())), notified: vec![] }
    }

    fn new_entity<T: 'static>(&mut self, build: impl FnOnce(&mut Context<T>) -> T) -> Entity<T> {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        self.counts.borrow_mut().insert(id, 1);
        let mut cx = Context { app: self, id, _t: PhantomData };
        let value = build(&mut cx);
        self.entities.insert(id, Box::new(value));
        Entity { id, counts: self.counts.clone(), _t: PhantomData }
    }

    fn update_entity<T: 'static, R>(&mut self, e: &Entity<T>, f: impl FnOnce(&mut T, &mut Context<T>) -> R) -> R {
        let mut leased = self.entities.remove(&e.id).expect("entity already leased or released");
        let r = {
            let mut cx = Context { app: self, id: e.id, _t: PhantomData };
            f(leased.downcast_mut::<T>().unwrap(), &mut cx)
        };
        self.entities.insert(e.id, leased);
        r
    }

    fn gc(&mut self) -> Vec<EntityId> {
        let dead: Vec<EntityId> = self.counts.borrow().iter().filter(|(_, n)| **n == 0).map(|(id, _)| *id).collect();
        for id in &dead {
            self.entities.remove(id);
            self.counts.borrow_mut().remove(id);
        }
        dead
    }
}

struct Counter {
    count: i32,
    label: String,
}

struct Panel {
    title: String,
    counters: Vec<Entity<Counter>>,
}

fn main() {
    let mut app = App::new();
    let c1 = app.new_entity(|cx| Counter { count: 0, label: format!("c{}", cx.entity_id().0) });
    let panel = app.new_entity(|cx: &mut Context<Panel>| {
        let a = cx.new_entity(|cx| Counter { count: 10, label: format!("inner{}", cx.entity_id().0) });
        Panel { title: "Main".into(), counters: vec![a] }
    });

    c1.update(&mut app, |c, cx| {
        c.count += 5;
        cx.notify();
    });
    let shared = c1.clone();
    panel.update(&mut app, |p, _| p.counters.push(shared));
    println!("c1 = {} {}", c1.read(&app).label, c1.read(&app).count);

    let total = panel.update(&mut app, |p, cx| {
        let mut total = 0;
        for c in &p.counters {
            total += c.update(cx.app, |c, cx| {
                c.count *= 2;
                cx.notify();
                c.count
            });
        }
        p.title = format!("{} ({})", p.title, total);
        cx.notify();
        total
    });
    println!("total {} title {:?}", total, panel.read(&app).title);
    println!("notified {:?}", app.notified);
    let labels: Vec<String> =
        panel.read(&app).counters.iter().map(|c| format!("{}={}", c.read(&app).label, c.read(&app).count)).collect();
    println!("{:?}", labels);

    let weak = c1.downgrade();
    println!("counts {:?}", app.counts.borrow());
    drop(c1);
    println!("upgrade after dropping c1: {}", weak.upgrade().map(|e| e.read(&app).count).unwrap_or(-1));
    println!("gc {:?}", app.gc());
    let weak_panel = panel.downgrade();
    drop(panel);
    println!("gc {:?}", app.gc());
    println!("panel upgrade: {}", weak_panel.upgrade().is_some());
    println!("c1 upgrade (still held by panel?): {}", weak.upgrade().is_some());
    println!("entities left {:?}", app.entities.keys().collect::<Vec<_>>());
    println!("counts {:?}", app.counts.borrow());
}
